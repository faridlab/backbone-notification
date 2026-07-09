//! Integrity probes — the fan-out engine's invariants: a recipient needs an address, one template per
//! (event, channel), a dispatch rejection is recorded, and dedup is per-recipient (a new recipient on the
//! same event still sends).

mod common;
use common::*;

use backbone_notification::application::service::notification_events::LoggingSink;
use backbone_notification::application::service::notification_write_service::*;
use serde_json::json;
use uuid::Uuid;

async fn with_template(pool: &sqlx::PgPool, company: Uuid) -> NotificationWriteService {
    let svc = NotificationWriteService::new(pool.clone());
    svc.create_template(NewTemplate {
        company_id: company, event_type: "InvoiceDue".into(), channel: "whatsapp".into(),
        name: "Invoice due".into(), subject_template: None, body_template: "Tagihan jatuh tempo".into(),
    }).await.unwrap();
    svc
}

fn ev(company: Uuid, recipients: Vec<Recipient>) -> NotifyEvent {
    NotifyEvent {
        company_id: company, event_id: Uuid::new_v4(), event_type: "InvoiceDue".into(),
        channel: "whatsapp".into(), recipients, data: json!({}),
    }
}

// NIP-1 — a recipient with a blank address is refused.
#[tokio::test]
async fn nip1_recipient_needs_address() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = with_template(&pool, company).await;
    let r = svc.notify(ev(company, vec![Recipient { party_id: None, address: "  ".into() }]),
        &FakeComm::new(), &LoggingSink).await;
    assert!(matches!(r, Err(NotifyError::Invalid(_))));
}

// NIP-2 — one active template per (company, event_type, channel).
#[tokio::test]
async fn nip2_one_template_per_event_channel() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let _svc = with_template(&pool, company).await;
    let svc2 = NotificationWriteService::new(pool.clone());
    let dup = svc2.create_template(NewTemplate {
        company_id: company, event_type: "InvoiceDue".into(), channel: "whatsapp".into(),
        name: "dup".into(), subject_template: None, body_template: "x".into(),
    }).await;
    assert!(matches!(dup, Err(NotifyError::Invalid(_))), "duplicate template refused");
}

// NIP-3 — a dispatch rejection records a failed notification (not swallowed).
#[tokio::test]
async fn nip3_dispatch_rejection_recorded() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = with_template(&pool, company).await;
    let port = FakeComm::rejecting("invalid_number", "bad msisdn");

    let event = ev(company, vec![Recipient { party_id: None, address: "+628111".into() }]);
    let event_id = event.event_id;
    let out = svc.notify(event, &port, &LoggingSink).await.unwrap();
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
        company_id: company, event_id, event_type: "InvoiceDue".into(), channel: "whatsapp".into(),
        recipients: vec![Recipient { party_id: None, address: addr.into() }], data: json!({}),
    };
    let a = svc.notify(mk("+628111"), &port, &LoggingSink).await.unwrap();
    let b = svc.notify(mk("+628222"), &port, &LoggingSink).await.unwrap();
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
             (id, company_id, event_id, event_type, channel, recipient_address, body, status)
           VALUES ($1,$2,$3,'InvoiceDue','whatsapp'::notif_channel,$4,$5,'pending'::notification_status)"#,
    )
    .bind(notification_id).bind(company).bind(event_id).bind("+628999").bind("Tagihan jatuh tempo")
    .execute(&pool).await.unwrap();

    let port = FakeComm::new();
    let n = svc.dispatch_pending(50, &port, &LoggingSink).await.unwrap();
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
