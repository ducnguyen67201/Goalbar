use std::sync::{Arc, Mutex, PoisonError};

use serde::Deserialize;
use tauri::{AppHandle, Manager as _};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::browser::manager::BrowserManager;
use crate::browser::policy::{browser_url, page_kind, platform_from_url, strip_tracking};
use crate::domain::browser::{
    BROWSER_SCHEMA_VERSION, BrowserObservation, BrowserObservationBlock, BrowserViewport,
};
use crate::error::{AppError, AppResult};

const OBSERVATION_SCRIPT: &str = include_str!("../../browser-scripts/semantic-observation.js");
const FEED_OBSERVATION_SCRIPT: &str = include_str!("../../browser-scripts/feed-observation.js");
const SELECTION_SCRIPT: &str = include_str!("../../browser-scripts/selection-capture.js");
const MAX_BLOCKS: usize = 40;
const MAX_FEED_BLOCKS: usize = 80;
const MAX_BLOCK_CHARS: usize = 4_000;
const MAX_TOTAL_CHARS: usize = 60_000;
const MAX_LINKS: usize = 12;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawObservation {
    title: String,
    viewport: BrowserViewport,
    blocks: Vec<BrowserObservationBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSelection {
    selected_text: Option<String>,
}

pub async fn observe(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
) -> AppResult<BrowserObservation> {
    observe_with_script(app, manager, tab_id, OBSERVATION_SCRIPT, MAX_BLOCKS).await
}

pub async fn observe_feed(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
) -> AppResult<BrowserObservation> {
    observe_with_script(
        app,
        manager,
        tab_id,
        FEED_OBSERVATION_SCRIPT,
        MAX_FEED_BLOCKS,
    )
    .await
}

async fn observe_with_script(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    script: &str,
    maximum_blocks: usize,
) -> AppResult<BrowserObservation> {
    let tab = manager.tab(tab_id)?;
    let raw = evaluate(app, manager, tab_id, script).await?;
    let mut observation: RawObservation = parse_evaluation(&raw)?;
    let current_url = browser_url(&tab.current_url)?;
    let mut remaining = MAX_TOTAL_CHARS;
    let mut blocks = Vec::new();
    for mut block in observation.blocks.drain(..).take(maximum_blocks) {
        block.text = normalize_text(&block.text, MAX_BLOCK_CHARS.min(remaining));
        if block.text.is_empty() {
            continue;
        }
        remaining = remaining.saturating_sub(block.text.chars().count());
        block.links = block
            .links
            .into_iter()
            .filter_map(|value| browser_url(&value).ok())
            .map(strip_tracking)
            .map(|url| url.to_string())
            .take(MAX_LINKS)
            .collect();
        block.timestamp = block
            .timestamp
            .filter(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok());
        block.key = normalize_text(&block.key, 160);
        block.role = normalize_text(&block.role, 80);
        blocks.push(block);
        if remaining == 0 {
            break;
        }
    }
    Ok(BrowserObservation {
        schema_version: BROWSER_SCHEMA_VERSION,
        tab_id,
        url: strip_tracking(current_url.clone()).to_string(),
        title: normalize_text(&observation.title, 160),
        platform: platform_from_url(&current_url),
        page_kind: page_kind(&current_url),
        viewport: observation.viewport,
        visible_blocks: blocks,
        captured_item_keys: Vec::new(),
        warning: None,
    })
}

pub async fn selected_text(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
) -> AppResult<Option<String>> {
    let raw = evaluate(app, manager, tab_id, SELECTION_SCRIPT).await?;
    let selection: RawSelection = parse_evaluation(&raw)?;
    Ok(selection
        .selected_text
        .map(|text| normalize_text(&text, 20_000))
        .filter(|text| !text.is_empty()))
}

pub fn scroll(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    delta_y: i32,
) -> AppResult<()> {
    let label = manager.webview_label(tab_id)?;
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| AppError::NotFound("browser surface".to_owned()))?;
    webview
        .eval(format!(
            "window.scrollBy({{ top: {delta_y}, behavior: 'auto' }})"
        ))
        .map_err(|error| AppError::Internal(error.to_string()))
}

async fn evaluate(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    script: &str,
) -> AppResult<String> {
    let label = manager.webview_label(tab_id)?;
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| AppError::NotFound("browser surface".to_owned()))?;
    let (sender, receiver) = oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    webview
        .eval_with_callback(script, move |value| {
            let sender = sender.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(sender) = sender {
                let _ = sender.send(value);
            }
        })
        .map_err(|error| AppError::Internal(error.to_string()))?;
    tokio::time::timeout(std::time::Duration::from_secs(5), receiver)
        .await
        .map_err(|_| AppError::Timeout("browser observation".to_owned()))?
        .map_err(|_| AppError::Internal("browser callback closed".to_owned()))
}

fn parse_evaluation<T: for<'de> Deserialize<'de>>(raw: &str) -> AppResult<T> {
    let first: serde_json::Value = serde_json::from_str(raw)?;
    match first {
        serde_json::Value::String(value) => Ok(serde_json::from_str(&value)?),
        value => Ok(serde_json::from_value(value)?),
    }
}

fn normalize_text(value: &str, max_chars: usize) -> String {
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
        .take(max_chars)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{FEED_OBSERVATION_SCRIPT, normalize_text};

    #[test]
    fn normalizes_and_bounds_untrusted_text() {
        assert_eq!(normalize_text(" a\n\tb\u{0}c ", 3), "a b");
        assert_eq!(normalize_text("abcdef", 4), "abcd");
    }

    #[test]
    fn feed_observation_reads_all_mounted_posts_without_using_the_clipboard() {
        assert!(FEED_OBSERVATION_SCRIPT.contains("article, [role='article']"));
        assert!(!FEED_OBSERVATION_SCRIPT.contains("getBoundingClientRect"));
        assert!(!FEED_OBSERVATION_SCRIPT.contains("clipboard"));
    }
}
