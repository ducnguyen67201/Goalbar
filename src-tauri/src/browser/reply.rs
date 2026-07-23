use serde::Deserialize;
use tauri::AppHandle;
use uuid::Uuid;

use crate::browser::BrowserManager;
use crate::browser::extraction::{evaluate, parse_evaluation};
use crate::browser::policy::{browser_url, page_kind, platform_from_url, strip_tracking};
use crate::domain::Platform;
use crate::domain::browser::{
    BrowserLoadState, BrowserPageKind, BrowserReplyPreparation, BrowserReplyPreparationStatus,
};
use crate::error::{AppError, AppResult};

const PREPARE_REPLY_SCRIPT: &str = include_str!("../../browser-scripts/prepare-reply.js");
const MAX_REPLY_CHARS: usize = 8_000;
const MAX_PREPARATION_ATTEMPTS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawPreparationState {
    Prepared,
    ComposerOpening,
    ComposerNotFound,
    LoginRequired,
    VerificationRequired,
    UnsupportedPage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawPreparation {
    state: RawPreparationState,
    character_count: usize,
}

pub async fn prepare(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    target_url: &str,
    exact_reply: &str,
) -> AppResult<BrowserReplyPreparation> {
    let reply = validate_exact_reply(exact_reply)?;
    let target = strip_tracking(browser_url(target_url)?);
    if page_kind(&target) != BrowserPageKind::Post {
        return Ok(result(
            BrowserReplyPreparationStatus::UnsupportedPage,
            platform_from_url(&target),
            0,
        ));
    }
    let platform = platform_from_url(&target).ok_or_else(|| {
        AppError::Validation("reply preparation requires X, Reddit, or LinkedIn".to_owned())
    })?;

    let current = manager.tab(tab_id)?;
    let current_url = strip_tracking(browser_url(&current.current_url)?);
    if current_url != target {
        manager.navigate(app, tab_id, target.as_str())?;
    }

    for _ in 0..MAX_PREPARATION_ATTEMPTS {
        let tab = manager.tab(tab_id)?;
        let loaded_url = strip_tracking(browser_url(&tab.current_url)?);
        if tab.load_state != BrowserLoadState::Loaded {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            continue;
        }
        match page_kind(&loaded_url) {
            BrowserPageKind::Login => {
                return Ok(result(
                    BrowserReplyPreparationStatus::LoginRequired,
                    Some(platform),
                    0,
                ));
            }
            BrowserPageKind::Challenge => {
                return Ok(result(
                    BrowserReplyPreparationStatus::VerificationRequired,
                    Some(platform),
                    0,
                ));
            }
            _ => {}
        }
        if !same_post_target(&loaded_url, &target) {
            return Ok(result(
                BrowserReplyPreparationStatus::UnsupportedPage,
                platform_from_url(&loaded_url),
                0,
            ));
        }

        let raw = evaluate_preparation(app, manager, tab_id, platform, reply).await?;
        let status = match raw.state {
            RawPreparationState::Prepared => BrowserReplyPreparationStatus::Prepared,
            RawPreparationState::LoginRequired => BrowserReplyPreparationStatus::LoginRequired,
            RawPreparationState::VerificationRequired => {
                BrowserReplyPreparationStatus::VerificationRequired
            }
            RawPreparationState::UnsupportedPage => BrowserReplyPreparationStatus::UnsupportedPage,
            RawPreparationState::ComposerOpening | RawPreparationState::ComposerNotFound => {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                continue;
            }
        };
        return Ok(result(status, Some(platform), raw.character_count));
    }

    Ok(result(
        BrowserReplyPreparationStatus::ComposerNotFound,
        Some(platform),
        0,
    ))
}

async fn evaluate_preparation(
    app: &AppHandle,
    manager: &BrowserManager,
    tab_id: Uuid,
    platform: Platform,
    reply: &str,
) -> AppResult<RawPreparation> {
    let platform_value = serde_json::to_string(platform.as_str())?;
    let reply_value = serde_json::to_string(reply)?;
    let script = format!(
        "globalThis.__GOALBAR_REPLY_PLATFORM__ = {platform_value};\
         globalThis.__GOALBAR_REPLY_TEXT__ = {reply_value};\
         {PREPARE_REPLY_SCRIPT}"
    );
    let raw = evaluate(app, manager, tab_id, &script).await?;
    parse_evaluation(&raw)
}

fn validate_exact_reply(value: &str) -> AppResult<&str> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(
            "the exact reply cannot be empty".to_owned(),
        ));
    }
    if value.chars().count() > MAX_REPLY_CHARS {
        return Err(AppError::Validation(format!(
            "the exact reply cannot exceed {MAX_REPLY_CHARS} characters"
        )));
    }
    Ok(value)
}

fn same_post_target(current: &url::Url, target: &url::Url) -> bool {
    platform_from_url(current) == platform_from_url(target)
        && current.path().trim_end_matches('/') == target.path().trim_end_matches('/')
}

fn result(
    status: BrowserReplyPreparationStatus,
    platform: Option<Platform>,
    character_count: usize,
) -> BrowserReplyPreparation {
    BrowserReplyPreparation {
        status,
        platform,
        character_count: u32::try_from(character_count).unwrap_or(u32::MAX),
        saved_reply: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PREPARE_REPLY_SCRIPT, same_post_target, validate_exact_reply};
    use crate::browser::policy::browser_url;

    #[test]
    fn exact_reply_validation_preserves_the_approved_revision() {
        let reply = "First line.\n\nSecond line.";
        assert_eq!(validate_exact_reply(reply).ok(), Some(reply));
        assert!(validate_exact_reply(" \n ").is_err());
        assert!(validate_exact_reply(&"a".repeat(8_001)).is_err());
    }

    #[test]
    fn preparation_script_never_selects_or_clicks_a_submit_control() {
        assert!(!PREPARE_REPLY_SCRIPT.contains("tweetButton"));
        assert!(!PREPARE_REPLY_SCRIPT.contains("comments-comment-box__submit-button"));
        assert!(!PREPARE_REPLY_SCRIPT.contains(".submit("));
        assert!(PREPARE_REPLY_SCRIPT.contains("not([type=\"submit\"])"));
    }

    #[test]
    fn target_matching_allows_safe_host_redirects_but_not_a_different_post() {
        let target = browser_url("https://reddit.com/r/startups/comments/one").expect("target");
        let redirected =
            browser_url("https://www.reddit.com/r/startups/comments/one/").expect("redirected");
        let different =
            browser_url("https://www.reddit.com/r/startups/comments/two").expect("different");

        assert!(same_post_target(&redirected, &target));
        assert!(!same_post_target(&different, &target));
    }
}
