use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::NotificationType;
use super::NotificationChannel;
use super::NotificationStatus;
use super::NotificationPriority;
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
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub channel: NotificationChannel,
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<Uuid>,
    pub status: NotificationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivered_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub retry_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub priority: NotificationPriority,
    pub is_actionable: bool,
    pub action_taken: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_taken_at: Option<DateTime<Utc>>,
    pub is_dismissed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dismissed_at: Option<DateTime<Utc>>,
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
    pub fn new(user_id: Uuid, notification_type: NotificationType, channel: NotificationChannel, title: String, body: String, status: NotificationStatus, retry_count: i32, priority: NotificationPriority, is_actionable: bool, action_taken: bool, is_dismissed: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            notification_type,
            channel,
            title,
            body,
            image_url: None,
            action_url: None,
            action_type: None,
            reference_type: None,
            reference_id: None,
            status,
            scheduled_at: None,
            sent_at: None,
            delivered_at: None,
            read_at: None,
            failed_at: None,
            failure_reason: None,
            retry_count,
            external_id: None,
            priority,
            is_actionable,
            action_taken,
            action_taken_at: None,
            is_dismissed,
            dismissed_at: None,
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

    /// Set the image_url field (chainable)
    pub fn with_image_url(mut self, value: String) -> Self {
        self.image_url = Some(value);
        self
    }

    /// Set the action_url field (chainable)
    pub fn with_action_url(mut self, value: String) -> Self {
        self.action_url = Some(value);
        self
    }

    /// Set the action_type field (chainable)
    pub fn with_action_type(mut self, value: String) -> Self {
        self.action_type = Some(value);
        self
    }

    /// Set the reference_type field (chainable)
    pub fn with_reference_type(mut self, value: String) -> Self {
        self.reference_type = Some(value);
        self
    }

    /// Set the reference_id field (chainable)
    pub fn with_reference_id(mut self, value: Uuid) -> Self {
        self.reference_id = Some(value);
        self
    }

    /// Set the scheduled_at field (chainable)
    pub fn with_scheduled_at(mut self, value: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(value);
        self
    }

    /// Set the sent_at field (chainable)
    pub fn with_sent_at(mut self, value: DateTime<Utc>) -> Self {
        self.sent_at = Some(value);
        self
    }

    /// Set the delivered_at field (chainable)
    pub fn with_delivered_at(mut self, value: DateTime<Utc>) -> Self {
        self.delivered_at = Some(value);
        self
    }

    /// Set the read_at field (chainable)
    pub fn with_read_at(mut self, value: DateTime<Utc>) -> Self {
        self.read_at = Some(value);
        self
    }

    /// Set the failed_at field (chainable)
    pub fn with_failed_at(mut self, value: DateTime<Utc>) -> Self {
        self.failed_at = Some(value);
        self
    }

    /// Set the failure_reason field (chainable)
    pub fn with_failure_reason(mut self, value: String) -> Self {
        self.failure_reason = Some(value);
        self
    }

    /// Set the external_id field (chainable)
    pub fn with_external_id(mut self, value: String) -> Self {
        self.external_id = Some(value);
        self
    }

    /// Set the action_taken_at field (chainable)
    pub fn with_action_taken_at(mut self, value: DateTime<Utc>) -> Self {
        self.action_taken_at = Some(value);
        self
    }

    /// Set the dismissed_at field (chainable)
    pub fn with_dismissed_at(mut self, value: DateTime<Utc>) -> Self {
        self.dismissed_at = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "user_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.user_id = v; }
                }
                "notification_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.notification_type = v; }
                }
                "channel" => {
                    if let Ok(v) = serde_json::from_value(value) { self.channel = v; }
                }
                "title" => {
                    if let Ok(v) = serde_json::from_value(value) { self.title = v; }
                }
                "body" => {
                    if let Ok(v) = serde_json::from_value(value) { self.body = v; }
                }
                "image_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.image_url = v; }
                }
                "action_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.action_url = v; }
                }
                "action_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.action_type = v; }
                }
                "reference_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.reference_type = v; }
                }
                "reference_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.reference_id = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "scheduled_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.scheduled_at = v; }
                }
                "sent_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.sent_at = v; }
                }
                "delivered_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.delivered_at = v; }
                }
                "read_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.read_at = v; }
                }
                "failed_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failed_at = v; }
                }
                "failure_reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failure_reason = v; }
                }
                "retry_count" => {
                    if let Ok(v) = serde_json::from_value(value) { self.retry_count = v; }
                }
                "external_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.external_id = v; }
                }
                "priority" => {
                    if let Ok(v) = serde_json::from_value(value) { self.priority = v; }
                }
                "is_actionable" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_actionable = v; }
                }
                "action_taken" => {
                    if let Ok(v) = serde_json::from_value(value) { self.action_taken = v; }
                }
                "action_taken_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.action_taken_at = v; }
                }
                "is_dismissed" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_dismissed = v; }
                }
                "dismissed_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.dismissed_at = v; }
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
        m.insert("user_id".to_string(), "uuid".to_string());
        m.insert("reference_id".to_string(), "uuid".to_string());
        m.insert("notification_type".to_string(), "notification_type".to_string());
        m.insert("channel".to_string(), "notification_channel".to_string());
        m.insert("status".to_string(), "notification_status".to_string());
        m.insert("priority".to_string(), "notification_priority".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["title", "body"]
    }
}

