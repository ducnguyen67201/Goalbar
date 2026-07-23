use url::Url;

use crate::browser::adapters::{BrowserPageAdapter, normalize_observation};
use crate::browser::policy::{collection_policy, platform_from_url};
use crate::domain::Platform;
use crate::domain::browser::{BrowserObservation, BrowserPolicyState};
use crate::domain::history::{ActivityOwnership, NormalizedActivityItem};

#[derive(Debug)]
pub struct RedditBrowserAdapter;

impl BrowserPageAdapter for RedditBrowserAdapter {
    fn platform(&self) -> Platform {
        Platform::Reddit
    }

    fn matches(&self, url: &Url) -> bool {
        platform_from_url(url) == Some(self.platform())
    }

    fn collection_policy(&self) -> BrowserPolicyState {
        collection_policy(self.platform())
    }

    fn normalize(
        &self,
        observation: &BrowserObservation,
        ownership: ActivityOwnership,
        selected_text: Option<&str>,
    ) -> Vec<NormalizedActivityItem> {
        normalize_observation(self.platform(), observation, ownership, selected_text)
    }
}
