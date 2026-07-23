pub mod linkedin;
pub mod reddit;
pub mod x;

use std::fmt::Debug;
use std::sync::Arc;

use chrono::Utc;
use sha2::{Digest as _, Sha256};
use url::Url;

use crate::browser::policy::{browser_url, strip_tracking};
use crate::domain::Platform;
use crate::domain::browser::{BrowserObservation, BrowserPageKind, BrowserPolicyState};
use crate::domain::history::{ActivityItemKind, ActivityOwnership, NormalizedActivityItem};

use self::linkedin::LinkedInBrowserAdapter;
use self::reddit::RedditBrowserAdapter;
use self::x::XBrowserAdapter;

pub trait BrowserPageAdapter: Debug + Send + Sync {
    fn platform(&self) -> Platform;
    fn matches(&self, url: &Url) -> bool;
    fn collection_policy(&self) -> BrowserPolicyState;
    fn normalize(
        &self,
        observation: &BrowserObservation,
        ownership: ActivityOwnership,
        selected_text: Option<&str>,
    ) -> Vec<NormalizedActivityItem>;
}

#[derive(Debug, Clone)]
pub struct BrowserPageRegistry {
    adapters: Vec<Arc<dyn BrowserPageAdapter>>,
}

impl Default for BrowserPageRegistry {
    fn default() -> Self {
        Self {
            adapters: vec![
                Arc::new(XBrowserAdapter),
                Arc::new(RedditBrowserAdapter),
                Arc::new(LinkedInBrowserAdapter),
            ],
        }
    }
}

impl BrowserPageRegistry {
    pub fn for_url(&self, url: &Url) -> Option<Arc<dyn BrowserPageAdapter>> {
        self.adapters
            .iter()
            .find(|adapter| adapter.matches(url))
            .cloned()
    }
}

pub(crate) fn normalize_observation(
    platform: Platform,
    observation: &BrowserObservation,
    ownership: ActivityOwnership,
    selected_text: Option<&str>,
) -> Vec<NormalizedActivityItem> {
    let observed_at = Utc::now().to_rfc3339();
    if let Some(selected_text) = selected_text.filter(|value| !value.trim().is_empty()) {
        return vec![normalized_item(
            platform,
            page_item_kind(observation.page_kind),
            ownership,
            selected_text,
            Some(&observation.url),
            None,
            &observed_at,
        )];
    }
    observation
        .visible_blocks
        .iter()
        .filter(|block| !block.text.trim().is_empty())
        .map(|block| {
            let canonical = block
                .links
                .iter()
                .filter_map(|value| browser_url(value).ok())
                .map(strip_tracking)
                .next()
                .map(|url| url.to_string())
                .unwrap_or_else(|| observation.url.clone());
            normalized_item(
                platform,
                page_item_kind(observation.page_kind),
                ownership,
                &block.text,
                Some(&canonical),
                block.timestamp.as_deref(),
                &observed_at,
            )
        })
        .collect()
}

fn normalized_item(
    platform: Platform,
    item_kind: ActivityItemKind,
    ownership: ActivityOwnership,
    body: &str,
    canonical_url: Option<&str>,
    published_at: Option<&str>,
    observed_at: &str,
) -> NormalizedActivityItem {
    let bounded_body = body.chars().take(20_000).collect::<String>();
    let identity = format!(
        "{}\n{}\n{}\n{}",
        platform.as_str(),
        item_kind.as_str(),
        canonical_url.unwrap_or_default(),
        bounded_body
    );
    let dedupe_key = format!("{:x}", Sha256::digest(identity.as_bytes()));
    NormalizedActivityItem {
        platform,
        item_kind,
        ownership,
        direction: None,
        remote_id: None,
        canonical_url: canonical_url.map(str::to_owned),
        author_handle: None,
        counterparty_handle: None,
        body: bounded_body,
        published_at: published_at.map(str::to_owned),
        observed_at: observed_at.to_owned(),
        dedupe_key,
        metadata: serde_json::json!({"capture": "browser", "schemaVersion": 1}),
    }
}

fn page_item_kind(page_kind: BrowserPageKind) -> ActivityItemKind {
    match page_kind {
        BrowserPageKind::Profile => ActivityItemKind::Profile,
        BrowserPageKind::Messages => ActivityItemKind::Message,
        BrowserPageKind::Post => ActivityItemKind::Post,
        BrowserPageKind::Feed
        | BrowserPageKind::Search
        | BrowserPageKind::Login
        | BrowserPageKind::Challenge
        | BrowserPageKind::Unknown => ActivityItemKind::Post,
    }
}
