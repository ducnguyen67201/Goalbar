use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("operation is not supported: {0}")]
    Unsupported(String),
    #[error("authentication is required: {0}")]
    Authentication(String),
    #[error("permission is required: {0}")]
    Permission(String),
    #[error("operation timed out: {0}")]
    Timeout(String),
    #[error("operation was cancelled")]
    Cancelled,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("credential store error: {0}")]
    Credential(String),
    #[error("agent error: {0}")]
    Agent(String),
    #[error("platform error: {0}")]
    Platform(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
    pub recovery: Option<String>,
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        let (code, recovery) = match &error {
            AppError::Validation(_) => ("validation", Some("Review the highlighted fields.")),
            AppError::NotFound(_) => ("not_found", Some("Refresh the local data and try again.")),
            AppError::Unsupported(_) => ("unsupported", None),
            AppError::Authentication(_) => {
                ("authentication_required", Some("Reconnect the account."))
            }
            AppError::Permission(_) => ("permission_required", Some("Review the granted scopes.")),
            AppError::Timeout(_) => ("timeout", Some("Try again when the provider is available.")),
            AppError::Cancelled => ("cancelled", None),
            AppError::Database(_) => (
                "database",
                Some("Back up the data directory before retrying."),
            ),
            AppError::Credential(_) => (
                "credential_store",
                Some("Unlock the OS keyring and try again."),
            ),
            AppError::Agent(_) => (
                "agent",
                Some("Check the selected CLI and its login status."),
            ),
            AppError::Platform(_) => ("platform", Some("Check the platform status and retry.")),
            AppError::Io(_) => ("io", None),
            AppError::Serialization(_) => ("invalid_data", None),
            AppError::Internal(_) => ("internal", None),
        };
        Self {
            code,
            message: error.to_string(),
            recovery: recovery.map(str::to_owned),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(format!("{error:#}"))
    }
}
