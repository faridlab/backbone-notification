//! The hand-authored notification write path (user-owned; survives regen).
//!
//! Outbound templated fan-out: a domain event fires, the matching active template renders one message per
//! recipient, and each is dispatched through backbone-communication. **Idempotent per (event_id,
//! recipient_address)** — a redelivered domain event does not re-create nor re-dispatch a recipient's
//! notification (the inbox pattern realized on the notification row's natural idempotency key). Posts NO
//! GL. The Indonesia statutory/business content is the template author's concern, not this engine's.

use backbone_orm::company_scope;
use serde::Deserialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::notification_events::*;
use super::notification_ports::*;

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("db: {0}")]
    Db(#[from] sqlx::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
}

pub struct NewTemplate {
    pub company_id: Uuid,
    pub event_type: String,
    pub channel: String, // whatsapp | email | sms
    pub name: String,
    pub subject_template: Option<String>,
    pub body_template: String,
}

pub struct Recipient {
    pub party_id: Option<Uuid>,
    pub address: String,
}

/// A domain event to fan out. `data` supplies the `{{placeholder}}` values the template renders.
pub struct NotifyEvent {
    pub company_id: Uuid,
    pub event_id: Uuid,
    pub event_type: String,
    pub channel: String,
    pub recipients: Vec<Recipient>,
    pub data: serde_json::Value,
}

/// The provider's real delivery outcome for a dispatched message (from communication's receipts).
#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryOutcome {
    Delivered,
    Undelivered(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NotifyOutcome {
    pub dispatched: usize,
    pub deduped: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Deserialize)]
struct TemplateRow {
    id: Uuid,
    subject_template: Option<String>,
    body_template: String,
}

pub struct NotificationWriteService {
    pool: PgPool,
}

