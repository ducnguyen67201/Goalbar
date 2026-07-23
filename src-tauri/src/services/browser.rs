use chrono::Utc;
use sha2::{Digest as _, Sha256};
use tauri::{AppHandle, Emitter as _};
use uuid::Uuid;

use crate::browser::adapters::BrowserPageRegistry;
use crate::browser::extraction;
use crate::browser::manager::BrowserManager;
use crate::browser::policy::{browser_url, capture_policy, collection_policy};
use crate::db::repositories::history::HistoryRepository;
use crate::domain::browser::{
    BrowserCapturePreview, BrowserPageKind, BrowserPauseReason, BrowserPolicyState,
    BrowserRunLimits, BrowserRunProgress, BrowserRunStatus,
};
use crate::domain::history::{ActivityOwnership, HistoryImportResult, NormalizedActivityItem};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Visible,
    Selection,
}

impl CaptureMode {
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "visible" => Ok(Self::Visible),
            "selection" => Ok(Self::Selection),
            _ => Err(AppError::Validation(format!(
                "unknown browser capture mode: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BrowserConductorService {
    manager: BrowserManager,
    adapters: BrowserPageRegistry,
    repository: HistoryRepository,
}

impl BrowserConductorService {
    pub fn new(manager: BrowserManager, pool: sqlx::SqlitePool) -> Self {
        Self {
            manager,
            adapters: BrowserPageRegistry::default(),
            repository: HistoryRepository::new(pool),
        }
    }

    pub async fn preview_capture(
        &self,
        app: &AppHandle,
        tab_id: Uuid,
        mode: CaptureMode,
        ownership: ActivityOwnership,
    ) -> AppResult<BrowserCapturePreview> {
        let observation = extraction::observe(app, &self.manager, tab_id).await?;
        let platform = observation.platform.ok_or_else(|| {
            AppError::Unsupported("this page has no platform capture adapter".to_owned())
        })?;
        let selected_text = if mode == CaptureMode::Selection {
            extraction::selected_text(app, &self.manager, tab_id).await?
        } else {
            None
        };
        if mode == CaptureMode::Selection && selected_text.is_none() {
            return Err(AppError::Validation(
                "select visible text in the browser first".to_owned(),
            ));
        }
        let url = browser_url(&observation.url)?;
        let adapter = self.adapters.for_url(&url).ok_or_else(|| {
            AppError::Unsupported("this page has no platform capture adapter".to_owned())
        })?;
        let normalized = adapter.normalize(&observation, ownership, selected_text.as_deref());
        Ok(BrowserCapturePreview {
            observation,
            selected_text,
            normalized_item_count: normalized.len() as u32,
            policy_state: capture_policy(platform),
        })
    }

    pub async fn commit_capture(
        &self,
        app: &AppHandle,
        tab_id: Uuid,
        mode: CaptureMode,
        ownership: ActivityOwnership,
    ) -> AppResult<HistoryImportResult> {
        let preview = self.preview_capture(app, tab_id, mode, ownership).await?;
        let platform = preview.observation.platform.ok_or_else(|| {
            AppError::Unsupported("this page has no platform capture adapter".to_owned())
        })?;
        let url = browser_url(&preview.observation.url)?;
        let adapter = self.adapters.for_url(&url).ok_or_else(|| {
            AppError::Unsupported("this page has no platform capture adapter".to_owned())
        })?;
        let items = adapter.normalize(
            &preview.observation,
            ownership,
            preview.selected_text.as_deref(),
        );
        if items.is_empty() {
            return Err(AppError::Validation(
                "the current browser view contains no capturable content".to_owned(),
            ));
        }
        let fingerprint = capture_fingerprint(platform.as_str(), &preview.observation.url, &items);
        self.repository
            .commit_browser_capture(
                platform,
                ownership,
                "Explicit browser capture",
                &fingerprint,
                &items,
            )
            .await
    }

    pub async fn collect(
        &self,
        app: &AppHandle,
        tab_id: Uuid,
        objective: &str,
        limits: BrowserRunLimits,
        ownership: ActivityOwnership,
        provider: Option<&str>,
    ) -> AppResult<BrowserRunProgress> {
        let initial = extraction::observe(app, &self.manager, tab_id).await?;
        let platform = initial.platform.ok_or_else(|| {
            AppError::Unsupported("this page has no platform collection adapter".to_owned())
        })?;
        if collection_policy(platform) != BrowserPolicyState::BoundedCollection {
            return Err(AppError::Unsupported(
                "automated website collection is manual-only for this platform; use explicit capture or an official archive"
                    .to_owned(),
            ));
        }
        let adapter = self
            .adapters
            .for_url(&browser_url(&initial.url)?)
            .ok_or_else(|| AppError::Unsupported("no platform adapter".to_owned()))?;
        let record = self
            .repository
            .create_browser_run(platform, ownership, objective, &limits, provider)
            .await?;
        let earliest_date = limits
            .earliest_date
            .as_deref()
            .map(chrono::DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|_| {
                AppError::Validation("earliestDate must be an RFC 3339 timestamp".to_owned())
            })?
            .map(|value| value.with_timezone(&Utc));
        let cancellation = self.manager.register_run(record.run_id);
        let mut total = 0_u32;
        let mut no_new_steps = 0_u8;
        for step in 0..limits.maximum_steps {
            if cancellation.is_cancelled() {
                self.repository
                    .finish_browser_run(record.run_id, "cancelled", total, None)
                    .await?;
                self.manager.finish_run(record.run_id);
                return Ok(progress(
                    record.run_id,
                    BrowserRunStatus::Cancelled,
                    step,
                    total,
                    0,
                    None,
                    Some("Collection cancelled".to_owned()),
                ));
            }
            let observation = if step == 0 {
                initial.clone()
            } else {
                extraction::observe(app, &self.manager, tab_id).await?
            };
            if observation.platform != Some(platform) {
                self.repository
                    .finish_browser_run(
                        record.run_id,
                        "paused",
                        total,
                        Some(pause_reason(BrowserPauseReason::HostChanged)),
                    )
                    .await?;
                self.manager.finish_run(record.run_id);
                return Ok(progress(
                    record.run_id,
                    BrowserRunStatus::Paused,
                    step,
                    total,
                    0,
                    Some(BrowserPauseReason::HostChanged),
                    Some("The browser navigated to a different platform.".to_owned()),
                ));
            }
            if matches!(
                observation.page_kind,
                BrowserPageKind::Login | BrowserPageKind::Challenge
            ) {
                let reason = if observation.page_kind == BrowserPageKind::Login {
                    BrowserPauseReason::LoginRequired
                } else {
                    BrowserPauseReason::VerificationRequired
                };
                self.repository
                    .finish_browser_run(record.run_id, "paused", total, Some(pause_reason(reason)))
                    .await?;
                self.manager.finish_run(record.run_id);
                return Ok(progress(
                    record.run_id,
                    BrowserRunStatus::Paused,
                    step,
                    total,
                    0,
                    Some(reason),
                    None,
                ));
            }
            let remaining = limits.maximum_items.saturating_sub(total) as usize;
            let items = adapter
                .normalize(&observation, ownership, None)
                .into_iter()
                .filter(|item| {
                    earliest_date.as_ref().is_none_or(|earliest| {
                        item.published_at
                            .as_deref()
                            .and_then(|published| {
                                chrono::DateTime::parse_from_rfc3339(published).ok()
                            })
                            .is_none_or(|published| published >= *earliest)
                    })
                })
                .take(remaining)
                .collect::<Vec<_>>();
            let inserted = self
                .repository
                .append_browser_batch(&record, step, &observation.url, &items, total)
                .await?;
            total = total.saturating_add(inserted);
            no_new_steps = if inserted == 0 {
                no_new_steps.saturating_add(1)
            } else {
                0
            };
            let current = progress(
                record.run_id,
                BrowserRunStatus::Running,
                step,
                total,
                inserted,
                None,
                None,
            );
            let _ = app.emit_to("main", "browser://run-progress", &current);
            if total >= limits.maximum_items || no_new_steps >= 3 {
                self.repository
                    .finish_browser_run(record.run_id, "completed", total, None)
                    .await?;
                self.manager.finish_run(record.run_id);
                return Ok(progress(
                    record.run_id,
                    BrowserRunStatus::Completed,
                    step,
                    total,
                    inserted,
                    None,
                    Some(if total >= limits.maximum_items {
                        "Item limit reached".to_owned()
                    } else {
                        "No new items after three observations".to_owned()
                    }),
                ));
            }
            let delta = i32::try_from(observation.viewport.height)
                .unwrap_or(800)
                .saturating_mul(4)
                / 5;
            extraction::scroll(app, &self.manager, tab_id, delta)?;
            tokio::time::sleep(std::time::Duration::from_millis(650)).await;
        }
        self.repository
            .finish_browser_run(record.run_id, "completed", total, None)
            .await?;
        self.manager.finish_run(record.run_id);
        Ok(progress(
            record.run_id,
            BrowserRunStatus::Completed,
            limits.maximum_steps,
            total,
            0,
            None,
            Some("Step limit reached".to_owned()),
        ))
    }
}

fn progress(
    run_id: Uuid,
    status: BrowserRunStatus,
    step: u32,
    item_count: u32,
    new_item_count: u32,
    pause_reason: Option<BrowserPauseReason>,
    summary: Option<String>,
) -> BrowserRunProgress {
    BrowserRunProgress {
        run_id,
        status,
        step,
        item_count,
        new_item_count,
        pause_reason,
        summary,
    }
}

fn pause_reason(reason: BrowserPauseReason) -> &'static str {
    match reason {
        BrowserPauseReason::LoginRequired => "login_required",
        BrowserPauseReason::VerificationRequired => "verification_required",
        BrowserPauseReason::RateLimited => "rate_limited",
        BrowserPauseReason::UnsupportedPage => "unsupported_page",
        BrowserPauseReason::HostChanged => "host_changed",
        BrowserPauseReason::PolicyRestricted => "policy_restricted",
        BrowserPauseReason::Uncertain => "uncertain",
    }
}

fn capture_fingerprint(platform: &str, url: &str, items: &[NormalizedActivityItem]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(platform.as_bytes());
    hasher.update(url.as_bytes());
    hasher.update(Utc::now().date_naive().to_string().as_bytes());
    for item in items {
        hasher.update(item.dedupe_key.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
