use serde::Deserialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::terminal::{TerminalKind, TerminalSession};
use crate::error::{AppError, CommandError};

#[tauri::command]
pub async fn list_terminal_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<TerminalSession>, CommandError> {
    Ok(state.terminals.list())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateTerminalSessionInput {
    pub kind: TerminalKind,
    pub rows: u16,
    pub cols: u16,
}

#[tauri::command]
pub async fn create_terminal_session(
    input: CreateTerminalSessionInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TerminalSession, CommandError> {
    state
        .terminals
        .create(&app, input.kind, input.rows, input.cols)
        .map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WriteTerminalSessionInput {
    pub session_id: String,
    pub data: String,
}

#[tauri::command]
pub async fn write_terminal_session(
    input: WriteTerminalSessionInput,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .terminals
        .write(parse_uuid(&input.session_id)?, &input.data)
        .map_err(CommandError::from)?;
    Ok(true)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResizeTerminalSessionInput {
    pub session_id: String,
    pub rows: u16,
    pub cols: u16,
}

#[tauri::command]
pub async fn resize_terminal_session(
    input: ResizeTerminalSessionInput,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .terminals
        .resize(parse_uuid(&input.session_id)?, input.rows, input.cols)
        .map_err(CommandError::from)?;
    Ok(true)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloseTerminalSessionInput {
    pub session_id: String,
}

#[tauri::command]
pub async fn close_terminal_session(
    input: CloseTerminalSessionInput,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .terminals
        .close(parse_uuid(&input.session_id)?)
        .map_err(CommandError::from)?;
    Ok(true)
}

fn parse_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))
}
