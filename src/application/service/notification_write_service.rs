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

use crate::domain::entity::{Notification, NotificationTemplate};
use crate::infrastructure::persistence::{
    NewNotificationRow, NewTemplateRow, NotificationRepository, NotificationTemplateRepository,
};

use super::notification_events::*;
use super::notification_ports::*;

/// Assert both entities expose a `company_field()` so the multi-tenant scope fence is wired.
/// Called from [`NotificationWriteService::new`] — a `None` would make `backbone_orm::company_scope`
/// degrade silently into unscoped queries (cross-tenant leak); refuse to build instead.
fn assert_tenant_fence_wired() {
    use backbone_orm::EntityRepoMeta;
    assert!(
        Notification::company_field().is_some(),
        "Notification::company_field() is None — tenant isolation unwired; refusing to build \
         NotificationWriteService (a scoped write would run without a company fence → cross-tenant leak)"
    );
    assert!(
        NotificationTemplate::company_field().is_some(),
        "NotificationTemplate::company_field() is None — tenant isolation unwired; refusing to \
         build NotificationWriteService"
    );
}

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
        // Fail loud at wiring time: see `assert_tenant_fence_wired`. If a schema/codegen change
        // ever drops the entities' `company_field()`, the scope helpers would fence on nothing.
        assert_tenant_fence_wired();

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
            // Canonicalize the address BEFORE the dedup claim, so two spellings of the same
            // recipient (`+62 812-345` vs `+62812345`, `User@Foo.com` vs `user@foo.com`) collapse
            // onto one idempotency key instead of double-notifying. The normalized form is what we
            // store AND dispatch. See `normalize_recipient_address`.
            let address = normalize_recipient_address(&ev.channel, &r.address);
            if address.is_empty() {
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
                    recipient_address: &address,
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
                recipient_party_id: r.party_id, recipient_address: address.clone(),
                subject: subject.clone(), body: body.clone(),
            };
            match port.dispatch(&req).await {
                Ok(ack) => {
                    self.commit_dispatched(ev.company_id, notification_id, ev.event_id, ack.message_id, events).await?;
                    outcome.dispatched += 1;
                }
                Err(rej) => {
                    self.commit_failed(ev.company_id, notification_id, ev.event_id, &rej, events).await?;
                    outcome.failed += 1;
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
                    self.commit_dispatched(row_company, notification_id, event_id, ack.message_id, events).await?;
                    dispatched += 1;
                }
                Err(rej) => {
                    self.commit_failed(row_company, notification_id, event_id, &rej, events).await?;
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
        // Correlated by `message_id` alone — this verb has NO company of its own, so the UPDATE binds
        // the AMBIENT scope. The CALLER (the receipt consumer) MUST wrap this in
        // `with_company_scope(Some(event.company_id))` from the communication receipt it is reacting to.
        company_scope::bind_current_company(&mut tx).await?;
        let row = self.notifications
            .apply_delivery_receipt(&mut tx, message_id, status, reason.as_ref())
            .await?;
        let Some(row) = row else { tx.rollback().await?; return Ok(false) };
        let notification_id = row.id;
        let event_id = row.event_id;
        // The notification's own tenant — read off the row (the authoritative source), never off the
        // event payload (a NotificationEvent carries no company_id; the prior extraction always failed).
        let company_id = row.company_id;
        let event = match outcome {
            DeliveryOutcome::Delivered =>
                NotificationEvent::NotificationDelivered { notification_id, event_id },
            DeliveryOutcome::Undelivered(reason) =>
                NotificationEvent::NotificationUndelivered { notification_id, event_id, reason },
        };
        // Stage the delivery-state event durably in the same tx as the status transition: a consumer
        // escalates on it, so a crash before the in-proc publish must not drop it.
        stage_lifecycle_event(&mut tx, company_id, &event).await?;
        tx.commit().await?;
        events.publish(&event);
        Ok(true)
    }

    /// `pending → sent` and stage `NotificationDispatched` to the outbox in ONE tx, then publish
    /// in-process. The transition and the event land atomically, so a crash between them cannot drop
    /// the dispatch signal a downstream consumer escalates on (the durability gap the maturity council
    /// flagged for `notify`/`dispatch_pending`). The company is bound EXPLICITLY off the known tenant —
    /// no ambient-scope dependency, correct for the event-subscriber/job callers that drive this engine.
    async fn commit_dispatched(
        &self,
        company_id: Uuid,
        notification_id: Uuid,
        event_id: Uuid,
        message_id: Uuid,
        events: &dyn NotificationEventSink,
    ) -> Result<(), NotifyError> {
        let event = NotificationEvent::NotificationDispatched { notification_id, event_id, message_id };
        let mut tx = self.pool.begin().await?;
        company_scope::bind_company_on(&mut tx, company_id).await?;
        self.notifications.mark_sent_in_tx(&mut tx, notification_id, message_id).await?;
        stage_lifecycle_event(&mut tx, company_id, &event).await?;
        tx.commit().await?;
        events.publish(&event);
        Ok(())
    }

    /// `pending → failed` and stage `NotificationFailed` to the outbox in ONE tx, then publish
    /// in-process. The stored `failure_reason` is the human `message`; the event's `reason` is the
    /// stable `code` (preserving the prior split). See [`Self::commit_dispatched`] for the tx rationale.
    async fn commit_failed(
        &self,
        company_id: Uuid,
        notification_id: Uuid,
        event_id: Uuid,
        rej: &DispatchRejected,
        events: &dyn NotificationEventSink,
    ) -> Result<(), NotifyError> {
        let event = NotificationEvent::NotificationFailed {
            notification_id, event_id, reason: rej.code.clone(),
        };
        let mut tx = self.pool.begin().await?;
        company_scope::bind_company_on(&mut tx, company_id).await?;
        self.notifications.mark_failed_in_tx(&mut tx, notification_id, &rej.message).await?;
        stage_lifecycle_event(&mut tx, company_id, &event).await?;
        tx.commit().await?;
        events.publish(&event);
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

/// Canonicalize a recipient address to its dedup key, by channel.
///
/// The idempotency unique index sits on `(event_id, recipient_address)`, so without a canonical form
/// the same human typed two ways (`+62 812-345` vs `+62812-345`, `User@Foo.com` vs `user@foo.com`)
/// records TWO rows and double-sends. This collapses those spellings:
///
/// - **email**: trim + ASCII-lowercase. `@` and `.` are structural to an address's identity and are
///   never stripped (`a@b.com` must not collide with `ab.com`).
/// - **sms / whatsapp / any phone channel**: keep only an optional leading `+` and the digits,
///   dropping spaces, dashes, parens, etc. → `+62 (812) 345-6789` becomes `+628123456789`.
///
/// Phone normalization is INTENTIONALLY limited to format stripping. It does NOT insert or guess a
/// country code: turning a national number (`0812 345`) into international form needs a locale the
/// engine does not own, and the deterministic in-module norm exists precisely to avoid that
/// ambiguity (and a new dependency). Callers MUST supply the international form (`+<country><number>`)
/// as the contract; two numbers differing only in national-vs-international form will NOT collapse,
/// and that is the accepted trade. Returns empty for all-punctuation/no-digit input (rejected upstream).
fn normalize_recipient_address(channel: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    if channel.eq_ignore_ascii_case("email") {
        return trimmed.to_ascii_lowercase();
    }
    // Phone-like channel: optional leading '+' then digits only.
    let mut out = String::with_capacity(trimmed.len());
    let mut rest = trimmed;
    if let Some(after_plus) = rest.strip_prefix('+') {
        out.push('+');
        rest = after_plus;
    }
    out.extend(rest.chars().filter(|c| c.is_ascii_digit()));
    // Lone `+` (plus pushed, no digits) or all-punctuation input is not a real address.
    if out.len() <= 1 {
        return String::new();
    }
    out
}

/// Stage a notification lifecycle event to the outbox on the caller's open transaction, in-tx with
/// the status transition that produced it — so a crash between the write and the in-process publish
/// cannot drop the event.
///
/// `company_id` is the notification's own tenant, passed EXPLICITLY. A `NotificationEvent` is a terminal
/// observability signal and carries NO tenant in its payload, so the company must never be read back out
/// of the serialized event (`record_delivery` once did that and the staging always failed).
async fn stage_lifecycle_event(
    conn: &mut sqlx::PgConnection,
    company_id: Uuid,
    event: &NotificationEvent,
) -> Result<(), NotifyError> {
    let payload = serde_json::to_value(event)
        .map_err(|e| NotifyError::Invalid(format!("event serialize: {e}")))?;
    let (event_type, notification_id) = match event {
        NotificationEvent::NotificationDispatched { notification_id, .. } =>
            ("NotificationDispatched", *notification_id),
        NotificationEvent::NotificationFailed { notification_id, .. } =>
            ("NotificationFailed", *notification_id),
        NotificationEvent::NotificationDelivered { notification_id, .. } =>
            ("NotificationDelivered", *notification_id),
        NotificationEvent::NotificationUndelivered { notification_id, .. } =>
            ("NotificationUndelivered", *notification_id),
    };
    let record = backbone_outbox::OutboxRecord::new(
        event_type,
        "Notification",
        notification_id.to_string(),
        company_id,
        payload,
        chrono::Utc::now(),
    );
    backbone_outbox::outbox::stage(conn, "notification", &record)
        .await
        .map_err(|e| NotifyError::Invalid(format!("outbox stage: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod normalize_tests {
    use super::normalize_recipient_address;

    #[test]
    fn phone_strips_formatting_to_canonical() {
        // The leak the council flagged: two spellings of one recipient must collapse.
        assert_eq!(normalize_recipient_address("whatsapp", "+62 812-345"), "+62812345");
        assert_eq!(normalize_recipient_address("sms", "+62 (812) 345-6789"), "+628123456789");
        assert_eq!(normalize_recipient_address("whatsapp", " +62 812-345 "), "+62812345");
        assert_eq!(normalize_recipient_address("sms", "+62812345"), "+62812345"); // idempotent on clean
    }

    #[test]
    fn phone_two_spellings_collide() {
        let a = normalize_recipient_address("whatsapp", "+62 812-345");
        let b = normalize_recipient_address("whatsapp", "+62812345");
        assert_eq!(a, b, "the whole point: same key, no double-notify");
    }

    #[test]
    fn email_lowercases_and_keeps_structure() {
        assert_eq!(normalize_recipient_address("email", "User@Foo.COM"), "user@foo.com");
        assert_eq!(normalize_recipient_address("email", " user@foo.com "), "user@foo.com");
        // `@` and `.` are structural — must NOT be stripped into a collision.
        assert_ne!(normalize_recipient_address("email", "a@b.com"), normalize_recipient_address("email", "ab.com"));
    }

    #[test]
    fn national_form_does_not_collapse_to_international() {
        // Documented contract: we strip format only; we do not guess a country code.
        assert_ne!(normalize_recipient_address("sms", "0812 345"), normalize_recipient_address("sms", "+62812345"));
    }

    #[test]
    fn garbage_is_rejected_as_empty() {
        assert_eq!(normalize_recipient_address("sms", "  "), "");
        assert_eq!(normalize_recipient_address("whatsapp", "+"), "");
        assert_eq!(normalize_recipient_address("sms", "+ - ()"), "");
        assert_eq!(normalize_recipient_address("email", "   "), "");
    }
}
