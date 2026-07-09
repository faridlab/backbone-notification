//! Outbound dispatch port (hand-authored, user-owned) — the seam to backbone-communication.
//!
//! Notification renders a message, then dispatches it through the **channel gateway** (backbone-
//! communication), which owns the actual WhatsApp/email/SMS provider. Notification never imports
//! communication — a composing service implements this port over communication's `send_outbound`; tests
//! supply a fake or drive the REAL communication module. Zero normal Cargo edge.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A rendered message to hand to the channel gateway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchRequest {
    /// Stable per-notification key (the notification id). A composing service forwards it to the gateway
    /// as a provider-level dedup token so the retry reaper (`dispatch_pending`) can re-drive a stranded
    /// notification without double-notifying the recipient (maturity council 2026-07-08).
    pub idempotency_key: String,
    pub company_id: Uuid,
    pub channel: String, // whatsapp | email | sms
    pub recipient_party_id: Option<Uuid>,
    pub recipient_address: String,
    pub subject: Option<String>,
    pub body: String,
}

/// The gateway accepted the message and assigned it a communication message id (the correlation key).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchAck {
    pub message_id: Uuid,
}

/// The gateway rejected the dispatch. `code` is stable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchRejected {
    pub code: String,
    pub message: String,
}

/// The dispatch seam. A composing service implements it over backbone-communication.
#[async_trait::async_trait]
pub trait CommunicationPort: Send + Sync {
    async fn dispatch(&self, req: &DispatchRequest) -> Result<DispatchAck, DispatchRejected>;
}
