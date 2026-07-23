use serde::Deserialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::browser::extraction;
use crate::db::repositories::research::ResearchRepository;
use crate::domain::browser::{
    BrowserBounds, BrowserCapturePreview, BrowserObservation, BrowserResearchTrace,
    BrowserRunLimits, BrowserRunProgress, BrowserTab, ResearchFindingStatus, StoredResearchFinding,
};
use crate::domain::history::{ActivityOwnership, HistoryImportResult};
use crate::error::{AppError, CommandError};
use crate::services::browser::{BrowserConductorService, CaptureMode};
use crate::validation::{validate_browser_bounds, validate_browser_run_limits};

#[tauri::command]
pub async fn list_browser_tabs(
    state: State<'_, AppState>,
) -> Result<Vec<BrowserTab>, CommandError> {
    Ok(state.browser.tabs())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateBrowserTabInput {
    pub url: String,
    pub bounds: BrowserBounds,
}

#[tauri::command]
pub async fn create_browser_tab(
    input: CreateBrowserTabInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BrowserTab, CommandError> {
    validate_browser_bounds(input.bounds).map_err(CommandError::from)?;
    state
        .browser
        .create_tab(&app, &input.url, input.bounds)
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserTabInput {
    pub tab_id: String,
}

#[tauri::command]
pub async fn activate_browser_tab(
    input: BrowserTabInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BrowserTab, CommandError> {
    state
        .browser
        .activate(&app, parse_uuid(&input.tab_id)?)
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserBoundsInput {
    pub bounds: BrowserBounds,
}

#[tauri::command]
pub async fn update_browser_bounds(
    input: BrowserBoundsInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    validate_browser_bounds(input.bounds).map_err(CommandError::from)?;
    state
        .browser
        .set_bounds(&app, input.bounds)
        .map_err(CommandError::from)?;
    Ok(true)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NavigateBrowserInput {
    pub tab_id: String,
    pub url: String,
}

#[tauri::command]
pub async fn navigate_browser_tab(
    input: NavigateBrowserInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BrowserTab, CommandError> {
    state
        .browser
        .navigate(&app, parse_uuid(&input.tab_id)?, &input.url)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn browser_go_back(
    input: BrowserTabInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .browser
        .history(&app, parse_uuid(&input.tab_id)?, -1)
        .map_err(CommandError::from)?;
    Ok(true)
}

#[tauri::command]
pub async fn browser_go_forward(
    input: BrowserTabInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .browser
        .history(&app, parse_uuid(&input.tab_id)?, 1)
        .map_err(CommandError::from)?;
    Ok(true)
}

#[tauri::command]
pub async fn reload_browser_tab(
    input: BrowserTabInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .browser
        .reload(&app, parse_uuid(&input.tab_id)?)
        .map_err(CommandError::from)?;
    Ok(true)
}

#[tauri::command]
pub async fn close_browser_tab(
    input: BrowserTabInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .browser
        .close(&app, parse_uuid(&input.tab_id)?)
        .map_err(CommandError::from)?;
    Ok(true)
}

#[tauri::command]
pub async fn hide_browser_views(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state.browser.hide_all(&app).map_err(CommandError::from)?;
    Ok(true)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClearBrowserDataInput {
    pub confirmation: String,
}

#[tauri::command]
pub async fn clear_browser_data(
    input: ClearBrowserDataInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    if input.confirmation != "CLEAR BROWSER DATA" {
        return Err(CommandError::from(AppError::Validation(
            "type CLEAR BROWSER DATA to confirm".to_owned(),
        )));
    }
    state.browser.clear_data(&app).map_err(CommandError::from)?;
    Ok(true)
}

#[tauri::command]
pub async fn get_browser_panel_width(
    state: State<'_, AppState>,
) -> Result<Option<f64>, CommandError> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value_json FROM app_settings WHERE key = ?")
            .bind("browser_panel_width")
            .fetch_optional(state.database.pool())
            .await
            .map_err(AppError::from)
            .map_err(CommandError::from)?;
    value
        .map(|value| serde_json::from_str::<f64>(&value))
        .transpose()
        .map_err(AppError::from)
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserPanelWidthInput {
    pub width: f64,
}

#[tauri::command]
pub async fn set_browser_panel_width(
    input: BrowserPanelWidthInput,
    state: State<'_, AppState>,
) -> Result<f64, CommandError> {
    if !input.width.is_finite() || !(280.0..=480.0).contains(&input.width) {
        return Err(CommandError::from(AppError::Validation(
            "browser panel width must be between 280 and 480".to_owned(),
        )));
    }
    sqlx::query("INSERT INTO app_settings (key, value_json, updated_at) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at")
        .bind("browser_panel_width")
        .bind(serde_json::to_string(&input.width).map_err(AppError::from).map_err(CommandError::from)?)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(state.database.pool())
        .await
        .map_err(AppError::from)
        .map_err(CommandError::from)?;
    Ok(input.width)
}

#[tauri::command]
pub async fn observe_browser_tab(
    input: BrowserTabInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BrowserObservation, CommandError> {
    extraction::observe(&app, &state.browser, parse_uuid(&input.tab_id)?)
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserCaptureInput {
    pub tab_id: String,
    pub mode: String,
    pub ownership: String,
}

#[tauri::command]
pub async fn preview_browser_capture(
    input: BrowserCaptureInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BrowserCapturePreview, CommandError> {
    BrowserConductorService::new(
        state.browser.clone(),
        state.database.pool().clone(),
        state.conductor.clone(),
    )
    .preview_capture(
        &app,
        parse_uuid(&input.tab_id)?,
        CaptureMode::parse(&input.mode).map_err(CommandError::from)?,
        parse_ownership(&input.ownership)?,
    )
    .await
    .map_err(CommandError::from)
}

#[tauri::command]
pub async fn commit_browser_capture(
    input: BrowserCaptureInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<HistoryImportResult, CommandError> {
    BrowserConductorService::new(
        state.browser.clone(),
        state.database.pool().clone(),
        state.conductor.clone(),
    )
    .commit_capture(
        &app,
        parse_uuid(&input.tab_id)?,
        CaptureMode::parse(&input.mode).map_err(CommandError::from)?,
        parse_ownership(&input.ownership)?,
    )
    .await
    .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartBrowserCollectionInput {
    pub tab_id: String,
    pub objective: String,
    pub limits: BrowserRunLimits,
    pub ownership: String,
    pub provider: Option<String>,
}

#[tauri::command]
pub async fn start_browser_collection(
    input: StartBrowserCollectionInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BrowserRunProgress, CommandError> {
    let objective = crate::validation::require_non_empty(&input.objective, "objective", 1_000)
        .map_err(CommandError::from)?;
    validate_browser_run_limits(&input.limits).map_err(CommandError::from)?;
    BrowserConductorService::new(
        state.browser.clone(),
        state.database.pool().clone(),
        state.conductor.clone(),
    )
    .collect(
        &app,
        parse_uuid(&input.tab_id)?,
        &objective,
        input.limits,
        parse_ownership(&input.ownership)?,
        input.provider.as_deref(),
    )
    .await
    .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelBrowserCollectionInput {
    pub run_id: String,
}

#[tauri::command]
pub async fn cancel_browser_collection(
    input: CancelBrowserCollectionInput,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    let run_id = parse_uuid(&input.run_id)?;
    let browser_cancelled = state.browser.cancel_run(run_id);
    let agent_cancelled = state.conductor.cancel(run_id);
    Ok(browser_cancelled || agent_cancelled)
}

#[tauri::command]
pub async fn list_browser_research_findings(
    input: CancelBrowserCollectionInput,
    state: State<'_, AppState>,
) -> Result<Vec<StoredResearchFinding>, CommandError> {
    ResearchRepository::new(state.database.pool().clone())
        .list_findings(parse_uuid(&input.run_id)?)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_browser_research_trace(
    input: CancelBrowserCollectionInput,
    state: State<'_, AppState>,
) -> Result<Vec<BrowserResearchTrace>, CommandError> {
    ResearchRepository::new(state.database.pool().clone())
        .list_trace(parse_uuid(&input.run_id)?)
        .await
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewBrowserResearchFindingInput {
    pub finding_id: String,
    pub status: String,
}

#[tauri::command]
pub async fn review_browser_research_finding(
    input: ReviewBrowserResearchFindingInput,
    state: State<'_, AppState>,
) -> Result<StoredResearchFinding, CommandError> {
    let status = match input.status.as_str() {
        "accepted" => ResearchFindingStatus::Accepted,
        "rejected" => ResearchFindingStatus::Rejected,
        _ => {
            return Err(CommandError::from(AppError::Validation(
                "finding review status must be accepted or rejected".to_owned(),
            )));
        }
    };
    ResearchRepository::new(state.database.pool().clone())
        .review(parse_uuid(&input.finding_id)?, status)
        .await
        .map_err(CommandError::from)
}

fn parse_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))
}

fn parse_ownership(value: &str) -> Result<ActivityOwnership, CommandError> {
    match value {
        "own" => Ok(ActivityOwnership::Own),
        "reference" => Ok(ActivityOwnership::Reference),
        _ => Err(CommandError::from(AppError::Validation(format!(
            "unknown ownership: {value}"
        )))),
    }
}
