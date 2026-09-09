//! Golden cases — the manufactured oracle for the fan-out engine: render a template per recipient,
//! dispatch each, dedup per (event_id, recipient), and skip when no template matches. Posts NO GL.

mod common;
use common::*;

use backbone_notification::application::service::notification_events::LoggingSink;
use backbone_notification::application::service::notification_write_service::*;
use serde_json::json;
use uuid::Uuid;

/// Key the templates and events of one golden case on an `event_type` unique to that run. The
/// scratch-DB harness shares ONE undecorated database across test binaries and repeated runs —
/// there is no per-unit fence or per-(event_type, channel) unique to separate rows, so a fixed
/// key would resolve templates left behind by earlier runs and fan out on those instead.
fn fresh_event_type() -> String {
    format!("OrderConfirmed-{}", Uuid::new_v4())
}

async fn template(svc: &NotificationWriteService, event_type: &str) {
    svc.create_template(NewTemplate {
        event_type: event_type.into(), channel: "whatsapp".into(),
        name: "Order confirmed".into(), subject_template: None,
        body_template: "Halo {{name}}, pesanan {{order_no}} dikonfirmasi.".into(),
    }).await.unwrap();
}

fn event(event_type: &str, recipients: Vec<Recipient>) -> NotifyEvent {
    NotifyEvent {
        event_id: Uuid::new_v4(), event_type: event_type.into(),
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
    let port = FakeComm::new();

    let et = fresh_event_type();
    let ev = event(&et, vec![
        Recipient { party_id: Some(Uuid::new_v4()), address: "+628111".into() },
        Recipient { party_id: Some(Uuid::new_v4()), address: "+628222".into() },
    ]);
    let out = with_org_scope(&pool, company, async {
        template(&svc, &et).await;
        svc.notify(ev, &port, &LoggingSink).await.unwrap()
    }).await;
    assert_eq!(out.dispatched, 2);
    assert_eq!(port.count(), 2, "two messages dispatched to the gateway");
}

// NGC-2 — the template renders {{placeholders}} from the event data.
#[tokio::test]
async fn ngc2_template_renders_placeholders() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    let port = FakeComm::new();

    let et = fresh_event_type();
    let ev = event(&et, vec![Recipient { party_id: None, address: "+628111".into() }]);
    with_org_scope(&pool, company, async {
        template(&svc, &et).await;
        svc.notify(ev, &port, &LoggingSink).await.unwrap();
    }).await;
    assert_eq!(port.bodies(), vec!["Halo Budi, pesanan SO-1001 dikonfirmasi."]);
}

// NGC-3 — idempotent per (event_id, recipient): a redelivered domain event does not double-notify.
#[tokio::test]
async fn ngc3_idempotent_per_event_and_recipient() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    let port = FakeComm::new();

    let et = fresh_event_type();
    let recipients = vec![Recipient { party_id: None, address: "+628111".into() }];
    let ev1 = event(&et, recipients.clone_via());
    let event_id = ev1.event_id;

    let (first, second) = with_org_scope(&pool, company, async {
        template(&svc, &et).await;
        let first = svc.notify(ev1, &port, &LoggingSink).await.unwrap();
        // Same event_id redelivered.
        let ev2 = NotifyEvent {
            event_id, event_type: et.clone(), channel: "whatsapp".into(),
            recipients: recipients.clone_via(), data: serde_json::json!({"name":"Budi","order_no":"SO-1001"}),
        };
        let second = svc.notify(ev2, &port, &LoggingSink).await.unwrap();
        (first, second)
    }).await;

    assert_eq!(first.dispatched, 1);
    assert_eq!(second.deduped, 1, "redelivery deduped — no second notify");
    assert_eq!(second.dispatched, 0);
    assert_eq!(port.count(), 1, "the gateway is hit exactly once for this recipient+event");
}

// NGC-4 — a recipient with no active template for (event_type, channel) is skipped (nothing sent).
#[tokio::test]
async fn ngc4_no_template_skips() {
    let pool = pool().await;
    let svc = NotificationWriteService::new(pool.clone());
    let et = fresh_event_type();
    // No template created for this event_type. The skip path neither inserts nor stages — it
    // returns before anything reaches the company-keyed outbox mirror, so it runs with no org
    // scope bound. The unique key is what guarantees the miss: a shared key would resolve some
    // earlier run's leftover template and fall through to the dispatch path, which does stage.
    let port = FakeComm::new();
    let ev = event(&et, vec![Recipient { party_id: None, address: "+628111".into() }]);
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
    let port = FakeComm::new();
    let sink = CapturingSink::new();

    // Dispatch two recipients → both 'sent' with a message_id.
    let et = fresh_event_type();
    let ev = event(&et, vec![
        Recipient { party_id: None, address: "+628111".into() },
        Recipient { party_id: None, address: "+628222".into() },
    ]);
    let event_id = ev.event_id;

    let (m1, m2) = with_org_scope(&pool, company, async {
        template(&svc, &et).await;
        svc.notify(ev, &port, &sink).await.unwrap();

        let ids: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT message_id, recipient_address FROM notification.notifications WHERE event_id=$1 ORDER BY recipient_address")
            .bind(event_id).fetch_all(&pool).await.unwrap();
        (ids[0].clone().0, ids[1].clone().0)
    }).await;

    // Provider confirms one delivered, one bounced. The receipt transitions stage outbox rows —
    // each relay runs under the org scope so the company-keyed mirror has its key.
    assert!(with_org_scope(&pool, company, async {
        svc.record_delivery(m1, DeliveryOutcome::Delivered, &sink).await.unwrap()
    }).await);
    assert!(with_org_scope(&pool, company, async {
        svc.record_delivery(m2, DeliveryOutcome::Undelivered("no such number".into()), &sink).await.unwrap()
    }).await);
    // A redelivered receipt is a no-op (idempotent, state-guarded on 'sent').
    assert!(!with_org_scope(&pool, company, async {
        svc.record_delivery(m1, DeliveryOutcome::Delivered, &sink).await.unwrap()
    }).await);

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
