//! `QueueCommunicationPort` — the `CommunicationPort` over the SAME gateway
//! the messaging pump uses (increment 3, the plan's "one gateway, two
//! producers": the mail/sms queues are the transport, notification is a second
//! producer fanning into them).
//!
//! Channel map:
//! - `email` → the mail queue (`message_post`, email type). The ack's
//!   `message_id` is the staged mail row's id — the correlation key
//!   `record_delivery` correlates on.
//! - `sms` → the sms queue (`enqueue`, with `notification_id` = the
//!   idempotency key parsed back to the notification row). `sms::process_queue`
//!   drives it out through the selected provider port.
//! - anything else (`whatsapp`…) → a stable `channel_not_composed` rejection —
//!   honest at dispatch time, never a silent drop.
//!
//! Staging errors are rejections (`stage_failed`), which the write service
//! records as `failed` on the notification row; the retry reaper only re-drives
//! `pending`, so a permanently failing dispatch stays visible, not looping.

use std::sync::Arc;

use backbone_mail::application::service::message_write_service::{
    MessagePostCommand, MessageWriteService, NotificationChannel, PostRecipient,
};
use backbone_mail::application::service::sms_write_service::SmsWriteService;

use crate::application::service::notification_ports::{
    CommunicationPort, DispatchAck, DispatchRejected, DispatchRequest,
};

pub struct QueueCommunicationPort {
    mails: Arc<MessageWriteService>,
    sms: Arc<SmsWriteService>,
}

impl QueueCommunicationPort {
    pub fn new(mails: Arc<MessageWriteService>, sms: Arc<SmsWriteService>) -> Self {
        Self { mails, sms }
    }
}

#[async_trait::async_trait]
impl CommunicationPort for QueueCommunicationPort {
    async fn dispatch(&self, req: &DispatchRequest) -> Result<DispatchAck, DispatchRejected> {
        match req.channel.as_str() {
            "email" => {
                let posted = self
                    .mails
                    .message_post(MessagePostCommand {
                        body: req.body.clone(),
                        subject: req.subject.clone(),
                        message_type: "email".into(),
                        recipients: vec![PostRecipient {
                            res_partner_id: req.recipient_party_id,
                            channel: NotificationChannel::Email,
                            email: Some(req.recipient_address.clone()),
                            number: None,
                        }],
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| DispatchRejected {
                        code: "stage_failed".into(),
                        message: format!("mail queue staging failed: {e}"),
                    })?;
                // An email recipient always stages a mail row; a None here
                // means the command degenerated (no recipients honored) —
                // reject rather than ack a correlation key that correlates
                // nothing.
                posted.mail_id.map(|id| DispatchAck { message_id: id }).ok_or_else(|| DispatchRejected {
                    code: "stage_failed".into(),
                    message: "message_post staged no mail row for the email dispatch".into(),
                })
            }
            "sms" => {
                // Tie the sms row to the notification it serves — the
                // idempotency key IS the notification id.
                let notification_id = req.idempotency_key.parse().ok();
                let (sms_id, _uuid) = self
                    .sms
                    .enqueue(
                        &req.recipient_address,
                        &req.body,
                        None,
                        notification_id,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| DispatchRejected {
                        code: "stage_failed".into(),
                        message: format!("sms queue staging failed: {e}"),
                    })?;
                Ok(DispatchAck { message_id: sms_id })
            }
            other => Err(DispatchRejected {
                code: "channel_not_composed".into(),
                message: format!(
                    "channel {other:?} has no transport in this composition (email | sms only)"
                ),
            }),
        }
    }
}