impl NotificationWriteService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Define (or replace) the active template for a (company, event_type, channel).
    pub async fn create_template(&self, t: NewTemplate) -> Result<Uuid, NotifyError> {
        if t.body_template.trim().is_empty() {
            return Err(NotifyError::Invalid("template needs a body".into()));
        }
        let id = Uuid::new_v4();
        let ins_q = sqlx::query(
            r#"INSERT INTO notification.notification_templates
                 (id, company_id, event_type, channel, name, subject_template, body_template, is_active)
               VALUES ($1,$2,$3,$4::notif_channel,$5,$6,$7,true)"#,
        )
        .bind(id).bind(t.company_id).bind(&t.event_type).bind(&t.channel).bind(&t.name)
        .bind(&t.subject_template).bind(&t.body_template);
        let r = company_scope::with_company_scope(
            Some(t.company_id),
            company_scope::execute_scoped(&self.pool, ins_q),
        ).await;
        match r {
            Ok(_) => Ok(id),
            Err(e) if e.as_database_error().map(|d| d.is_unique_violation()).unwrap_or(false) =>
                Err(NotifyError::Invalid("a template already exists for this event/channel".into())),
            Err(e) => Err(e.into()),
        }
    }

    /// Fan a domain event out to its recipients. For each recipient: render the active template and
    /// dispatch through the `CommunicationPort`, recording exactly one notification per (event_id,
    /// recipient_address). A redelivered event dedups on that key (no double-notify); a recipient with no
    /// active template is skipped.
    pub async fn notify(
        &self,
        ev: NotifyEvent,
        port: &dyn CommunicationPort,
        events: &dyn NotificationEventSink,
    ) -> Result<NotifyOutcome, NotifyError> {
        let mut outcome = NotifyOutcome::default();

        // Resolve the active template for (company, event_type, channel).
        let template_q = sqlx::query_as::<_, (Uuid, Option<String>, String)>(
            r#"SELECT id, subject_template, body_template FROM notification.notification_templates
               WHERE company_id=$1 AND event_type=$2 AND channel=$3::notif_channel AND is_active=true
                 AND (metadata->>'deleted_at') IS NULL
               LIMIT 1"#,
        )
        .bind(ev.company_id).bind(&ev.event_type).bind(&ev.channel);
        let template: Option<TemplateRow> = company_scope::with_company_scope(
            Some(ev.company_id),
            company_scope::fetch_optional_scoped(&self.pool, template_q),
        ).await?
        .map(|(id, subject_template, body_template)| TemplateRow { id, subject_template, body_template });

        let Some(template) = template else {
            // No template for this event/channel — nothing to send. Skipped (recorded in the outcome).
            outcome.skipped = ev.recipients.len();
            return Ok(outcome);
        };

        for r in &ev.recipients {
            if r.address.trim().is_empty() {
                return Err(NotifyError::Invalid("recipient needs an address".into()));
            }
            let subject = template.subject_template.as_ref().map(|s| render(s, &ev.data));
            let body = render(&template.body_template, &ev.data);

            // Claim the (event_id, recipient) dedup slot. A redelivered event conflicts here → deduped.
            let claim_q = sqlx::query_scalar(
                r#"INSERT INTO notification.notifications
                     (id, company_id, event_id, event_type, template_id, channel, recipient_party_id,
                      recipient_address, subject, body, status)
                   VALUES ($1,$2,$3,$4,$5,$6::notif_channel,$7,$8,$9,$10,'pending'::notification_status)
                   ON CONFLICT (event_id, recipient_address) DO NOTHING
                   RETURNING id"#,
            )
            .bind(Uuid::new_v4()).bind(ev.company_id).bind(ev.event_id).bind(&ev.event_type)
            .bind(template.id).bind(&ev.channel).bind(r.party_id).bind(&r.address).bind(&subject).bind(&body);
            let inserted: Option<Uuid> = company_scope::with_company_scope(
                Some(ev.company_id),
                company_scope::fetch_optional_scalar_scoped(&self.pool, claim_q),
            ).await?;

            let Some(notification_id) = inserted else {
                outcome.deduped += 1;
                continue;
            };

            // Dispatch through the channel gateway.
            let req = DispatchRequest {
                idempotency_key: notification_id.to_string(),
                company_id: ev.company_id, channel: ev.channel.clone(),
                recipient_party_id: r.party_id, recipient_address: r.address.clone(),
                subject: subject.clone(), body: body.clone(),
            };
            match port.dispatch(&req).await {
                Ok(ack) => {
                    company_scope::with_company_scope(
                        Some(ev.company_id), self.mark_sent(notification_id, ack.message_id)).await?;
                    outcome.dispatched += 1;
                    events.publish(&NotificationEvent::NotificationDispatched {
                        notification_id, event_id: ev.event_id, message_id: ack.message_id,
                    });
                }
                Err(rej) => {
                    company_scope::with_company_scope(
                        Some(ev.company_id), self.mark_failed(notification_id, &rej.message)).await?;
                    outcome.failed += 1;
                    events.publish(&NotificationEvent::NotificationFailed {
                        notification_id, event_id: ev.event_id, reason: rej.code.clone(),
                    });
                }
            }
        }
        Ok(outcome)
    }

    /// Re-drive notifications stranded in `pending` — the reaper the dedup slot demands. A crash between
    /// the committed `pending` INSERT and the dispatch leaves a row `pending` that the `(event_id,
    /// recipient)` dedup then protects from ever being re-sent by a redelivery; without this sweep the
    /// recipient is silently NEVER notified (maturity council 2026-07-08). Recovery is keyed on STATE, not
    /// existence. Safe against double-notify because the dispatch carries the notification's idempotency
    /// key. Run on a schedule. Returns the number newly dispatched.
    pub async fn dispatch_pending(
        &self,
        limit: i64,
        port: &dyn CommunicationPort,
        events: &dyn NotificationEventSink,
    ) -> Result<usize, NotifyError> {
        // The sweep carries no company of its own — it reads under the AMBIENT scope, so the CALLER (the
        // scheduler) MUST wrap this call in `with_company_scope(Some(company))` and drive it once per
        // company; otherwise the RLS fence returns nothing and the stranded rows are never re-driven.
        let rows_q = sqlx::query(
            r#"SELECT id, event_id, company_id, channel::text AS channel, recipient_party_id,
                      recipient_address, subject, body
               FROM notification.notifications
               WHERE status='pending'::notification_status AND (metadata->>'deleted_at') IS NULL
               ORDER BY (metadata->>'created_at') NULLS FIRST LIMIT $1"#,
        )
        .bind(limit);
        let rows = company_scope::fetch_all_rows_scoped(&self.pool, rows_q).await?;

        let mut dispatched = 0usize;
        for row in &rows {
            let notification_id: Uuid = row.get("id");
            let event_id: Uuid = row.get("event_id");
            let row_company: Uuid = row.get("company_id");
            let req = DispatchRequest {
                idempotency_key: notification_id.to_string(),
                company_id: row_company, channel: row.get("channel"),
                recipient_party_id: row.get("recipient_party_id"),
                recipient_address: row.get("recipient_address"),
                subject: row.get("subject"), body: row.get("body"),
            };
            match port.dispatch(&req).await {
                Ok(ack) => {
                    company_scope::with_company_scope(
                        Some(row_company), self.mark_sent(notification_id, ack.message_id)).await?;
                    dispatched += 1;
                    events.publish(&NotificationEvent::NotificationDispatched {
                        notification_id, event_id, message_id: ack.message_id,
                    });
                }
                Err(rej) => {
                    company_scope::with_company_scope(
                        Some(row_company), self.mark_failed(notification_id, &rej.message)).await?;
                    events.publish(&NotificationEvent::NotificationFailed {
                        notification_id, event_id, reason: rej.code.clone(),
                    });
                }
            }
        }
        Ok(dispatched)
    }

    /// Close the loop on a `sent` notification with the provider's real delivery outcome — the verb a
    /// composing service calls when backbone-communication emits `MessageDelivered`/`MessageFailed`.
    /// Without it a `sent` notification is indistinguishable from a bounced one, and a delivery-driven
    /// escalation (undelivered invoice reminder → retry another channel) is unimplementable (completeness
    /// council 2026-07-08). Correlates by `message_id`; state-guarded on `sent` (idempotent — a redelivered
    /// receipt is a no-op). Emits `NotificationDelivered`/`NotificationUndelivered`.
    pub async fn record_delivery(
        &self,
        message_id: Uuid,
        outcome: DeliveryOutcome,
        events: &dyn NotificationEventSink,
    ) -> Result<bool, NotifyError> {
        let (status, reason) = match &outcome {
            DeliveryOutcome::Delivered => ("delivered", None),
            DeliveryOutcome::Undelivered(reason) => ("undelivered", Some(reason.clone())),
        };
        let mut tx = self.pool.begin().await?;
        // Correlated by `message_id` alone — this verb has NO company of its own, so it binds the AMBIENT
        // scope. The CALLER (the receipt consumer) MUST wrap this in
        // `with_company_scope(Some(event.company_id))` from the communication receipt it is reacting to.
        company_scope::bind_current_company(&mut tx).await?;
        let row = sqlx::query(
            r#"UPDATE notification.notifications
               SET status=$2::notification_status, failure_reason=COALESCE($3, failure_reason)
               WHERE message_id=$1 AND status='sent'::notification_status
               RETURNING id, event_id"#,
        )
        .bind(message_id).bind(status).bind(&reason)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else { tx.rollback().await?; return Ok(false) };
        let notification_id: Uuid = row.get("id");
        let event_id: Uuid = row.get("event_id");
        let event = match outcome {
            DeliveryOutcome::Delivered =>
                NotificationEvent::NotificationDelivered { notification_id, event_id },
            DeliveryOutcome::Undelivered(reason) =>
                NotificationEvent::NotificationUndelivered { notification_id, event_id, reason },
        };
        // Stage the delivery-state event durably in the same tx as the status transition (outbox rollout
        // plan, P2): a consumer escalates on it, so a crash before the in-proc publish must not drop it.
        let record = backbone_outbox::OutboxRecord::new(
            match &event { NotificationEvent::NotificationUndelivered { .. } => "NotificationUndelivered", _ => "NotificationDelivered" },
            "Notification", notification_id.to_string(),
            serde_json::to_value(&event).map_err(|e| NotifyError::Invalid(e.to_string()))?,
            chrono::Utc::now(),
        );
        backbone_outbox::outbox::stage(&mut *tx, "notification", &record)
            .await.map_err(|e| NotifyError::Invalid(format!("outbox stage: {e}")))?;
        tx.commit().await?;
        events.publish(&event);
        Ok(true)
    }

    /// ID-only — rides the caller's company scope (every call site wraps in `with_company_scope`).
    async fn mark_sent(&self, notification_id: Uuid, message_id: Uuid) -> Result<(), NotifyError> {
        let upd_q = sqlx::query(
            r#"UPDATE notification.notifications SET status='sent'::notification_status, message_id=$2
               WHERE id=$1 AND status='pending'::notification_status"#,
        )
        .bind(notification_id).bind(message_id);
        company_scope::execute_scoped(&self.pool, upd_q).await?;
        Ok(())
    }

    /// ID-only — rides the caller's company scope (every call site wraps in `with_company_scope`).
    async fn mark_failed(&self, notification_id: Uuid, reason: &str) -> Result<(), NotifyError> {
        let upd_q = sqlx::query(
            r#"UPDATE notification.notifications SET status='failed'::notification_status, failure_reason=$2
               WHERE id=$1 AND status='pending'::notification_status"#,
        )
        .bind(notification_id).bind(reason);
        company_scope::execute_scoped(&self.pool, upd_q).await?;
        Ok(())
    }
}

/// Minimal `{{key}}` substitution from a JSON object. An unknown key renders empty.
fn render(template: &str, data: &serde_json::Value) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        if let Some(end) = rest.find("}}") {
            let key = rest[..end].trim();
            let val = data.get(key).map(json_scalar).unwrap_or_default();
            out.push_str(&val);
            rest = &rest[end + 2..];
        } else {
            out.push_str("{{");
            break;
        }
    }
    out.push_str(rest);
    out
}

fn json_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
