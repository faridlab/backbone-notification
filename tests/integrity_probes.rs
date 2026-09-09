//! Integrity probes — the fan-out engine's invariants: a recipient needs an address, a dispatch
//! rejection is recorded, and dedup is per-recipient (a new recipient on the same event still
//! sends). The one-active-template-per-(event_type, channel) uniqueness is tenancy posture: no
//! module table carries it (the composing service's tenancy decorator installs the per-unit
//! form), so the probe that pinned the company-leading unique retired with the strip.

mod common;
use common::*;

use backbone_notification::application::service::notification_events::LoggingSink;
use backbone_notification::application::service::notification_write_service::*;
use serde_json::json;
use uuid::Uuid;

async fn with_template(pool: &sqlx::PgPool, company: Uuid) -> NotificationWriteService {
    let svc = NotificationWriteService::new(pool.clone());
    with_org_scope(pool, company, async {
        svc.create_template(NewTemplate {
            event_type: "InvoiceDue".into(), channel: "whatsapp".into(),
            name: "Invoice due".into(), subject_template: None, body_template: "Tagihan jatuh tempo".into(),
        }).await.unwrap();
    }).await;
    svc
}

fn ev(recipients: Vec<Recipient>) -> NotifyEvent {
    NotifyEvent {
        event_id: Uuid::new_v4(), event_type: "InvoiceDue".into(),
        channel: "whatsapp".into(), recipients, data: json!({}),
    }
}

// NIP-1 — a recipient with a blank address is refused.
#[tokio::test]
async fn nip1_recipient_needs_address() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = with_template(&pool, company).await;
    let r = svc.notify(ev(vec![Recipient { party_id: None, address: "  ".into() }]),
        &FakeComm::new(), &LoggingSink).await;
    assert!(matches!(r, Err(NotifyError::Invalid(_))));
}

// NIP-3 — a dispatch rejection records a failed notification (not swallowed).
#[tokio::test]
async fn nip3_dispatch_rejection_recorded() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = with_template(&pool, company).await;
    let port = FakeComm::rejecting("invalid_number", "bad msisdn");

    let event = ev(vec![Recipient { party_id: None, address: "+628111".into() }]);
    let event_id = event.event_id;
    let out = with_org_scope(&pool, company, async {
        svc.notify(event, &port, &LoggingSink).await.unwrap()
    }).await;
    assert_eq!(out.failed, 1);

    let (status, reason): (String, Option<String>) = sqlx::query_as(
        "SELECT status::text, failure_reason FROM notification.notifications WHERE event_id=$1")
        .bind(event_id).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "failed");
    assert_eq!(reason.as_deref(), Some("bad msisdn"));
}

// NIP-4 — dedup is per-recipient: a new recipient on the SAME event still gets notified.
#[tokio::test]
async fn nip4_dedup_is_per_recipient() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = with_template(&pool, company).await;
    let port = FakeComm::new();
    let event_id = Uuid::new_v4();

    let mk = |addr: &str| NotifyEvent {
        event_id, event_type: "InvoiceDue".into(), channel: "whatsapp".into(),
        recipients: vec![Recipient { party_id: None, address: addr.into() }], data: json!({}),
    };
    let (a, b) = with_org_scope(&pool, company, async {
        let a = svc.notify(mk("+628111"), &port, &LoggingSink).await.unwrap();
        let b = svc.notify(mk("+628222"), &port, &LoggingSink).await.unwrap();
        (a, b)
    }).await;
    assert_eq!(a.dispatched, 1);
    assert_eq!(b.dispatched, 1, "a different recipient on the same event is a distinct notification");
    assert_eq!(port.count(), 2);
}

// NIP-5 — the reaper re-drives a stranded 'pending' notification (maturity council 2026-07-08). A crash
// between the committed 'pending' INSERT and the dispatch leaves a row 'pending' that the (event_id,
// recipient) dedup then protects from ever being re-sent by a redelivery; `dispatch_pending` recovers it,
// keyed on STATE, carrying the notification's idempotency key so the re-drive can't double-notify.
#[tokio::test]
async fn nip5_reaper_redrives_stranded_pending() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = with_template(&pool, company).await;

    // A row stranded 'pending' — models a crash after the slot was claimed but before dispatch.
    let notification_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO notification.notifications
             (id, event_id, event_type, channel, recipient_address, body, status)
           VALUES ($1,$2,'InvoiceDue','whatsapp'::notif_channel,$3,$4,'pending'::notification_status)"#,
    )
    .bind(notification_id).bind(event_id).bind("+628999").bind("Tagihan jatuh tempo")
    .execute(&pool).await.unwrap();

    let port = FakeComm::new();
    let n = with_org_scope(&pool, company, async {
        svc.dispatch_pending(50, &port, &LoggingSink).await.unwrap()
    }).await;
    assert!(n >= 1, "the reaper re-dispatched the stranded notification");

    let status: String = sqlx::query_scalar(
        "SELECT status::text FROM notification.notifications WHERE id=$1")
        .bind(notification_id).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "sent", "the stranded notification finally reached the recipient");
    // The re-drive carried the notification's idempotency key (so the gateway can dedup a double send).
    let dispatched = port.dispatches.lock().unwrap().clone();
    assert!(dispatched.iter().any(|d| d.idempotency_key == notification_id.to_string()),
        "dispatch carries the idempotency key");
}
