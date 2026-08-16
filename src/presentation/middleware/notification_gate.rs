//! The notification gate — for fence-none apps composing this company-fenced
//! module (ADR-0014). Two jobs, one layer:
//!
//! 1. **Admin only.** Template CRUD is master data and notification reads are
//!    cross-user, so every notification route requires `IsAdmin` (the
//!    composition's allowlist). Anonymous/guest → 401; authenticated
//!    non-admin → 403.
//! 2. **Sentinel company scope.** backbone-notification's tables are RLS-fenced
//!    on `app.company_id`; a fence-none composing app has no company registry,
//!    so the composition runs under ONE configured platform company id. The
//!    request future is wrapped in `with_company_scope` — every scoped
//!    statement inside (generic CRUD and the write engine alike) sets the GUC
//!    and the fence matches. Without this layer the RLS fails closed and the
//!    surface would silently see zero rows.
//!
//! There is no per-request company selection to get wrong: the id is fixed at
//! boot, one value for the whole deployment — the honest shape for a
//! single-tenant service composing a fenced module.
//!
//! Promoted from backbone-messaging-app (increment 3) so every composing
//! service gates this surface identically.

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use backbone_mail::presentation::middleware::{AuthPartnerId, IsAdmin};
use backbone_orm::company_scope;
use uuid::Uuid;

#[derive(Clone)]
pub struct NotificationGate {
    /// The configured platform company id (parsed + validated at boot).
    pub company: Uuid,
}

pub async fn notification_gate(
    State(gate): State<NotificationGate>,
    req: Request,
    next: Next,
) -> Response {
    let partner = req.extensions().get::<AuthPartnerId>();
    let Some(partner) = partner else {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error": "notification surface requires an authenticated admin"})),
        )
            .into_response();
    };
    let is_admin = req
        .extensions()
        .get::<IsAdmin>()
        .map(|IsAdmin(v)| *v)
        .unwrap_or(false);
    if !is_admin {
        tracing::warn!(
            target: "auth::notification_gate",
            partner = %partner.0,
            "non-admin partner blocked from the notification surface"
        );
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({"error": "notification surface is admin-only"})),
        )
            .into_response();
    }

    company_scope::with_company_scope(Some(gate.company), next.run(req)).await
}
