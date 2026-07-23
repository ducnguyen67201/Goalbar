use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::adapters::agent::AgentRegistry;
use crate::app_state::AppState;
use crate::conductor::prompt::LEARNING_PROMPT;
use crate::conductor::task::structured_task;
use crate::domain::metrics::GrowthScore;
use crate::error::CommandError;
use crate::services::learning::WeeklyLearningDraft;
use crate::services::scoring::ScoringService;

#[tauri::command]
pub async fn get_growth_overview(state: State<'_, AppState>) -> Result<GrowthScore, CommandError> {
    ScoringService::new(state.database.pool().clone())
        .current()
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
    let task = structured_task::<WeeklyLearningDraft>(
        "weekly_learning",
        LEARNING_PROMPT,
        serde_json::json!({"growthScore": score}),
    );
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
