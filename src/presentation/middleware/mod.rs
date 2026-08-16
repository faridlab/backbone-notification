//! Presentation middleware (hand-written; user-owned).
//!
//! Gated on `gateway-queue`: the notification gate binds this module's routes
//! to backbone-mail's partner-identity extensions, so it only exists when the
//! queue-backed composition is compiled in.

#[cfg(feature = "gateway-queue")]
pub mod notification_gate;
#[cfg(feature = "gateway-queue")]
pub use notification_gate::{notification_gate, NotificationGate};
