use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "digest_frequency", rename_all = "snake_case")]
pub enum DigestFrequency {
    Daily,
    Weekly,
}

impl std::fmt::Display for DigestFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Daily => write!(f, "daily"),
            Self::Weekly => write!(f, "weekly"),
        }
    }
}

impl FromStr for DigestFrequency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            _ => Err(format!("Unknown DigestFrequency variant: {}", s)),
        }
    }
}

impl Default for DigestFrequency {
    fn default() -> Self {
        Self::Daily
    }
}
