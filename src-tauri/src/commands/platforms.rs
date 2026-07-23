use tauri::State;
use uuid::Uuid;

use crate::adapters::platform::oauth::{
    BeginOAuthRequest, BeginOAuthResponse, OAuthStatusResponse,
};
use crate::app_state::AppState;
use crate::db::repositories::platform::{ConnectedAccount, PlatformRepository};
use crate::error::{AppError, CommandError};
use crate::services::publishing::PublishingService;
use crate::services::sync::SyncService;

#[tauri::command]
pub async fn list_platform_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectedAccount>, CommandError> {
    PlatformRepository::new(state.database.pool().clone())
        .list()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn begin_platform_oauth(
    input: BeginOAuthRequest,
    state: State<'_, AppState>,
) -> Result<BeginOAuthResponse, CommandError> {
    state
        .oauth
        .begin(input, true)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_oauth_status(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<OAuthStatusResponse, CommandError> {
    state
        .oauth
        .status(parse_uuid(&session_id)?)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn complete_platform_oauth(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<ConnectedAccount, CommandError> {
    let completed = state
        .oauth
        .complete(parse_uuid(&session_id)?)
        .await
        .map_err(CommandError::from)?;
    let id = Uuid::new_v4();
    let secret_ref = format!("platform/{}/{id}", completed.request.platform.as_str());
    let secret = PublishingService::token_secret(
        completed.token.access_token,
        completed.token.refresh_token,
    )
    .map_err(CommandError::from)?;
    state
        .secrets
        .save(&secret_ref, &secret)
        .map_err(CommandError::from)?;
    let capabilities = state
        .platforms
        .get(completed.request.platform)
        .capabilities(&completed.token.scopes);
    let account = ConnectedAccount {
        id,
        platform: completed.request.platform,
        client_id: completed.request.client_id,
        remote_account_id: completed.request.remote_account_id,
        display_name: completed.request.display_name,
        secret_ref: secret_ref.clone(),
        scopes: completed.token.scopes,
        capabilities,
        token_expires_at: completed.token.expires_at,
        status: "connected".to_owned(),
    };
    if let Err(error) = PlatformRepository::new(state.database.pool().clone())
        .upsert(&account)
        .await
    {
        let _ = state.secrets.delete(&secret_ref);
        return Err(CommandError::from(error));
    }
    Ok(account)
}

#[tauri::command]
pub async fn disconnect_platform(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    let repository = PlatformRepository::new(state.database.pool().clone());
    let account = repository
        .get(parse_uuid(&account_id)?)
        .await
        .map_err(CommandError::from)?;
    let removed = state
        .secrets
        .delete(&account.secret_ref)
        .map_err(CommandError::from)?;
    repository
        .mark_revoked(account.id)
        .await
        .map_err(CommandError::from)?;
    Ok(removed)
}

#[tauri::command]
pub async fn sync_platform_now(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<crate::adapters::platform::SyncPage, CommandError> {
    SyncService::new(state.database.pool().clone(), state.platforms.clone())
        .sync_account(state.secrets.as_ref(), parse_uuid(&account_id)?)
        .await
        .map_err(CommandError::from)
}

fn parse_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::from(AppError::Validation(error.to_string())))
}
