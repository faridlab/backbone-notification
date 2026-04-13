use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "notification_channel", rename_all = "snake_case")]
pub enum NotificationChannel {
    Push,
    Sms,
    Email,
    Whatsapp,
    InApp,
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Push => write!(f, "push"),
            Self::Sms => write!(f, "sms"),
            Self::Email => write!(f, "email"),
            Self::Whatsapp => write!(f, "whatsapp"),
            Self::InApp => write!(f, "in_app"),
        }
    }
}

impl FromStr for NotificationChannel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "push" => Ok(Self::Push),
            "sms" => Ok(Self::Sms),
            "email" => Ok(Self::Email),
            "whatsapp" => Ok(Self::Whatsapp),
            "in_app" => Ok(Self::InApp),
            _ => Err(format!("Unknown NotificationChannel variant: {}", s)),
        }
    }
}

impl Default for NotificationChannel {
    fn default() -> Self {
        Self::Push
    }
}