/// Builder for Notification entity
///
/// Provides a fluent API for constructing Notification instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct NotificationBuilder {
    user_id: Option<Uuid>,
    notification_type: Option<NotificationType>,
    channel: Option<NotificationChannel>,
    title: Option<String>,
    body: Option<String>,
    image_url: Option<String>,
    action_url: Option<String>,
    action_type: Option<String>,
    reference_type: Option<String>,
    reference_id: Option<Uuid>,
    status: Option<NotificationStatus>,
    scheduled_at: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    delivered_at: Option<DateTime<Utc>>,
    read_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    retry_count: Option<i32>,
    external_id: Option<String>,
    priority: Option<NotificationPriority>,
    is_actionable: Option<bool>,
    action_taken: Option<bool>,
    action_taken_at: Option<DateTime<Utc>>,
    is_dismissed: Option<bool>,
    dismissed_at: Option<DateTime<Utc>>,
}

impl NotificationBuilder {
    /// Set the user_id field (required)
    pub fn user_id(mut self, value: Uuid) -> Self {
        self.user_id = Some(value);
        self
    }

    /// Set the notification_type field (required)
    pub fn notification_type(mut self, value: NotificationType) -> Self {
        self.notification_type = Some(value);
        self
    }

    /// Set the channel field (required)
    pub fn channel(mut self, value: NotificationChannel) -> Self {
        self.channel = Some(value);
        self
    }

    /// Set the title field (required)
    pub fn title(mut self, value: String) -> Self {
        self.title = Some(value);
        self
    }

    /// Set the body field (required)
    pub fn body(mut self, value: String) -> Self {
        self.body = Some(value);
        self
    }

    /// Set the image_url field (optional)
    pub fn image_url(mut self, value: String) -> Self {
        self.image_url = Some(value);
        self
    }

    /// Set the action_url field (optional)
    pub fn action_url(mut self, value: String) -> Self {
        self.action_url = Some(value);
        self
    }

    /// Set the action_type field (optional)
    pub fn action_type(mut self, value: String) -> Self {
        self.action_type = Some(value);
        self
    }

    /// Set the reference_type field (optional)
    pub fn reference_type(mut self, value: String) -> Self {
        self.reference_type = Some(value);
        self
    }

    /// Set the reference_id field (optional)
    pub fn reference_id(mut self, value: Uuid) -> Self {
        self.reference_id = Some(value);
        self
    }

