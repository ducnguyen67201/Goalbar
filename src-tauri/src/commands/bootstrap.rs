use tauri::State;

use crate::app_state::AppState;
use crate::error::CommandError;
use crate::services::bootstrap::BootstrapState;

#[tauri::command]
pub async fn get_bootstrap_state(
    state: State<'_, AppState>,
) -> Result<BootstrapState, CommandError> {
    crate::services::bootstrap::load(&state)
        .await
        .map_err(CommandError::from)
}
