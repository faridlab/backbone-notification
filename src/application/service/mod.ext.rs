// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

pub mod notification_ports;
pub mod notification_events;
pub mod notification_write_service;
pub use notification_ports::{CommunicationPort, DispatchAck, DispatchRejected, DispatchRequest};
pub use notification_events::{LoggingSink, NotificationEvent, NotificationEventSink};
pub use notification_write_service::{
    DeliveryOutcome, NewTemplate, NotificationWriteService, NotifyError, NotifyEvent, NotifyOutcome,
    Recipient,
};
