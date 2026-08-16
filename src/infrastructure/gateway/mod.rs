//! Gateway adapters (ADR-0024 seam) — the module-side transports for its
//! outbound ports. Feature-gated so the crate stays transport-free unless a
//! composing service opts in:
//!
//! - `gateway-queue` → [`queue_communication_port::QueueCommunicationPort`],
//!   the `CommunicationPort` over backbone-mail's queues
//!
//! Promoted from backbone-messaging-app (increment 3) so no composing service
//! re-implements the fan-in.

#[cfg(feature = "gateway-queue")]
pub mod queue_communication_port;
#[cfg(feature = "gateway-queue")]
pub use queue_communication_port::QueueCommunicationPort;
