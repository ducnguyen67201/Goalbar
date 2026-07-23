use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Platform;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentIdeaInput {
    pub title: String,
    pub insight: String,
    pub hypothesis: String,
    pub success_metric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContentIdea {
    pub id: Uuid,
    pub founder_id: Uuid,
    #[serde(flatten)]
    pub input: ContentIdeaInput,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentVariantDraft {
    pub platform: Platform,
    pub body: String,
    pub rationale: String,
    pub call_to_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedContentSet {
    pub variants: Vec<ContentVariantDraft>,
    pub video_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoredContentVariant {
    pub id: Uuid,
    pub platform: Platform,
    pub revision: u32,
    pub body: String,
    pub status: String,
}

impl GeneratedContentSet {
    pub fn has_all_platforms(&self) -> bool {
        Platform::ALL.iter().all(|platform| {
            self.variants
                .iter()
                .any(|variant| variant.platform == *platform)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentVariantDraft, GeneratedContentSet};
    use crate::domain::Platform;

    #[test]
    fn content_set_requires_each_platform_for_complete_state() {
        let variants = Platform::ALL
            .iter()
            .map(|platform| ContentVariantDraft {
                platform: *platform,
                body: "Draft".to_owned(),
                rationale: "Native format".to_owned(),
                call_to_action: None,
            })
            .collect();
        assert!(
            GeneratedContentSet {
                variants,
                video_script: None
            }
            .has_all_platforms()
        );
    }
}
