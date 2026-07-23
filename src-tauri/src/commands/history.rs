use serde::Deserialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::history::{
    HistoryImportResult, HistoryOverview, HistoryPreview, HistorySelection,
};
use crate::error::{AppError, CommandError};
use crate::services::history::HistoryImportService;

#[tauri::command]
pub async fn choose_history_archive(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<HistorySelection>, CommandError> {
    state
        .history_selections
        .choose(&app)
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistorySelectionInput {
    pub selection_id: String,
}

#[tauri::command]
pub async fn preview_history_archive(
    input: HistorySelectionInput,
    state: State<'_, AppState>,
) -> Result<HistoryPreview, CommandError> {
    HistoryImportService::new(
        state.history_selections.clone(),
        state.database.pool().clone(),
    )
    .preview(parse_uuid(&input.selection_id)?)
    .await
    .map_err(CommandError::from)
}

#[tauri::command]
pub async fn import_history_archive(
    input: HistorySelectionInput,
    state: State<'_, AppState>,
) -> Result<HistoryImportResult, CommandError> {
    HistoryImportService::new(
        state.history_selections.clone(),
        state.database.pool().clone(),
    )
    .commit(parse_uuid(&input.selection_id)?)
    .await
    .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_history_overview(
    state: State<'_, AppState>,
) -> Result<HistoryOverview, CommandError> {
    HistoryImportService::new(
        state.history_selections.clone(),
        state.database.pool().clone(),
    )
    .overview()
    .await
    .map_err(CommandError::from)
}

fn parse_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))
}
