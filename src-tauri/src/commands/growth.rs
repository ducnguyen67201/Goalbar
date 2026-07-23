use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::adapters::agent::AgentRegistry;
use crate::app_state::AppState;
use crate::conductor::context::ContextAssembler;
use crate::conductor::prompt::LEARNING_PROMPT;
use crate::conductor::task::structured_task;
use crate::db::repositories::founder::FounderRepository;
use crate::db::repositories::growth_loop::GrowthLoopRepository;
use crate::domain::approval::Approval;
use crate::domain::growth_loop::{
    ApproveGrowthActionInput, GrowthAction, GrowthLoopOverview, ProposeGrowthActionInput,
    RecordGrowthActionExecutionInput, RecordGrowthActionMetricInput, RecordGrowthLearningInput,
    ReviseGrowthActionInput, TrackedGrowthLearning,
};
use crate::domain::metrics::GrowthScore;
use crate::error::{AppError, CommandError};
use crate::services::history::HistoryContextService;
use crate::services::learning::WeeklyLearningDraft;
use crate::services::scoring::ScoringService;

#[tauri::command]
pub async fn get_growth_overview(state: State<'_, AppState>) -> Result<GrowthScore, CommandError> {
    ScoringService::new(state.database.pool().clone())
        .current()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_growth_loop_overview(
    state: State<'_, AppState>,
) -> Result<GrowthLoopOverview, CommandError> {
    let founder_id = founder_id(&state).await?;
    GrowthLoopRepository::new(state.database.pool().clone())
        .overview(founder_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn propose_growth_action(
    input: ProposeGrowthActionInput,
    state: State<'_, AppState>,
) -> Result<GrowthAction, CommandError> {
    let founder_id = founder_id(&state).await?;
    GrowthLoopRepository::new(state.database.pool().clone())
        .propose(founder_id, input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn revise_growth_action(
    input: ReviseGrowthActionInput,
    state: State<'_, AppState>,
) -> Result<GrowthAction, CommandError> {
    let founder_id = founder_id(&state).await?;
    GrowthLoopRepository::new(state.database.pool().clone())
        .revise(founder_id, input.action_id, &input.exact_payload)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn approve_growth_action(
    input: ApproveGrowthActionInput,
    state: State<'_, AppState>,
) -> Result<Approval, CommandError> {
    let founder_id = founder_id(&state).await?;
    GrowthLoopRepository::new(state.database.pool().clone())
        .approve(founder_id, input.action_id, &input.exact_payload)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn record_growth_action_execution(
    input: RecordGrowthActionExecutionInput,
    state: State<'_, AppState>,
) -> Result<GrowthAction, CommandError> {
    let founder_id = founder_id(&state).await?;
    GrowthLoopRepository::new(state.database.pool().clone())
        .record_execution(founder_id, input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn record_growth_action_metric(
    input: RecordGrowthActionMetricInput,
    state: State<'_, AppState>,
) -> Result<GrowthAction, CommandError> {
    let founder_id = founder_id(&state).await?;
    GrowthLoopRepository::new(state.database.pool().clone())
        .record_metric(founder_id, input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn record_growth_action_learning(
    input: RecordGrowthLearningInput,
    state: State<'_, AppState>,
) -> Result<TrackedGrowthLearning, CommandError> {
    let founder_id = founder_id(&state).await?;
    GrowthLoopRepository::new(state.database.pool().clone())
        .record_learning(founder_id, input)
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WeeklyReviewInput {
    pub provider: String,
}

#[tauri::command]
pub async fn generate_weekly_review(
    input: WeeklyReviewInput,
    state: State<'_, AppState>,
) -> Result<WeeklyLearningDraft, CommandError> {
    let provider = AgentRegistry::parse_provider(&input.provider).map_err(CommandError::from)?;
    let score = ScoringService::new(state.database.pool().clone())
        .current()
        .await
        .map_err(CommandError::from)?;
    let history = HistoryContextService::new(state.database.pool().clone())
        .learning_evidence(30, 8_000)
        .await
        .map_err(CommandError::from)?;
    let growth_loop = GrowthLoopRepository::new(state.database.pool().clone())
        .overview(founder_id(&state).await?)
        .await
        .map_err(CommandError::from)?;
    let context = ContextAssembler::new(24_000).assemble([
        (
            "growthScore".to_owned(),
            serde_json::to_value(score)
                .map_err(AppError::from)
                .map_err(CommandError::from)?,
        ),
        (
            "controlledGrowthLoop".to_owned(),
            serde_json::to_value(growth_loop)
                .map_err(AppError::from)
                .map_err(CommandError::from)?,
        ),
        ("historyEvidence".to_owned(), history),
    ]);
    let task = structured_task::<WeeklyLearningDraft>("weekly_learning", LEARNING_PROMPT, context);
    state
        .conductor
        .run::<WeeklyLearningDraft>(Uuid::new_v4(), provider, task)
        .await
        .map(|(learning, _)| learning)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn accept_learning(
    input: WeeklyLearningDraft,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let founder =
        crate::db::repositories::founder::FounderRepository::new(state.database.pool().clone())
            .latest()
            .await
            .map_err(CommandError::from)?
            .ok_or_else(|| {
                CommandError::from(crate::error::AppError::Validation(
                    "complete founder onboarding first".to_owned(),
                ))
            })?;
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO learnings (id, founder_id, summary, evidence_json, confidence, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'accepted', ?, ?)")
        .bind(id.to_string())
        .bind(founder.id.to_string())
        .bind(input.learning)
        .bind(serde_json::json!({"observation": input.observation, "counterEvidence": input.counter_evidence, "nextExperiment": input.next_experiment}).to_string())
        .bind(input.confidence.clamp(0.0, 1.0))
        .bind(&now)
        .bind(&now)
        .execute(state.database.pool())
        .await
        .map_err(crate::error::AppError::from)
        .map_err(CommandError::from)?;
    Ok(id.to_string())
}

async fn founder_id(state: &AppState) -> Result<Uuid, CommandError> {
    FounderRepository::new(state.database.pool().clone())
        .latest()
        .await
        .map_err(CommandError::from)?
        .map(|founder| founder.id)
        .ok_or_else(|| {
            CommandError::from(AppError::Validation(
                "complete founder onboarding first".to_owned(),
            ))
        })
}
