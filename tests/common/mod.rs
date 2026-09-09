//! Shared test helpers: a live pool, a fake dispatch port (records dispatches / can reject), a REAL
//! backbone-communication port (dispatches a genuine outbound message through the channel gateway), and a
//! capturing event sink.

#![expect(clippy::expect_used, reason = "test harness: a panic here names the setup failure precisely")]
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use backbone_notification::application::service::notification_events::{
    NotificationEvent, NotificationEventSink,
};
use backbone_notification::application::service::notification_ports::{
    CommunicationPort, DispatchAck, DispatchRejected, DispatchRequest,
};
use sqlx::PgPool;
use uuid::Uuid;

pub fn dburl() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/backbone_notification".into())
}
/// Composition-boot parity for the outbox mirror, run ONCE per test process (the heal's
/// drop-and-recreate policy step is not safe to run concurrently): a composing service's boot
/// heals notification.outbox_events to the framework shape (the company column + fence), and
/// the lifecycle-event staging writes that shape. The once-cell applies the same heal here so
/// the module-era 11-column table the migration chain creates carries it too.
static OUTBOX_HEAL: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

pub async fn pool() -> PgPool {
    let pool = PgPool::connect(&dburl()).await.expect("connect");
    OUTBOX_HEAL
        .get_or_init(|| async {
            backbone_outbox::outbox::migrate(&pool, "notification")
                .await
                .expect("outbox heal");
        })
        .await;
    pool
}

/// Run `f` inside a company-anchored org request scope. The lifecycle events' outbox
/// mirror rows are company-keyed (they mirror the framework-owned outbox table), and
/// their key is sourced from the ambient org scope, failing closed when none is bound
/// (composition-installed tenancy, ADR-0029) — probes that drive a path which stages
/// bind one here. Undecorated probes that neither stage nor depend on a fence run bare.
pub async fn with_org_scope<R, F: std::future::Future<Output = R>>(
    pool: &sqlx::PgPool,
    company: uuid::Uuid,
    f: F,
) -> R {
    backbone_orm::org_scope::with_org_request_scope(
        pool,
        backbone_orm::org_scope::OrgScope::for_company_unit(company),
        f,
    )
    .await
    .expect("org request scope")
}

/// A fake channel gateway. Records every dispatch; assigns a message id, or rejects when armed.
#[derive(Clone, Default)]
pub struct FakeComm {
    pub dispatches: Arc<Mutex<Vec<DispatchRequest>>>,
    pub reject: Arc<Mutex<Option<(String, String)>>>,
}
impl FakeComm {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn rejecting(code: &str, message: &str) -> Self {
        let f = Self::default();
        *f.reject.lock().unwrap() = Some((code.into(), message.into()));
        f
    }
    pub fn count(&self) -> usize {
        self.dispatches.lock().unwrap().len()
    }
    pub fn bodies(&self) -> Vec<String> {
        self.dispatches.lock().unwrap().iter().map(|d| d.body.clone()).collect()
    }
}
#[async_trait::async_trait]
impl CommunicationPort for FakeComm {
    async fn dispatch(&self, req: &DispatchRequest) -> Result<DispatchAck, DispatchRejected> {
        self.dispatches.lock().unwrap().push(req.clone());
        if let Some((code, message)) = self.reject.lock().unwrap().clone() {
            return Err(DispatchRejected { code, message });
        }
        Ok(DispatchAck { message_id: Uuid::new_v4() })
    }
}

/// The ACL over the REAL backbone-communication module: opens a thread and sends a genuine outbound
/// message through the channel gateway (which drives its own fake channel provider).
pub struct RealCommPort {
    pub comm: backbone_communication::application::service::communication_write_service::CommunicationWriteService,
    pub channel: CommFakeChannel,
}
impl RealCommPort {
    pub fn new(pool: PgPool) -> Self {
        Self {
            comm: backbone_communication::application::service::communication_write_service::CommunicationWriteService::new(pool),
            channel: CommFakeChannel,
        }
    }
}
#[async_trait::async_trait]
impl CommunicationPort for RealCommPort {
    async fn dispatch(&self, req: &DispatchRequest) -> Result<DispatchAck, DispatchRejected> {
        use backbone_communication::application::service::communication_events::LoggingSink;
        let thread = self.comm
            .open_thread(&req.channel, req.recipient_party_id, None)
            .await
            .map_err(|e| DispatchRejected { code: "comm_open".into(), message: e.to_string() })?;
        let message_id = self.comm
            .send_outbound(thread, req.recipient_address.clone(), req.body.clone(), &self.channel, &LoggingSink)
            .await
            .map_err(|e| DispatchRejected { code: "comm_send".into(), message: e.to_string() })?;
        Ok(DispatchAck { message_id })
    }
}

/// A fake provider for the REAL communication module's ChannelPort.
pub struct CommFakeChannel;
#[async_trait::async_trait]
impl backbone_communication::application::service::communication_ports::ChannelPort for CommFakeChannel {
    async fn send(
        &self,
        _s: &backbone_communication::application::service::communication_ports::OutboundSend,
    ) -> Result<
        backbone_communication::application::service::communication_ports::ChannelAck,
        backbone_communication::application::service::communication_ports::ChannelRejected,
    > {
        Ok(backbone_communication::application::service::communication_ports::ChannelAck {
            external_id: format!("prov-{}", Uuid::new_v4()),
        })
    }
}

/// Captures notification events.
#[derive(Clone, Default)]
pub struct CapturingSink {
    pub events: Arc<Mutex<Vec<NotificationEvent>>>,
}
impl CapturingSink {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn dispatched(&self) -> usize {
        self.events.lock().unwrap().iter()
            .filter(|e| matches!(e, NotificationEvent::NotificationDispatched { .. })).count()
    }
    pub fn delivered(&self) -> usize {
        self.events.lock().unwrap().iter()
            .filter(|e| matches!(e, NotificationEvent::NotificationDelivered { .. })).count()
    }
    pub fn undelivered(&self) -> usize {
        self.events.lock().unwrap().iter()
            .filter(|e| matches!(e, NotificationEvent::NotificationUndelivered { .. })).count()
    }
}
impl NotificationEventSink for CapturingSink {
    fn publish(&self, event: &NotificationEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}
