use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppResult;
use crate::validation::require_non_empty;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FounderProfileInput {
    pub name: String,
    pub product_name: String,
    pub offer: String,
    pub expertise: String,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub boundaries: Vec<String>,
}

impl FounderProfileInput {
    pub fn validate(mut self) -> AppResult<Self> {
        self.name = require_non_empty(&self.name, "name", 120)?;
        self.product_name = require_non_empty(&self.product_name, "product name", 160)?;
        self.offer = require_non_empty(&self.offer, "offer", 2_000)?;
        self.expertise = require_non_empty(&self.expertise, "expertise", 4_000)?;
        self.goals.retain(|goal| !goal.trim().is_empty());
        self.boundaries.retain(|rule| !rule.trim().is_empty());
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FounderProfile {
    pub id: Uuid,
    #[serde(flatten)]
    pub input: FounderProfileInput,
    pub onboarding_completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FounderProfile {
    pub fn new(input: FounderProfileInput) -> AppResult<Self> {
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            input: input.validate()?,
            onboarding_completed: true,
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoiceProfileInput {
    pub traits: Vec<String>,
    pub do_rules: Vec<String>,
    pub dont_rules: Vec<String>,
    pub example: String,
}

#[cfg(test)]
mod tests {
    use super::{FounderProfile, FounderProfileInput};

    #[test]
    fn founder_requires_core_context() {
        let invalid = FounderProfileInput {
            name: String::new(),
            product_name: "Lab".to_owned(),
            offer: "Growth clarity".to_owned(),
            expertise: "Product".to_owned(),
            goals: vec![],
            boundaries: vec![],
        };
        assert!(FounderProfile::new(invalid).is_err());
    }
}
