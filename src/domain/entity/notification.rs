use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::NotifChannel;
use super::NotificationStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Notification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotificationId(pub Uuid);

impl NotificationId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for NotificationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for NotificationId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for NotificationId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<NotificationId> for Uuid {
    fn from(id: NotificationId) -> Self { id.0 }
}

impl AsRef<Uuid> for NotificationId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for NotificationId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub company_id: Uuid,
    pub event_id: Uuid,
    pub event_type: String,
    pub template_id: Option<Uuid>,
    pub channel: NotifChannel,
    pub recipient_party_id: Option<Uuid>,
    pub recipient_address: String,
    pub subject: Option<String>,
    pub body: String,
    pub status: NotificationStatus,
    pub message_id: Option<Uuid>,
    pub failure_reason: Option<String>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Notification {
    /// Create a builder for Notification
    pub fn builder() -> NotificationBuilder {
        NotificationBuilder::default()
    }

    /// Create a new Notification with required fields
    pub fn new(company_id: Uuid, event_id: Uuid, event_type: String, channel: NotifChannel, recipient_address: String, body: String, status: NotificationStatus) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            event_id,
            event_type,
            template_id: None,
            channel,
            recipient_party_id: None,
            recipient_address,
            subject: None,
            body,
            status,
            message_id: None,
            failure_reason: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> NotificationId {
        NotificationId(self.id)
    }

    /// Get when this entity was created
    pub fn created_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.created_at.as_ref()
    }

    /// Get when this entity was last updated
    pub fn updated_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.updated_at.as_ref()
    }

    /// Check if this entity is soft deleted
    pub fn is_deleted(&self) -> bool {
        self.metadata.deleted_at.is_some()
    }

    /// Check if this entity is active (not deleted)
    pub fn is_active(&self) -> bool {
        self.metadata.deleted_at.is_none()
    }

    /// Get when this entity was deleted
    pub fn deleted_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.deleted_at.as_ref()
    }

    /// Get who created this entity
    pub fn created_by(&self) -> Option<&Uuid> {
        self.metadata.created_by.as_ref()
    }

    /// Get who last updated this entity
    pub fn updated_by(&self) -> Option<&Uuid> {
        self.metadata.updated_by.as_ref()
    }

    /// Get who deleted this entity
    pub fn deleted_by(&self) -> Option<&Uuid> {
        self.metadata.deleted_by.as_ref()
    }

    /// Get the current status
    pub fn status(&self) -> &NotificationStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the template_id field (chainable)
    pub fn with_template_id(mut self, value: Uuid) -> Self {
        self.template_id = Some(value);
        self
    }

    /// Set the recipient_party_id field (chainable)
    pub fn with_recipient_party_id(mut self, value: Uuid) -> Self {
        self.recipient_party_id = Some(value);
        self
    }

    /// Set the subject field (chainable)
    pub fn with_subject(mut self, value: String) -> Self {
        self.subject = Some(value);
        self
    }

    /// Set the message_id field (chainable)
    pub fn with_message_id(mut self, value: Uuid) -> Self {
        self.message_id = Some(value);
        self
    }

    /// Set the failure_reason field (chainable)
    pub fn with_failure_reason(mut self, value: String) -> Self {
        self.failure_reason = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "company_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.company_id = v; }
                }
                "event_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.event_id = v; }
                }
                "event_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.event_type = v; }
                }
                "template_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.template_id = v; }
                }
                "channel" => {
                    if let Ok(v) = serde_json::from_value(value) { self.channel = v; }
                }
                "recipient_party_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.recipient_party_id = v; }
                }
                "recipient_address" => {
                    if let Ok(v) = serde_json::from_value(value) { self.recipient_address = v; }
                }
                "subject" => {
                    if let Ok(v) = serde_json::from_value(value) { self.subject = v; }
                }
                "body" => {
                    if let Ok(v) = serde_json::from_value(value) { self.body = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "message_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.message_id = v; }
                }
                "failure_reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failure_reason = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for Notification {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Notification"
    }
}

