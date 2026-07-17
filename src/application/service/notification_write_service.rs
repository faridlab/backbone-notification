//! The hand-authored notification write path (user-owned; survives regen).
//!
//! Outbound templated fan-out: a domain event fires, the matching active template renders one message per
//! recipient, and each is dispatched through backbone-communication. **Idempotent per (event_id,
//! recipient_address)** — a redelivered domain event does not re-create nor re-dispatch a recipient's
//! notification (the inbox pattern realized on the notification row's natural idempotency key). Posts NO
//! GL. The Indonesia statutory/business content is the template author's concern, not this engine's.

use backbone_orm::company_scope;
use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::persistence::{
    NewNotificationRow, NewTemplateRow, NotificationRepository, NotificationTemplateRepository,
};

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

pub struct NotificationWriteService {
    pool: PgPool,
    templates: NotificationTemplateRepository,
    notifications: NotificationRepository,
}

impl NotificationWriteService {
    pub fn new(pool: PgPool) -> Self {
        let templates = NotificationTemplateRepository::new(pool.clone());
        let notifications = NotificationRepository::new(pool.clone());
        Self { pool, templates, notifications }
    }

    /// Define (or replace) the active template for a (company, event_type, channel).
    pub async fn create_template(&self, t: NewTemplate) -> Result<Uuid, NotifyError> {
        if t.body_template.trim().is_empty() {
            return Err(NotifyError::Invalid("template needs a body".into()));
        }
        let id = Uuid::new_v4();
        let r = company_scope::with_company_scope(
            Some(t.company_id),
            self.templates.insert_template(&self.pool, &NewTemplateRow {
                id,
                company_id: t.company_id,
                event_type: &t.event_type,
                channel: &t.channel,
                name: &t.name,
                subject_template: t.subject_template.as_ref(),
                body_template: &t.body_template,
            }),
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
        let template = company_scope::with_company_scope(
            Some(ev.company_id),
            self.templates.find_active(&self.pool, ev.company_id, &ev.event_type, &ev.channel),
        ).await?;

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
            let inserted = company_scope::with_company_scope(
                Some(ev.company_id),
                self.notifications.claim_recipient(&self.pool, &NewNotificationRow {
                    id: Uuid::new_v4(),
                    company_id: ev.company_id,
                    event_id: ev.event_id,
                    event_type: &ev.event_type,
                    template_id: template.id,
                    channel: &ev.channel,
                    recipient_party_id: r.party_id,
                    recipient_address: &r.address,
                    subject: subject.as_ref(),
                    body: &body,
                }),
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
                        Some(ev.company_id), self.notifications.mark_sent(&self.pool, notification_id, ack.message_id)).await?;
                    outcome.dispatched += 1;
                    events.publish(&NotificationEvent::NotificationDispatched {
                        notification_id, event_id: ev.event_id, message_id: ack.message_id,
                    });
                }
                Err(rej) => {
                    company_scope::with_company_scope(
                        Some(ev.company_id), self.notifications.mark_failed(&self.pool, notification_id, &rej.message)).await?;
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
        let rows = self.notifications.list_pending(&self.pool, limit).await?;

        let mut dispatched = 0usize;
        for row in &rows {
            let notification_id = row.id;
            let event_id = row.event_id;
            let row_company = row.company_id;
            let req = DispatchRequest {
                idempotency_key: notification_id.to_string(),
                company_id: row_company, channel: row.channel.clone(),
                recipient_party_id: row.recipient_party_id,
                recipient_address: row.recipient_address.clone(),
                subject: row.subject.clone(), body: row.body.clone(),
            };
            match port.dispatch(&req).await {
                Ok(ack) => {
                    company_scope::with_company_scope(
                        Some(row_company), self.notifications.mark_sent(&self.pool, notification_id, ack.message_id)).await?;
                    dispatched += 1;
                    events.publish(&NotificationEvent::NotificationDispatched {
                        notification_id, event_id, message_id: ack.message_id,
                    });
                }
                Err(rej) => {
                    company_scope::with_company_scope(
                        Some(row_company), self.notifications.mark_failed(&self.pool, notification_id, &rej.message)).await?;
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
        let row = self.notifications
            .apply_delivery_receipt(&mut tx, message_id, status, reason.as_ref())
            .await?;
        let Some(row) = row else { tx.rollback().await?; return Ok(false) };
        let notification_id = row.id;
        let event_id = row.event_id;
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
