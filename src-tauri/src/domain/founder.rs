use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::validation::require_non_empty;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FounderProfileInput {
    pub name: String,
    pub product_name: String,
    #[serde(default)]
    pub website_url: Option<String>,
    pub offer: String,
    #[serde(default)]
    pub ideal_customer: String,
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
        self.website_url = normalize_website_url(self.website_url)?;
        self.offer = optional_text(&self.offer, "description", 2_000)?;
        if self.website_url.is_none() && self.offer.is_empty() {
            return Err(AppError::Validation(
                "add a landing page or a short description".to_owned(),
            ));
        }
        self.ideal_customer = require_non_empty(&self.ideal_customer, "ideal customer", 2_000)?;
        self.expertise = optional_text(&self.expertise, "expertise", 4_000)?;
        self.goals.retain(|goal| !goal.trim().is_empty());
        self.boundaries.retain(|rule| !rule.trim().is_empty());
        Ok(self)
    }
}

fn normalize_website_url(value: Option<String>) -> AppResult<Option<String>> {
    let Some(value) = value.map(|value| value.trim().to_owned()) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 2_048 {
        return Err(AppError::Validation(
            "website URL must be at most 2048 characters".to_owned(),
        ));
    }
    let url = Url::parse(&value).map_err(|_| {
        AppError::Validation("website URL must be a complete HTTP or HTTPS URL".to_owned())
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::Validation(
            "website URL must be a complete HTTP or HTTPS URL".to_owned(),
        ));
    }
    Ok(Some(url.to_string()))
}

fn optional_text(value: &str, field: &str, max_chars: usize) -> AppResult<String> {
    let value = value.trim();
    if value.chars().count() > max_chars {
        return Err(AppError::Validation(format!(
            "{field} must be at most {max_chars} characters"
        )));
    }
    Ok(value.to_owned())
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
            website_url: None,
            offer: "Growth clarity".to_owned(),
            ideal_customer: "Solo founders".to_owned(),
            expertise: "Product".to_owned(),
            goals: vec![],
            boundaries: vec![],
        };
        assert!(FounderProfile::new(invalid).is_err());
    }

    #[test]
    fn founder_accepts_a_website_instead_of_a_description() {
        let input = FounderProfileInput {
            name: "Duc".to_owned(),
            product_name: "Goalbar".to_owned(),
            website_url: Some("https://goalbar.example".to_owned()),
            offer: String::new(),
            ideal_customer: "Technical solo founders".to_owned(),
            expertise: String::new(),
            goals: vec![],
            boundaries: vec![],
        };
        let profile = FounderProfile::new(input).expect("profile");
        assert_eq!(
            profile.input.website_url.as_deref(),
            Some("https://goalbar.example/")
        );
    }
}
