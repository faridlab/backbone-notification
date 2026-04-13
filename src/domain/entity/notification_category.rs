use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "notification_category", rename_all = "snake_case")]
pub enum NotificationCategory {
    Order,
    Promo,
    Payment,
    Delivery,
    Loyalty,
    System,
    Marketing,
    Reminder,
}

impl std::fmt::Display for NotificationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order => write!(f, "order"),
            Self::Promo => write!(f, "promo"),
            Self::Payment => write!(f, "payment"),
            Self::Delivery => write!(f, "delivery"),
            Self::Loyalty => write!(f, "loyalty"),
            Self::System => write!(f, "system"),
            Self::Marketing => write!(f, "marketing"),
            Self::Reminder => write!(f, "reminder"),
        }
    }
}

impl FromStr for NotificationCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "order" => Ok(Self::Order),
            "promo" => Ok(Self::Promo),
            "payment" => Ok(Self::Payment),
            "delivery" => Ok(Self::Delivery),
            "loyalty" => Ok(Self::Loyalty),
            "system" => Ok(Self::System),
            "marketing" => Ok(Self::Marketing),
            "reminder" => Ok(Self::Reminder),
            _ => Err(format!("Unknown NotificationCategory variant: {}", s)),
        }
    }
}

impl Default for NotificationCategory {
    fn default() -> Self {
        Self::Order
    }
}
