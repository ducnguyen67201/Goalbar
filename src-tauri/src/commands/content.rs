use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::adapters::agent::AgentRegistry;
use crate::adapters::platform::{PublishRequest, RemoteContent};
use crate::app_state::AppState;
use crate::db::repositories::content::ContentRepository;
use crate::db::repositories::founder::FounderRepository;
use crate::domain::approval::Approval;
use crate::domain::content::{ContentIdeaInput, StoredContentVariant};
use crate::error::{AppError, CommandError};
use crate::services::content::ContentService;
use crate::services::publishing::PublishingService;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateContentInput {
    pub provider: String,
    pub idea: ContentIdeaInput,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    pub idea_id: Uuid,
    pub variants: Vec<StoredContentVariant>,
}

#[tauri::command]
pub async fn generate_content_variants(
    input: GenerateContentInput,
    state: State<'_, AppState>,
) -> Result<GenerateContentResponse, CommandError> {
    let provider = AgentRegistry::parse_provider(&input.provider).map_err(CommandError::from)?;
    let founder = FounderRepository::new(state.database.pool().clone())
        .latest()
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| {
            CommandError::from(AppError::Validation(
                "complete founder onboarding first".to_owned(),
            ))
        })?;
    let (idea_id, _set) =
        ContentService::new(state.conductor.clone(), state.database.pool().clone())
            .generate(provider, &founder, input.idea)
            .await
            .map_err(CommandError::from)?;
    let variants = ContentRepository::new(state.database.pool().clone())
        .variants_for_idea(idea_id)
        .await
        .map_err(CommandError::from)?;
    Ok(GenerateContentResponse { idea_id, variants })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApproveVariantInput {
    pub variant_id: String,
    pub body: String,
}

#[tauri::command]
pub async fn approve_variant(
    input: ApproveVariantInput,
    state: State<'_, AppState>,
) -> Result<Approval, CommandError> {
    ContentRepository::new(state.database.pool().clone())
        .approve_variant(parse_uuid(&input.variant_id)?, &input.body)
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishVariantInput {
    pub account_id: String,
    pub approval_id: String,
    pub variant_id: String,
    pub body: String,
    pub title: Option<String>,
    pub destination: Option<String>,
}

#[tauri::command]
pub async fn publish_variant(
    input: PublishVariantInput,
    state: State<'_, AppState>,
) -> Result<RemoteContent, CommandError> {
    PublishingService::new(state.database.pool().clone(), state.platforms.clone())
        .publish(
            state.secrets.as_ref(),
            parse_uuid(&input.account_id)?,
            parse_uuid(&input.approval_id)?,
            parse_uuid(&input.variant_id)?,
            PublishRequest {
                body: input.body,
                title: input.title,
                destination: input.destination,
                reply_to_id: None,
                idempotency_key: String::new(),
            },
        )
        .await
        .map_err(CommandError::from)
}

fn parse_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))
}
