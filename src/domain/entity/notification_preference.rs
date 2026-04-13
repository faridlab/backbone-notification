use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::NotificationChannel;
use super::NotificationCategory;
use super::PreferenceStatus;

/// Strongly-typed ID for NotificationPreference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotificationPreferenceId(pub Uuid);

impl NotificationPreferenceId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for NotificationPreferenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for NotificationPreferenceId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for NotificationPreferenceId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<NotificationPreferenceId> for Uuid {
    fn from(id: NotificationPreferenceId) -> Self { id.0 }
}

impl AsRef<Uuid> for NotificationPreferenceId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for NotificationPreferenceId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationPreference {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel: NotificationChannel,
    pub category: NotificationCategory,
    pub status: PreferenceStatus,
    pub is_enabled: bool,
    pub quiet_hours_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_day: Option<i32>,
    pub data: serde_json::Value,
}

impl NotificationPreference {
    /// Create a builder for NotificationPreference
    pub fn builder() -> NotificationPreferenceBuilder {
        NotificationPreferenceBuilder::default()
    }

    /// Create a new NotificationPreference with required fields
    pub fn new(user_id: Uuid, channel: NotificationChannel, category: NotificationCategory, status: PreferenceStatus, is_enabled: bool, quiet_hours_enabled: bool, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            channel,
            category,
            status,
            is_enabled,
            quiet_hours_enabled,
            quiet_start: None,
            quiet_end: None,
            frequency: None,
            max_per_day: None,
            data,
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> NotificationPreferenceId {
        NotificationPreferenceId(self.id)
    }

    /// Get the current status
    pub fn status(&self) -> &PreferenceStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the quiet_start field (chainable)
    pub fn with_quiet_start(mut self, value: String) -> Self {
        self.quiet_start = Some(value);
        self
    }

    /// Set the quiet_end field (chainable)
    pub fn with_quiet_end(mut self, value: String) -> Self {
        self.quiet_end = Some(value);
        self
    }

    /// Set the frequency field (chainable)
    pub fn with_frequency(mut self, value: String) -> Self {
        self.frequency = Some(value);
        self
    }

    /// Set the max_per_day field (chainable)
    pub fn with_max_per_day(mut self, value: i32) -> Self {
        self.max_per_day = Some(value);
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
                "channel" => {
                    if let Ok(v) = serde_json::from_value(value) { self.channel = v; }
                }
                "category" => {
                    if let Ok(v) = serde_json::from_value(value) { self.category = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "is_enabled" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_enabled = v; }
                }
                "quiet_hours_enabled" => {
                    if let Ok(v) = serde_json::from_value(value) { self.quiet_hours_enabled = v; }
                }
                "quiet_start" => {
                    if let Ok(v) = serde_json::from_value(value) { self.quiet_start = v; }
                }
                "quiet_end" => {
                    if let Ok(v) = serde_json::from_value(value) { self.quiet_end = v; }
                }
                "frequency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.frequency = v; }
                }
                "max_per_day" => {
                    if let Ok(v) = serde_json::from_value(value) { self.max_per_day = v; }
                }
                "data" => {
                    if let Ok(v) = serde_json::from_value(value) { self.data = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for NotificationPreference {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "NotificationPreference"
    }
}

impl backbone_core::PersistentEntity for NotificationPreference {
    fn entity_id(&self) -> String {
        self.id.to_string()
    }
    fn set_entity_id(&mut self, id: String) {
        if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            self.id = uuid;
        }
    }
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }
    fn set_created_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        let _ = ts;
    }
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }
    fn set_updated_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        let _ = ts;
    }
    fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }
    fn set_deleted_at(&mut self, ts: Option<chrono::DateTime<chrono::Utc>>) {
        let _ = ts;
    }
}

impl backbone_orm::EntityRepoMeta for NotificationPreference {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("user_id".to_string(), "uuid".to_string());
        m.insert("channel".to_string(), "notification_channel".to_string());
        m.insert("category".to_string(), "notification_category".to_string());
        m.insert("status".to_string(), "preference_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
}

/// Builder for NotificationPreference entity
///
/// Provides a fluent API for constructing NotificationPreference instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct NotificationPreferenceBuilder {
    user_id: Option<Uuid>,
    channel: Option<NotificationChannel>,
    category: Option<NotificationCategory>,
    status: Option<PreferenceStatus>,
    is_enabled: Option<bool>,
    quiet_hours_enabled: Option<bool>,
    quiet_start: Option<String>,
    quiet_end: Option<String>,
    frequency: Option<String>,
    max_per_day: Option<i32>,
    data: Option<serde_json::Value>,
}

impl NotificationPreferenceBuilder {
    /// Set the user_id field (required)
    pub fn user_id(mut self, value: Uuid) -> Self {
        self.user_id = Some(value);
        self
    }

    /// Set the channel field (required)
    pub fn channel(mut self, value: NotificationChannel) -> Self {
        self.channel = Some(value);
        self
    }

    /// Set the category field (required)
    pub fn category(mut self, value: NotificationCategory) -> Self {
        self.category = Some(value);
        self
    }

    /// Set the status field (default: `PreferenceStatus::default()`)
    pub fn status(mut self, value: PreferenceStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the is_enabled field (default: `true`)
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = Some(value);
        self
    }

    /// Set the quiet_hours_enabled field (default: `false`)
    pub fn quiet_hours_enabled(mut self, value: bool) -> Self {
        self.quiet_hours_enabled = Some(value);
        self
    }

    /// Set the quiet_start field (optional)
    pub fn quiet_start(mut self, value: String) -> Self {
        self.quiet_start = Some(value);
        self
    }

    /// Set the quiet_end field (optional)
    pub fn quiet_end(mut self, value: String) -> Self {
        self.quiet_end = Some(value);
        self
    }

    /// Set the frequency field (optional)
    pub fn frequency(mut self, value: String) -> Self {
        self.frequency = Some(value);
        self
    }

    /// Set the max_per_day field (optional)
    pub fn max_per_day(mut self, value: i32) -> Self {
        self.max_per_day = Some(value);
        self
    }

    /// Set the data field (default: `serde_json::json!({})`)
    pub fn data(mut self, value: serde_json::Value) -> Self {
        self.data = Some(value);
        self
    }

    /// Build the NotificationPreference entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<NotificationPreference, String> {
        let user_id = self.user_id.ok_or_else(|| "user_id is required".to_string())?;
        let channel = self.channel.ok_or_else(|| "channel is required".to_string())?;
        let category = self.category.ok_or_else(|| "category is required".to_string())?;

        Ok(NotificationPreference {
            id: Uuid::new_v4(),
            user_id,
            channel,
            category,
            status: self.status.unwrap_or(PreferenceStatus::default()),
            is_enabled: self.is_enabled.unwrap_or(true),
            quiet_hours_enabled: self.quiet_hours_enabled.unwrap_or(false),
            quiet_start: self.quiet_start,
            quiet_end: self.quiet_end,
            frequency: self.frequency,
            max_per_day: self.max_per_day,
            data: self.data.unwrap_or(serde_json::json!({})),
        })
    }
}
