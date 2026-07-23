use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::adapters::agent::AgentRegistry;
use crate::app_state::AppState;
use crate::conductor::context::ContextAssembler;
use crate::conductor::prompt::ICP_PROMPT;
use crate::conductor::task::structured_task;
use crate::db::repositories::founder::FounderRepository;
use crate::db::repositories::icp::IcpRepository;
use crate::domain::founder::{FounderProfile, FounderProfileInput, VoiceProfileInput};
use crate::domain::icp::{IcpHypotheses, StoredIcpHypothesis};
use crate::error::{AppError, CommandError};
use crate::services::onboarding::OnboardingService;

#[tauri::command]
pub async fn save_founder_profile(
    input: FounderProfileInput,
    state: State<'_, AppState>,
) -> Result<FounderProfile, CommandError> {
    OnboardingService::new(state.database.pool().clone())
        .save_founder(input)
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveVoiceInput {
    pub founder_id: String,
    pub voice: VoiceProfileInput,
}

#[tauri::command]
pub async fn save_voice_profile(
    input: SaveVoiceInput,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let founder_id = parse_uuid(&input.founder_id)?;
    OnboardingService::new(state.database.pool().clone())
        .save_voice(founder_id, input.voice)
        .await
        .map(|id| id.to_string())
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateIcpInput {
    pub provider: String,
}

#[tauri::command]
pub async fn generate_icp_hypotheses(
    input: GenerateIcpInput,
    state: State<'_, AppState>,
) -> Result<IcpHypotheses, CommandError> {
    let founder = FounderRepository::new(state.database.pool().clone())
        .latest()
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| {
            CommandError::from(AppError::Validation(
                "complete founder onboarding first".to_owned(),
            ))
        })?;
    let provider = AgentRegistry::parse_provider(&input.provider).map_err(CommandError::from)?;
    let context = ContextAssembler::new(20_000).assemble([(
        "founder".to_owned(),
        serde_json::to_value(&founder)
            .map_err(AppError::from)
            .map_err(CommandError::from)?,
    )]);
    let task = structured_task::<IcpHypotheses>("icp_hypotheses", ICP_PROMPT, context);
    let (hypotheses, _result) = state
        .conductor
        .run::<IcpHypotheses>(Uuid::new_v4(), provider, task)
        .await
        .map_err(CommandError::from)?;
    let repository = IcpRepository::new(state.database.pool().clone());
    for hypothesis in hypotheses.hypotheses.iter().cloned() {
        repository
            .save_hypothesis(founder.id, hypothesis)
            .await
            .map_err(CommandError::from)?;
    }
    Ok(hypotheses)
}

#[tauri::command]
pub async fn list_icp_hypotheses(
    state: State<'_, AppState>,
) -> Result<Vec<StoredIcpHypothesis>, CommandError> {
    let founder = FounderRepository::new(state.database.pool().clone())
        .latest()
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| {
            CommandError::from(AppError::Validation(
                "complete founder onboarding first".to_owned(),
            ))
        })?;
    IcpRepository::new(state.database.pool().clone())
        .list_for_founder(founder.id)
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptIcpInput {
    pub hypothesis_id: String,
}

#[tauri::command]
pub async fn accept_icp_hypothesis(
    input: AcceptIcpInput,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let founder = FounderRepository::new(state.database.pool().clone())
        .latest()
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| {
            CommandError::from(AppError::Validation(
                "complete founder onboarding first".to_owned(),
            ))
        })?;
    let hypothesis_id = parse_uuid(&input.hypothesis_id)?;
    IcpRepository::new(state.database.pool().clone())
        .accept(founder.id, hypothesis_id)
        .await
        .map_err(CommandError::from)?;
    let hypothesis_id_text = hypothesis_id.to_string();
    crate::db::repositories::audit::AuditRepository::new(state.database.pool().clone())
        .record(
            "icp_hypothesis_accepted",
            Some("icp_hypothesis"),
            Some(&hypothesis_id_text),
            &serde_json::json!({"founderId": founder.id}),
        )
        .await
        .map(|_| ())
        .map_err(CommandError::from)
}

fn parse_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))
}