    /// Set the status field (default: `NotificationStatus::default()`)
    pub fn status(mut self, value: NotificationStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the scheduled_at field (optional)
    pub fn scheduled_at(mut self, value: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(value);
        self
    }

    /// Set the sent_at field (optional)
    pub fn sent_at(mut self, value: DateTime<Utc>) -> Self {
        self.sent_at = Some(value);
        self
    }

    /// Set the delivered_at field (optional)
    pub fn delivered_at(mut self, value: DateTime<Utc>) -> Self {
        self.delivered_at = Some(value);
        self
    }

    /// Set the read_at field (optional)
    pub fn read_at(mut self, value: DateTime<Utc>) -> Self {
        self.read_at = Some(value);
        self
    }

    /// Set the failed_at field (optional)
    pub fn failed_at(mut self, value: DateTime<Utc>) -> Self {
        self.failed_at = Some(value);
        self
    }

    /// Set the failure_reason field (optional)
    pub fn failure_reason(mut self, value: String) -> Self {
        self.failure_reason = Some(value);
        self
    }

    /// Set the retry_count field (default: `0`)
    pub fn retry_count(mut self, value: i32) -> Self {
        self.retry_count = Some(value);
        self
    }

    /// Set the external_id field (optional)
    pub fn external_id(mut self, value: String) -> Self {
        self.external_id = Some(value);
        self
    }

    /// Set the priority field (default: `NotificationPriority::default()`)
    pub fn priority(mut self, value: NotificationPriority) -> Self {
        self.priority = Some(value);
        self
    }

    /// Set the is_actionable field (default: `false`)
    pub fn is_actionable(mut self, value: bool) -> Self {
        self.is_actionable = Some(value);
        self
    }

    /// Set the action_taken field (default: `false`)
    pub fn action_taken(mut self, value: bool) -> Self {
        self.action_taken = Some(value);
        self
    }

    /// Set the action_taken_at field (optional)
    pub fn action_taken_at(mut self, value: DateTime<Utc>) -> Self {
        self.action_taken_at = Some(value);
        self
    }

    /// Set the is_dismissed field (default: `false`)
    pub fn is_dismissed(mut self, value: bool) -> Self {
        self.is_dismissed = Some(value);
        self
    }

    /// Set the dismissed_at field (optional)
    pub fn dismissed_at(mut self, value: DateTime<Utc>) -> Self {
        self.dismissed_at = Some(value);
        self
    }

    /// Build the Notification entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Notification, String> {
        let user_id = self.user_id.ok_or_else(|| "user_id is required".to_string())?;
        let notification_type = self.notification_type.ok_or_else(|| "notification_type is required".to_string())?;
        let channel = self.channel.ok_or_else(|| "channel is required".to_string())?;
        let title = self.title.ok_or_else(|| "title is required".to_string())?;
        let body = self.body.ok_or_else(|| "body is required".to_string())?;

        Ok(Notification {
            id: Uuid::new_v4(),
            user_id,
            notification_type,
            channel,
            title,
            body,
            image_url: self.image_url,
            action_url: self.action_url,
            action_type: self.action_type,
            reference_type: self.reference_type,
            reference_id: self.reference_id,
            status: self.status.unwrap_or(NotificationStatus::default()),
            scheduled_at: self.scheduled_at,
            sent_at: self.sent_at,
            delivered_at: self.delivered_at,
            read_at: self.read_at,
            failed_at: self.failed_at,
            failure_reason: self.failure_reason,
            retry_count: self.retry_count.unwrap_or(0),
            external_id: self.external_id,
            priority: self.priority.unwrap_or(NotificationPriority::default()),
            is_actionable: self.is_actionable.unwrap_or(false),
            action_taken: self.action_taken.unwrap_or(false),
            action_taken_at: self.action_taken_at,
            is_dismissed: self.is_dismissed.unwrap_or(false),
            dismissed_at: self.dismissed_at,
            metadata: AuditMetadata::default(),
        })
    }
}
