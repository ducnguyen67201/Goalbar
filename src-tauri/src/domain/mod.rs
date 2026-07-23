pub mod approval;
pub mod browser;
pub mod content;
pub mod experiment;
pub mod founder;
pub mod history;
pub mod icp;
pub mod job;
pub mod metrics;
pub mod relationship;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    X,
    Reddit,
    Linkedin,
}

impl Platform {
    pub const ALL: [Self; 3] = [Self::X, Self::Reddit, Self::Linkedin];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Reddit => "reddit",
            Self::Linkedin => "linkedin",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "x" => Ok(Self::X),
            "reddit" => Ok(Self::Reddit),
            "linkedin" => Ok(Self::Linkedin),
            _ => Err(AppError::Validation(format!("unknown platform: {value}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Supported,
    Unsupported,
    ApprovalPending,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::{Platform, Platform::*};

    #[test]
    fn platform_round_trips() {
        for platform in Platform::ALL {
            assert_eq!(Platform::parse(platform.as_str()).ok(), Some(platform));
        }
        assert!(Platform::parse("threads").is_err());
        assert_eq!(Platform::ALL, [X, Reddit, Linkedin]);
    }
}
