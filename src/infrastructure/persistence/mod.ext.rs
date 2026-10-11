// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

// The hand-written notification SQL's parameter/projection types (see both repositories, declared
// `user_owned` in metaphor.codegen.yaml).
pub use notification_repository::{DeliveredRow, NewNotificationRow, PendingNotificationRow};
pub use notification_template_repository::{ActiveTemplateRow, NewTemplateRow};
