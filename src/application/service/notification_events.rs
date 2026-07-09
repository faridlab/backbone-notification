//! Notification domain events (hand-authored, user-owned) — a small observability surface. Notification is
//! a terminal consumer (it reacts to others' events), so it emits only the fate of a dispatch, for audit
//! and ops dashboards. A consuming service supplies the sink.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The notification event union.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum NotificationEvent {
    NotificationDispatched { notification_id: Uuid, event_id: Uuid, message_id: Uuid },
    NotificationFailed { notification_id: Uuid, event_id: Uuid, reason: String },
    /// The provider confirmed delivery — the loop closed from communication's MessageDelivered.
    NotificationDelivered { notification_id: Uuid, event_id: Uuid },
    /// The provider reported non-delivery after hand-off — from communication's MessageFailed.
    NotificationUndelivered { notification_id: Uuid, event_id: Uuid, reason: String },
}

/// Sink the write path publishes to. A consuming service supplies its own (bus, outbox, …).
pub trait NotificationEventSink: Send + Sync {
    fn publish(&self, event: &NotificationEvent);
}

/// A no-op/logging sink for tests and single-process composition.
#[derive(Debug, Default, Clone)]
pub struct LoggingSink;

impl NotificationEventSink for LoggingSink {
    fn publish(&self, event: &NotificationEvent) {
        tracing::info!(?event, "notification event");
    }
}
