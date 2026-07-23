use tauri::State;

use crate::app_state::AppState;
use crate::error::CommandError;
use crate::services::data::DataArtifact;
use crate::validation::allowlisted_external_url;

#[tauri::command]
pub async fn check_keyring(state: State<'_, AppState>) -> Result<bool, CommandError> {
    let probe = format!("health/{}", uuid::Uuid::new_v4());
    let secret = secrecy::SecretString::from("probe".to_owned());
    state
        .secrets
        .save(&probe, &secret)
        .map_err(CommandError::from)?;
    let loaded = state
        .secrets
        .load(&probe)
        .map_err(CommandError::from)?
        .is_some();
    let _ = state.secrets.delete(&probe);
    Ok(loaded)
}

#[tauri::command]
pub async fn open_remote_url(url: String) -> Result<(), CommandError> {
    let url = allowlisted_external_url(&url).map_err(CommandError::from)?;
    open::that(url.as_str()).map_err(|error| {
        CommandError::from(crate::error::AppError::Io(std::io::Error::other(
            error.to_string(),
        )))
    })
}

#[tauri::command]
pub async fn export_local_data(state: State<'_, AppState>) -> Result<DataArtifact, CommandError> {
    crate::services::data::export_json(&state)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn backup_local_database(
    state: State<'_, AppState>,
) -> Result<DataArtifact, CommandError> {
    crate::services::data::backup_database(&state)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn factory_reset_local_data(
    confirmation: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    crate::services::data::factory_reset(&state, &confirmation)
        .await
        .map_err(CommandError::from)
}
