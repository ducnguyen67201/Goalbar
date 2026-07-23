use std::time::Duration;

use uuid::Uuid;

use crate::app_state::AppState;
use crate::db::repositories::job::{JobRecord, JobRepository};
use crate::error::{AppError, AppResult};
use crate::services::sync::SyncService;

pub fn start(state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = process_one(&state).await {
                tracing::warn!(error = %error, "background job pass failed");
            }
        }
    });
}

pub async fn process_one(state: &AppState) -> AppResult<bool> {
    let repository = JobRepository::new(state.database.pool().clone());
    let Some(job) = repository.lease_next().await? else {
        return Ok(false);
    };
    let result = execute(state, &job).await;
    match result {
        Ok(value) => repository.finish(job.id, &value).await?,
        Err(error) => repository.fail(&job, error_code(&error)).await?,
    }
    Ok(true)
}

async fn execute(state: &AppState, job: &JobRecord) -> AppResult<serde_json::Value> {
    match job.kind.as_str() {
        "sync_account" => {
            let account_id = job
                .payload
                .get("accountId")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| AppError::Validation("sync job is missing accountId".to_owned()))?;
            let page = SyncService::new(state.database.pool().clone(), state.platforms.clone())
                .sync_account(
                    state.secrets.as_ref(),
                    Uuid::parse_str(account_id)
                        .map_err(|error| AppError::Validation(error.to_string()))?,
                )
                .await?;
            Ok(serde_json::json!({"items": page.items.len(), "nextCursor": page.next_cursor}))
        }
        kind => Err(AppError::Unsupported(format!(
            "unknown background job kind: {kind}"
        ))),
    }
}

fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Authentication(_) => "authentication",
        AppError::Permission(_) => "permission",
        AppError::Timeout(_) => "timeout",
        AppError::Validation(_) => "invalid_job",
        AppError::Unsupported(_) => "unsupported",
        _ => "transient",
    }
}

#[cfg(test)]
mod tests {
    use super::process_one;
    use crate::app_state::AppState;

    #[tokio::test]
    async fn empty_scheduler_pass_is_a_noop() {
        let state = AppState::for_tests().await.expect("state");
        assert!(!process_one(&state).await.expect("scheduler pass"));
    }
}
