//! The dispatch seam against the REAL backbone-communication module. A notification fan-out dispatches
//! through communication's `send_outbound`, landing a genuine outbound Message on a real thread. Proves
//! the fan-out reaches the channel gateway. ZERO normal Cargo edge — communication is reached through the
//! `CommunicationPort`, implemented over the REAL module only in the test.

mod common;
use common::*;

use backbone_notification::application::service::notification_events::LoggingSink;
use backbone_notification::application::service::notification_write_service::*;
use serde_json::json;
use uuid::Uuid;

// NSEAM-1 — a notification dispatches as a REAL communication outbound message.
#[tokio::test]
async fn nseam1_notification_lands_real_communication_message() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let svc = NotificationWriteService::new(pool.clone());
    let port = RealCommPort::new(pool.clone());

    svc.create_template(NewTemplate {
        company_id: company, event_type: "SLABreached".into(), channel: "whatsapp".into(),
        name: "SLA breach".into(), subject_template: None,
        body_template: "Tiket {{ticket}} melewati SLA.".into(),
    }).await.unwrap();

    let ev = NotifyEvent {
        company_id: company, event_id: Uuid::new_v4(), event_type: "SLABreached".into(),
        channel: "whatsapp".into(),
        recipients: vec![Recipient { party_id: Some(Uuid::new_v4()), address: "+628123".into() }],
        data: json!({"ticket": "ISS-42"}),
    };
    let event_id = ev.event_id;
    let out = svc.notify(ev, &port, &LoggingSink).await.unwrap();
    assert_eq!(out.dispatched, 1);

    // The notification recorded the REAL communication message id it dispatched to.
    let (status, message_id): (String, Option<Uuid>) = sqlx::query_as(
        "SELECT status::text, message_id FROM notification.notifications WHERE event_id=$1")
        .bind(event_id).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "sent");
    let message_id = message_id.expect("dispatched to a communication message");

    // That message exists in the REAL communication schema, outbound, sent, carrying the rendered body.
    let (dir, cstatus, body): (String, String, String) = sqlx::query_as(
        "SELECT direction::text, status::text, body FROM communication.messages WHERE id=$1")
        .bind(message_id).fetch_one(&pool).await.unwrap();
    assert_eq!(dir, "outbound");
    assert_eq!(cstatus, "sent");
    assert_eq!(body, "Tiket ISS-42 melewati SLA.");
}
