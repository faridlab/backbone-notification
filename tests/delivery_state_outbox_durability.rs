//! Durability probe (outbox rollout plan, P2): the delivery-state event (`NotificationDelivered` /
//! `NotificationUndelivered`) that a consumer escalates on is staged in the transactional outbox in the SAME
//! tx as the status transition, so a crash between the CAS and the in-proc publish cannot drop the escalation
//! signal. The `LoggingSink` drops the in-proc publish; the event must still be staged.

mod common;
use common::*;

use backbone_notification::application::service::notification_events::LoggingSink;
use backbone_notification::application::service::notification_write_service::*;
use serde_json::json;
use uuid::Uuid;

async fn template(svc: &NotificationWriteService) {
    svc.create_template(NewTemplate {
        event_type: "OrderConfirmed".into(), channel: "whatsapp".into(),
        name: "Order confirmed".into(), subject_template: None,
        body_template: "Halo {{name}}.".into(),
    }).await.unwrap();
}

// NOD-1 — recording an undelivered receipt durably stages the delivery-state event despite the dropped publish.
#[tokio::test]
async fn nod1_delivery_state_is_durably_staged() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());

    let ev = NotifyEvent {
        event_id: Uuid::new_v4(), event_type: "OrderConfirmed".into(),
        channel: "whatsapp".into(),
        recipients: vec![Recipient { party_id: None, address: "+628111".into() }],
        data: json!({"name": "Budi"}),
    };
    let event_id = ev.event_id;
    let message_id: Uuid = with_org_scope(&pool, company, async {
        template(&svc).await;
        svc.notify(ev, &FakeComm::new(), &LoggingSink).await.unwrap();
        sqlx::query_scalar(
            "SELECT message_id FROM notification.notifications WHERE event_id=$1")
            .bind(event_id).fetch_one(&pool).await.unwrap()
    }).await;

    // LoggingSink drops the in-proc publish — durability must come from the outbox. The receipt
    // transition stages a company-keyed outbox mirror row, so it relays under the org scope.
    assert!(with_org_scope(&pool, company, async {
        svc.record_delivery(message_id, DeliveryOutcome::Undelivered("no such number".into()), &LoggingSink)
            .await.unwrap()
    }).await);

    let staged: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM notification.outbox_events WHERE event_type='NotificationUndelivered'
         AND aggregate_id IN (SELECT id::text FROM notification.notifications WHERE event_id=$1)")
        .bind(event_id).fetch_one(&pool).await.unwrap();
    assert_eq!(staged, 1, "NotificationUndelivered durably staged despite the dropped in-proc publish");
}
