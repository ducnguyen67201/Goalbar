use std::path::PathBuf;

use crate::error::{AppError, AppResult};

pub const APP_ID: &str = "com.foundergrowthlab.desktop";
pub const SCHEMA_VERSION: u32 = 1;

pub fn resolve_data_dir(default_dir: PathBuf) -> AppResult<PathBuf> {
    match std::env::var_os("TAGLINE_HOME") {
        Some(value) if !value.is_empty() => {
            let path = PathBuf::from(value);
            if path.as_os_str().is_empty() {
                Err(AppError::Validation("data directory is empty".to_owned()))
            } else {
                Ok(path)
            }
        }
        _ => Ok(default_dir),
    }
}
