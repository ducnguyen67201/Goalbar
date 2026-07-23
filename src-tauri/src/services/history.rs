use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::{Duration, Utc};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt as _;
use uuid::Uuid;

use crate::adapters::history::{HistoryParserRegistry, MAX_ARCHIVE_BYTES, file_fingerprint};
use crate::db::repositories::history::{HistoryContextPurpose, HistoryRepository};
use crate::domain::Platform;
use crate::domain::history::{
    HistoryImportResult, HistoryOverview, HistoryPreview, HistorySelection,
};
use crate::error::{AppError, AppResult};

const SELECTION_TTL_MINUTES: i64 = 30;

#[derive(Debug, Clone)]
struct SelectedArchive {
    path: PathBuf,
    display_name: String,
    expires_at: chrono::DateTime<Utc>,
    fingerprint: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HistorySelectionManager {
    selections: Arc<Mutex<HashMap<Uuid, SelectedArchive>>>,
}

impl HistorySelectionManager {
    pub fn choose(&self, app: &AppHandle) -> AppResult<Option<HistorySelection>> {
        self.remove_expired();
        let selected = app
            .dialog()
            .file()
            .add_filter("Account archives", &["zip", "csv", "json", "js"])
            .blocking_pick_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let path = selected
            .into_path()
            .map_err(|_| AppError::Validation("selected archive path is invalid".to_owned()))?;
        let metadata = std::fs::metadata(&path)?;
        if metadata.len() > MAX_ARCHIVE_BYTES {
            return Err(AppError::Validation(
                "archive exceeds the 1 GiB safety limit".to_owned(),
            ));
        }
        let display_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("account-archive")
            .to_owned();
        let container = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("data")
            .to_ascii_lowercase();
        let id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::minutes(SELECTION_TTL_MINUTES);
        self.selections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                id,
                SelectedArchive {
                    path,
                    display_name: display_name.clone(),
                    expires_at,
                    fingerprint: None,
                },
            );
        Ok(Some(HistorySelection {
            selection_id: id,
            display_name,
            size_bytes: metadata.len(),
            container,
            expires_at: expires_at.to_rfc3339(),
        }))
    }

    fn get(&self, id: Uuid) -> AppResult<SelectedArchive> {
        self.remove_expired();
        self.selections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                AppError::NotFound("archive selection expired; choose the file again".to_owned())
            })
    }

    fn set_fingerprint(&self, id: Uuid, fingerprint: String) {
        if let Some(selection) = self
            .selections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(&id)
        {
            selection.fingerprint = Some(fingerprint);
        }
    }

    fn remove(&self, id: Uuid) {
        self.selections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
    }

    fn remove_expired(&self) {
        let now = Utc::now();
        self.selections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, selection| selection.expires_at > now);
    }
}

#[derive(Debug, Clone)]
pub struct HistoryImportService {
    selections: HistorySelectionManager,
    parsers: HistoryParserRegistry,
    repository: HistoryRepository,
}

impl HistoryImportService {
    pub fn new(selections: HistorySelectionManager, pool: sqlx::SqlitePool) -> Self {
        Self {
            selections,
            parsers: HistoryParserRegistry::default(),
            repository: HistoryRepository::new(pool),
        }
    }

    pub async fn preview(&self, selection_id: Uuid) -> AppResult<HistoryPreview> {
        let selection = self.selections.get(selection_id)?;
        let path = selection.path.clone();
        let display_name = selection.display_name.clone();
        let fingerprint = tokio::task::spawn_blocking(move || file_fingerprint(&path))
            .await
            .map_err(|error| AppError::Internal(error.to_string()))??;
        let parser_path = selection.path.clone();
        let parser_fingerprint = fingerprint.clone();
        let parsers = self.parsers.clone();
        let preview = tokio::task::spawn_blocking(move || {
            parsers
                .parse(
                    &parser_path,
                    selection_id,
                    &display_name,
                    &parser_fingerprint,
                )
                .map(|archive| archive.preview)
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
        self.selections.set_fingerprint(selection_id, fingerprint);
        Ok(preview)
    }

    pub async fn commit(&self, selection_id: Uuid) -> AppResult<HistoryImportResult> {
        let selection = self.selections.get(selection_id)?;
        let expected = selection.fingerprint.clone().ok_or_else(|| {
            AppError::Validation("preview the archive before importing".to_owned())
        })?;
        let fingerprint_path = selection.path.clone();
        let actual = tokio::task::spawn_blocking(move || file_fingerprint(&fingerprint_path))
            .await
            .map_err(|error| AppError::Internal(error.to_string()))??;
        if actual != expected {
            return Err(AppError::Validation(
                "the selected archive changed after preview; choose it again".to_owned(),
            ));
        }
        let parsers = self.parsers.clone();
        let path = selection.path.clone();
        let display_name = selection.display_name.clone();
        let archive = tokio::task::spawn_blocking(move || {
            parsers.parse(&path, selection_id, &display_name, &actual)
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
        let result = self.repository.commit_archive(&archive).await?;
        self.selections.remove(selection_id);
        Ok(result)
    }

    pub async fn overview(&self) -> AppResult<HistoryOverview> {
        self.repository.overview().await
    }
}

#[derive(Debug, Clone)]
pub struct HistoryContextService {
    repository: HistoryRepository,
}

impl HistoryContextService {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            repository: HistoryRepository::new(pool),
        }
    }

    pub async fn voice_examples(
        &self,
        limit: u32,
        max_chars: usize,
    ) -> AppResult<serde_json::Value> {
        self.repository
            .bounded_context(HistoryContextPurpose::Voice, limit, max_chars)
            .await
    }

    pub async fn icp_evidence(&self, limit: u32, max_chars: usize) -> AppResult<serde_json::Value> {
        self.repository
            .bounded_context(HistoryContextPurpose::Icp, limit, max_chars)
            .await
    }

    pub async fn content_examples(
        &self,
        platform: Platform,
        limit: u32,
        max_chars: usize,
    ) -> AppResult<serde_json::Value> {
        self.repository
            .bounded_context(HistoryContextPurpose::Content(platform), limit, max_chars)
            .await
    }

    pub async fn learning_evidence(
        &self,
        limit: u32,
        max_chars: usize,
    ) -> AppResult<serde_json::Value> {
        self.repository
            .bounded_context(HistoryContextPurpose::Learning, limit, max_chars)
            .await
    }

    pub async fn reply_evidence(
        &self,
        limit: u32,
        max_chars: usize,
    ) -> AppResult<serde_json::Value> {
        self.repository
            .bounded_context(HistoryContextPurpose::Reply, limit, max_chars)
            .await
    }
}
