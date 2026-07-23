use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::adapters::agent::app_server::{CodexChatState, CodexChatTurnResult};
use crate::adapters::agent::{AgentResult, AgentStatus};
use crate::app_state::AppState;
use crate::error::{AppError, CommandError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunAgentInput {
    pub provider: String,
    pub task_kind: String,
    pub prompt: String,
    pub context: Value,
    pub output_schema: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendCodexChatInput {
    pub message: String,
    pub active_tab_id: Option<String>,
}

#[tauri::command]
pub async fn detect_agents(state: State<'_, AppState>) -> Result<Vec<AgentStatus>, CommandError> {
    Ok(state.agents.statuses().await)
}

#[tauri::command]
pub async fn run_agent_task(
    input: RunAgentInput,
    state: State<'_, AppState>,
) -> Result<AgentResult, CommandError> {
    let provider = crate::adapters::agent::AgentRegistry::parse_provider(&input.provider)
        .map_err(CommandError::from)?;
    let task = crate::adapters::agent::StructuredAgentTask {
        task_kind: input.task_kind,
        prompt: input.prompt,
        context: input.context,
        output_schema: input.output_schema,
        timeout_seconds: 120,
    };
    let (_value, result) = state
        .conductor
        .run::<Value>(Uuid::new_v4(), provider, task)
        .await
        .map_err(CommandError::from)?;
    Ok(result)
}

#[tauri::command]
pub async fn send_codex_chat_message(
    input: SendCodexChatInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CodexChatTurnResult, CommandError> {
    let active_tab_id = input
        .active_tab_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))?;
    state
        .codex_chat
        .send_message(&app, &input.message, active_tab_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_codex_chat_state(
    state: State<'_, AppState>,
) -> Result<CodexChatState, CommandError> {
    Ok(state.codex_chat.current_state().await)
}

#[tauri::command]
pub async fn interrupt_codex_chat(state: State<'_, AppState>) -> Result<bool, CommandError> {
    state
        .codex_chat
        .interrupt()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn new_codex_chat(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    state
        .codex_chat
        .new_thread(&app)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn cancel_job(job_id: String, state: State<'_, AppState>) -> Result<bool, CommandError> {
    let job_id = Uuid::parse_str(&job_id)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))?;
    let in_memory = state.conductor.cancel(job_id);
    let persisted = crate::db::repositories::job::JobRepository::new(state.database.pool().clone())
        .cancel(job_id)
        .await
        .is_ok();
    Ok(in_memory || persisted)
}