impl backbone_core::PersistentEntity for Notification {
    fn entity_id(&self) -> String {
        self.id.to_string()
    }
    fn set_entity_id(&mut self, id: String) {
        if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            self.id = uuid;
        }
    }
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.created_at
    }
    fn set_created_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.created_at = Some(ts);
    }
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.updated_at
    }
    fn set_updated_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.updated_at = Some(ts);
    }
    fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.deleted_at
    }
    fn set_deleted_at(&mut self, ts: Option<chrono::DateTime<chrono::Utc>>) {
        self.metadata.deleted_at = ts;
    }
}

impl backbone_orm::EntityRepoMeta for Notification {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("event_id".to_string(), "uuid".to_string());
        m.insert("template_id".to_string(), "uuid".to_string());
        m.insert("recipient_party_id".to_string(), "uuid".to_string());
        m.insert("message_id".to_string(), "uuid".to_string());
        m.insert("channel".to_string(), "notif_channel".to_string());
        m.insert("status".to_string(), "notification_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["event_type", "recipient_address", "body"]
    }
}

/// Builder for Notification entity
///
/// Provides a fluent API for constructing Notification instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct NotificationBuilder {
    company_id: Option<Uuid>,
    event_id: Option<Uuid>,
    event_type: Option<String>,
    template_id: Option<Uuid>,
    channel: Option<NotifChannel>,
    recipient_party_id: Option<Uuid>,
    recipient_address: Option<String>,
    subject: Option<String>,
    body: Option<String>,
    status: Option<NotificationStatus>,
    message_id: Option<Uuid>,
    failure_reason: Option<String>,
}

impl NotificationBuilder {
    /// Set the company_id field (required)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the event_id field (required)
    pub fn event_id(mut self, value: Uuid) -> Self {
        self.event_id = Some(value);
        self
    }

    /// Set the event_type field (required)
    pub fn event_type(mut self, value: String) -> Self {
        self.event_type = Some(value);
        self
    }

    /// Set the template_id field (optional)
    pub fn template_id(mut self, value: Uuid) -> Self {
        self.template_id = Some(value);
        self
    }

    /// Set the channel field (required)
    pub fn channel(mut self, value: NotifChannel) -> Self {
        self.channel = Some(value);
        self
    }

    /// Set the recipient_party_id field (optional)
    pub fn recipient_party_id(mut self, value: Uuid) -> Self {
        self.recipient_party_id = Some(value);
        self
    }

    /// Set the recipient_address field (required)
    pub fn recipient_address(mut self, value: String) -> Self {
        self.recipient_address = Some(value);
        self
    }

    /// Set the subject field (optional)
    pub fn subject(mut self, value: String) -> Self {
        self.subject = Some(value);
        self
    }

    /// Set the body field (required)
    pub fn body(mut self, value: String) -> Self {
        self.body = Some(value);
        self
    }

    /// Set the status field (default: `NotificationStatus::default()`)
    pub fn status(mut self, value: NotificationStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the message_id field (optional)
    pub fn message_id(mut self, value: Uuid) -> Self {
        self.message_id = Some(value);
        self
    }

    /// Set the failure_reason field (optional)
    pub fn failure_reason(mut self, value: String) -> Self {
        self.failure_reason = Some(value);
        self
    }

    /// Build the Notification entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Notification, String> {
        let company_id = self.company_id.ok_or_else(|| "company_id is required".to_string())?;
        let event_id = self.event_id.ok_or_else(|| "event_id is required".to_string())?;
        let event_type = self.event_type.ok_or_else(|| "event_type is required".to_string())?;
        let channel = self.channel.ok_or_else(|| "channel is required".to_string())?;
        let recipient_address = self.recipient_address.ok_or_else(|| "recipient_address is required".to_string())?;
        let body = self.body.ok_or_else(|| "body is required".to_string())?;

        Ok(Notification {
            id: Uuid::new_v4(),
            company_id,
            event_id,
            event_type,
            template_id: self.template_id,
            channel,
            recipient_party_id: self.recipient_party_id,
            recipient_address,
            subject: self.subject,
            body,
            status: self.status.unwrap_or(NotificationStatus::default()),
            message_id: self.message_id,
            failure_reason: self.failure_reason,
            metadata: AuditMetadata::default(),
        })
    }
}
