//! Golden cases — the manufactured oracle for the fan-out engine: render a template per recipient,
//! dispatch each, dedup per (event_id, recipient), and skip when no template matches. Posts NO GL.

mod common;
use common::*;

use backbone_notification::application::service::notification_events::LoggingSink;
use backbone_notification::application::service::notification_write_service::*;
use serde_json::json;
use uuid::Uuid;

async fn template(svc: &NotificationWriteService, company: Uuid) -> Uuid {
    svc.create_template(NewTemplate {
        company_id: company, event_type: "OrderConfirmed".into(), channel: "whatsapp".into(),
        name: "Order confirmed".into(), subject_template: None,
        body_template: "Halo {{name}}, pesanan {{order_no}} dikonfirmasi.".into(),
    }).await.unwrap()
}

fn event(company: Uuid, recipients: Vec<Recipient>) -> NotifyEvent {
    NotifyEvent {
        company_id: company, event_id: Uuid::new_v4(), event_type: "OrderConfirmed".into(),
        channel: "whatsapp".into(), recipients,
        data: json!({"name": "Budi", "order_no": "SO-1001"}),
    }
}

// NGC-1 — fan-out renders and dispatches one message per recipient.
#[tokio::test]
async fn ngc1_fanout_dispatches_per_recipient() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    template(&svc, company).await;
    let port = FakeComm::new();

    let ev = event(company, vec![
        Recipient { party_id: Some(Uuid::new_v4()), address: "+628111".into() },
        Recipient { party_id: Some(Uuid::new_v4()), address: "+628222".into() },
    ]);
    let out = svc.notify(ev, &port, &LoggingSink).await.unwrap();
    assert_eq!(out.dispatched, 2);
    assert_eq!(port.count(), 2, "two messages dispatched to the gateway");
}

// NGC-2 — the template renders {{placeholders}} from the event data.
#[tokio::test]
async fn ngc2_template_renders_placeholders() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    template(&svc, company).await;
    let port = FakeComm::new();

    let ev = event(company, vec![Recipient { party_id: None, address: "+628111".into() }]);
    svc.notify(ev, &port, &LoggingSink).await.unwrap();
    assert_eq!(port.bodies(), vec!["Halo Budi, pesanan SO-1001 dikonfirmasi."]);
}

// NGC-3 — idempotent per (event_id, recipient): a redelivered domain event does not double-notify.
#[tokio::test]
async fn ngc3_idempotent_per_event_and_recipient() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    template(&svc, company).await;
    let port = FakeComm::new();

    let recipients = vec![Recipient { party_id: None, address: "+628111".into() }];
    let ev1 = event(company, recipients.clone_via());
    let event_id = ev1.event_id;
    let first = svc.notify(ev1, &port, &LoggingSink).await.unwrap();

    // Same event_id redelivered.
    let ev2 = NotifyEvent {
        company_id: company, event_id, event_type: "OrderConfirmed".into(), channel: "whatsapp".into(),
        recipients: recipients.clone_via(), data: serde_json::json!({"name":"Budi","order_no":"SO-1001"}),
    };
    let second = svc.notify(ev2, &port, &LoggingSink).await.unwrap();

    assert_eq!(first.dispatched, 1);
    assert_eq!(second.deduped, 1, "redelivery deduped — no second notify");
    assert_eq!(second.dispatched, 0);
    assert_eq!(port.count(), 1, "the gateway is hit exactly once for this recipient+event");
}

// NGC-4 — a recipient with no active template for (event_type, channel) is skipped (nothing sent).
#[tokio::test]
async fn ngc4_no_template_skips() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    // No template created.
    let port = FakeComm::new();
    let ev = event(company, vec![Recipient { party_id: None, address: "+628111".into() }]);
    let out = svc.notify(ev, &port, &LoggingSink).await.unwrap();
    assert_eq!(out.skipped, 1);
    assert_eq!(out.dispatched, 0);
    assert_eq!(port.count(), 0);
}

// NGC-5 — closing the delivery loop (completeness council 2026-07-08). A `sent` notification means
// "handed to the gateway"; `record_delivery` reconciles the provider's real outcome from communication's
// receipts so a consumer can tell delivered from bounced (and escalate on undelivered).
#[tokio::test]
async fn ngc5_record_delivery_closes_the_loop() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    template(&svc, company).await;
    let port = FakeComm::new();
    let sink = CapturingSink::new();

    // Dispatch two recipients → both 'sent' with a message_id.
    let ev = event(company, vec![
        Recipient { party_id: None, address: "+628111".into() },
        Recipient { party_id: None, address: "+628222".into() },
    ]);
    let event_id = ev.event_id;
    svc.notify(ev, &port, &sink).await.unwrap();

    let ids: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT message_id, recipient_address FROM notification.notifications WHERE event_id=$1 ORDER BY recipient_address")
        .bind(event_id).fetch_all(&pool).await.unwrap();
    let (m1, _) = ids[0].clone();
    let (m2, _) = ids[1].clone();

    // Provider confirms one delivered, one bounced.
    assert!(svc.record_delivery(m1, DeliveryOutcome::Delivered, &sink).await.unwrap());
    assert!(svc.record_delivery(m2, DeliveryOutcome::Undelivered("no such number".into()), &sink).await.unwrap());
    // A redelivered receipt is a no-op (idempotent, state-guarded on 'sent').
    assert!(!svc.record_delivery(m1, DeliveryOutcome::Delivered, &sink).await.unwrap());

    let statuses: Vec<String> = sqlx::query_scalar(
        "SELECT status::text FROM notification.notifications WHERE event_id=$1 ORDER BY recipient_address")
        .bind(event_id).fetch_all(&pool).await.unwrap();
    assert_eq!(statuses, vec!["delivered", "undelivered"], "the loop is closed with the real outcome");
    assert_eq!(sink.delivered(), 1);
    assert_eq!(sink.undelivered(), 1);
}

/// Small helper so a Vec<Recipient> can be reused across two events in a test.
trait CloneVia {
    fn clone_via(&self) -> Vec<Recipient>;
}
impl CloneVia for Vec<Recipient> {
    fn clone_via(&self) -> Vec<Recipient> {
        self.iter().map(|r| Recipient { party_id: r.party_id, address: r.address.clone() }).collect()
    }
}
