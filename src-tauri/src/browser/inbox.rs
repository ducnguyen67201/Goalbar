use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use tauri::AppHandle;
use uuid::Uuid;

use crate::browser::BrowserManager;
use crate::browser::extraction::{evaluate, parse_evaluation};
use crate::browser::policy::{browser_url, platform_from_url, strip_tracking};
use crate::domain::Platform;
use crate::error::{AppError, AppResult};
use crate::validation::payload_hash;

const INBOX_SCAN_SCRIPT: &str = include_str!("../../browser-scripts/inbox-scan.js");
const MAX_INITIAL_SCAN_BATCHES: usize = 500;
const MAX_INCREMENTAL_SCAN_BATCHES: usize = 50;
const MAX_INITIAL_ITEMS: usize = 10_000;
const MAX_INCREMENTAL_ITEMS: usize = 1_000;
const MAX_STALLED_BATCHES: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserInboxPageState {
    Ready,
    LoginRequired,
    VerificationRequired,
    UnsupportedPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserInboxDirection {
    Inbound,
    Outbound,
}

impl BrowserInboxDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserInboxItem {
    pub remote_id: String,
    pub display_name: String,
    pub preview: String,
    pub unread: bool,
    pub remote_url: String,
    pub timestamp: Option<String>,
    pub direction: BrowserInboxDirection,
}

#[derive(Debug, Clone)]
pub struct BrowserInboxPageScan {
    pub state: BrowserInboxPageState,
    pub items: Vec<BrowserInboxItem>,
    pub mode: BrowserInboxScanMode,
    pub stop: BrowserInboxScanStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserInboxScanMode {
    Initial,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserInboxScanStop {
    Exhausted,
    KnownConversation,
    SafetyLimit,
    Stalled,
}

impl BrowserInboxScanStop {
    pub const fn is_partial(self) -> bool {
        matches!(self, Self::SafetyLimit | Self::Stalled)
    }
}

#[derive(Debug)]
pub struct BrowserInboxScanProgress {
    mode: BrowserInboxScanMode,
    known_remote_ids: HashSet<String>,
    observed_remote_ids: HashSet<String>,
    batch_count: usize,
    stalled_batches: usize,
}

impl BrowserInboxScanProgress {
    pub fn new(mode: BrowserInboxScanMode, known_remote_ids: HashSet<String>) -> Self {
        Self {
            mode,
            known_remote_ids,
            observed_remote_ids: HashSet::new(),
            batch_count: 0,
            stalled_batches: 0,
        }
    }

    pub fn observe<'a>(
        &mut self,
        remote_ids: impl IntoIterator<Item = &'a str>,
        has_more: bool,
    ) -> Option<BrowserInboxScanStop> {
        self.batch_count += 1;
        let mut new_items = 0;
        let mut reached_known_conversation = false;
        for remote_id in remote_ids {
            reached_known_conversation |= self.known_remote_ids.contains(remote_id);
            new_items += usize::from(self.observed_remote_ids.insert(remote_id.to_owned()));
        }
        self.stalled_batches = if new_items == 0 {
            self.stalled_batches.saturating_add(1)
        } else {
            0
        };

        if !has_more {
            return Some(BrowserInboxScanStop::Exhausted);
        }
        if self.mode == BrowserInboxScanMode::Incremental && reached_known_conversation {
            return Some(BrowserInboxScanStop::KnownConversation);
        }
        if self.batch_count >= self.maximum_batches()
            || self.observed_remote_ids.len() >= self.maximum_items()
        {
            return Some(BrowserInboxScanStop::SafetyLimit);
        }
        if self.stalled_batches >= MAX_STALLED_BATCHES {
            return Some(BrowserInboxScanStop::Stalled);
        }
        None
    }

    const fn maximum_batches(&self) -> usize {
        match self.mode {
            BrowserInboxScanMode::Initial => MAX_INITIAL_SCAN_BATCHES,
            BrowserInboxScanMode::Incremental => MAX_INCREMENTAL_SCAN_BATCHES,
        }
    }

    const fn maximum_items(&self) -> usize {
        match self.mode {
            BrowserInboxScanMode::Initial => MAX_INITIAL_ITEMS,
            BrowserInboxScanMode::Incremental => MAX_INCREMENTAL_ITEMS,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawBatch {
    state: BrowserInboxPageState,
    items: Vec<RawItem>,
    has_more: bool,
    target_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawItem {
    remote_id: String,
    display_name: String,
    preview: String,
    unread: bool,
    remote_url: String,
    timestamp: Option<String>,
    direction: BrowserInboxDirection,
}

pub async fn scan(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    platform: Platform,
    mode: BrowserInboxScanMode,
    known_remote_ids: HashSet<String>,
) -> AppResult<BrowserInboxPageScan> {
    let outcome = scan_batches(app, manager, tab_id, platform, mode, known_remote_ids).await;
    let _ = evaluate_mode(app, manager, tab_id, platform, "finish").await;
    outcome
}

async fn scan_batches(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    platform: Platform,
    mode: BrowserInboxScanMode,
    known_remote_ids: HashSet<String>,
) -> AppResult<BrowserInboxPageScan> {
    let mut items = HashMap::new();
    let mut state = BrowserInboxPageState::Ready;
    let mut progress = BrowserInboxScanProgress::new(mode, known_remote_ids);
    let mut stop = BrowserInboxScanStop::Exhausted;
    for index in 0..MAX_INITIAL_SCAN_BATCHES {
        let script_mode = if index == 0 { "start" } else { "next" };
        let batch = evaluate_mode(app, manager, tab_id, platform, script_mode).await?;
        state = batch.state;
        if state != BrowserInboxPageState::Ready {
            break;
        }
        let normalized = batch
            .items
            .into_iter()
            .filter_map(|raw| normalize_item(raw, platform))
            .collect::<Vec<_>>();
        let decision = progress.observe(
            normalized.iter().map(|item| item.remote_id.as_str()),
            batch.has_more,
        );
        for item in normalized {
            items.insert(item.remote_id.clone(), item);
        }
        if let Some(reason) = decision {
            stop = reason;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    let mut items = items.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| left.remote_id.cmp(&right.remote_id));
    Ok(BrowserInboxPageScan {
        state,
        items,
        mode,
        stop,
    })
}

async fn evaluate_mode(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    platform: Platform,
    mode: &str,
) -> AppResult<RawBatch> {
    let platform_value = serde_json::to_string(platform.as_str())?;
    let mode = serde_json::to_string(mode)?;
    let script = format!(
        "globalThis.__GOALBAR_INBOX_SCAN_PLATFORM__ = {platform_value};\
         globalThis.__GOALBAR_INBOX_SCAN_MODE__ = {mode};\
         {INBOX_SCAN_SCRIPT}"
    );
    let raw = evaluate(app, manager, tab_id, &script).await?;
    let batch: RawBatch = parse_evaluation(&raw)?;
    let expected_target = browser_url(&batch.target_url)?;
    if platform_from_url(&expected_target) != Some(platform) {
        return Err(AppError::Validation(
            "browser inbox scanner returned a cross-platform target".to_owned(),
        ));
    }
    Ok(batch)
}

fn normalize_item(raw: RawItem, platform: Platform) -> Option<BrowserInboxItem> {
    let display_name = bounded(&raw.display_name, 120);
    if display_name.is_empty() {
        return None;
    }
    let remote_url = normalized_remote_url(&raw.remote_url, platform)?;
    if platform_from_url(&remote_url) != Some(platform) {
        return None;
    }
    let mut remote_id = bounded(&raw.remote_id, 500);
    if remote_id.is_empty() {
        remote_id = payload_hash(&format!(
            "{}\n{}\n{}",
            platform.as_str(),
            display_name,
            remote_url
        ));
    }
    let preview = bounded(&raw.preview, 600);
    Some(BrowserInboxItem {
        remote_id,
        display_name,
        preview: if preview.is_empty() {
            "Open this conversation on the platform.".to_owned()
        } else {
            preview
        },
        unread: raw.unread,
        remote_url: remote_url.to_string(),
        timestamp: raw
            .timestamp
            .map(|value| bounded(&value, 80))
            .filter(|value| !value.is_empty()),
        direction: raw.direction,
    })
}

fn normalized_remote_url(value: &str, platform: Platform) -> Option<url::Url> {
    let remote_url = strip_tracking(browser_url(value).ok()?);
    if platform == Platform::Linkedin
        && remote_url
            .path_segments()
            .map(|mut segments| segments.any(|segment| segment.eq_ignore_ascii_case("undefined")))
            .unwrap_or(false)
    {
        return browser_url("https://www.linkedin.com/messaging/").ok();
    }
    Some(remote_url)
}

fn bounded(value: &str, maximum: usize) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(maximum)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        BrowserInboxDirection, BrowserInboxScanMode, BrowserInboxScanProgress,
        BrowserInboxScanStop, RawItem, normalize_item,
    };
    use crate::domain::Platform;

    #[test]
    fn browser_inbox_items_are_bounded_and_same_platform() {
        let valid = normalize_item(
            RawItem {
                remote_id: "messages/123".to_owned(),
                display_name: "Ari".to_owned(),
                preview: "A useful note".to_owned(),
                unread: true,
                remote_url: "https://x.com/messages/123?tracking=1".to_owned(),
                timestamp: Some("6d".to_owned()),
                direction: BrowserInboxDirection::Inbound,
            },
            Platform::X,
        )
        .expect("valid item");
        assert_eq!(valid.remote_url, "https://x.com/messages/123");
        assert!(valid.unread);
        assert_eq!(valid.direction, BrowserInboxDirection::Inbound);

        assert!(
            normalize_item(
                RawItem {
                    remote_url: "https://reddit.com/message/messages/1".to_owned(),
                    ..RawItem {
                        remote_id: "messages/1".to_owned(),
                        display_name: "Wrong host".to_owned(),
                        preview: "No".to_owned(),
                        unread: false,
                        remote_url: String::new(),
                        timestamp: None,
                        direction: BrowserInboxDirection::Inbound,
                    }
                },
                Platform::X,
            )
            .is_none()
        );
    }

    #[test]
    fn linkedin_placeholder_thread_urls_fall_back_to_the_inbox() {
        let item = normalize_item(
            RawItem {
                remote_id: "fallback:linkedin:ross mcintyre".to_owned(),
                display_name: "Ross McIntyre".to_owned(),
                preview: "Status is reachable".to_owned(),
                unread: false,
                remote_url: "https://www.linkedin.com/messaging/thread/2-mailbox/undefined/"
                    .to_owned(),
                timestamp: None,
                direction: BrowserInboxDirection::Inbound,
            },
            Platform::Linkedin,
        )
        .expect("the row remains useful without a false deep link");

        assert_eq!(item.remote_url, "https://www.linkedin.com/messaging/");
    }

    #[test]
    fn initial_scan_continues_past_the_old_five_batch_limit_until_exhausted() {
        let mut progress =
            BrowserInboxScanProgress::new(BrowserInboxScanMode::Initial, HashSet::new());

        for index in 0..6 {
            let remote_id = format!("conversation-{index}");
            assert_eq!(progress.observe([remote_id.as_str()], true), None);
        }

        assert_eq!(
            progress.observe(["oldest-conversation"], false),
            Some(BrowserInboxScanStop::Exhausted)
        );
    }

    #[test]
    fn incremental_scan_stops_after_it_reaches_a_known_conversation() {
        let mut progress = BrowserInboxScanProgress::new(
            BrowserInboxScanMode::Incremental,
            HashSet::from(["known-conversation".to_owned()]),
        );

        assert_eq!(progress.observe(["new-conversation"], true), None);
        assert_eq!(
            progress.observe(["known-conversation"], true),
            Some(BrowserInboxScanStop::KnownConversation)
        );
    }
}
