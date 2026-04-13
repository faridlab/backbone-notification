use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "notification_type", rename_all = "snake_case")]
pub enum NotificationType {
    Order,
    Payment,
    Promo,
    Marketing,
    System,
    Reminder,
    Alert,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order => write!(f, "order"),
            Self::Payment => write!(f, "payment"),
            Self::Promo => write!(f, "promo"),
            Self::Marketing => write!(f, "marketing"),
            Self::System => write!(f, "system"),
            Self::Reminder => write!(f, "reminder"),
            Self::Alert => write!(f, "alert"),
        }
    }
}

impl FromStr for NotificationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "order" => Ok(Self::Order),
            "payment" => Ok(Self::Payment),
            "promo" => Ok(Self::Promo),
            "marketing" => Ok(Self::Marketing),
            "system" => Ok(Self::System),
            "reminder" => Ok(Self::Reminder),
            "alert" => Ok(Self::Alert),
            _ => Err(format!("Unknown NotificationType variant: {}", s)),
        }
    }
}

impl Default for NotificationType {
    fn default() -> Self {
        Self::Order
    }
}
