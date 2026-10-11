//! The module's extension (hand-written; ADR-0031): what the module adds beside its
//! generated services. The generated module and builder carry `ModuleExt` and
//! `ModuleBuilderExt` and dereference to them, so their fields read as `module.field`.

#[allow(unused_imports)]
use super::*;

/// State the module adds; read through the generated module's `Deref`.
pub struct ModuleExt {
    /// The validated write engine — idempotent fan-out, template render, dispatch through
    /// `CommunicationPort`. This delivers the module's stated purpose; the two CRUD services
    /// above are convenience scaffolding (see `readonly_routes` / `guarded_routes`).
    pub(crate) write_service: Arc<application::service::NotificationWriteService>,
}

/// State the builder adds.
pub struct ModuleBuilderExt {
}

impl Default for ModuleBuilderExt {
    fn default() -> Self {
        Self {
        }
    }
}

impl ModuleBuilderExt {
    /// Build the extension's state from what the generated build made.
    #[allow(unused_variables, clippy::redundant_clone)]
    pub(crate) fn build(self, parts: &ModuleParts<'_>) -> anyhow::Result<ModuleExt> {
        let db_pool = parts.db_pool.clone();
        // The validated write engine. Self-constructs from the pool; CommunicationPort and
        // NotificationEventSink stay per-call (the composing service's concern).
        let write_service = Arc::new(application::service::NotificationWriteService::new(db_pool.clone()));
        Ok(ModuleExt {
            write_service,
        })
    }
}

impl crate::ThisModule {
    /// The validated notification engine (idempotent fan-out → render → dispatch).
    ///
    /// A composing service drives this from its domain-event subscriptions, passing a
    /// `CommunicationPort` impl over backbone-communication. Generic CRUD on a `Notification`
    /// row bypasses the dedup invariant — use THIS, not the write routes.
    pub fn write_service(&self) -> Arc<application::service::NotificationWriteService> {
        self.write_service.clone()
    }
    /// The recommended production surface: `Notification` **read-only** (a Notification is a
    /// derived, append-only record — never hand-created) + `NotificationTemplate` full CRUD
    /// (a template IS master data). `Notification` writes go through [`Self::write_service`]
    /// only. `all_crud_routes()` exposes unguarded writes and is for trusted/admin/seeding.
    pub fn guarded_routes(&self) -> Router {
        use presentation::http::{
            create_notification_read_routes,
            create_notification_template_routes,
        };
        Router::new()
            .merge(create_notification_read_routes(self.notification_service.clone()))
            .merge(create_notification_template_routes(self.notification_template_service.clone()))
    }
}
