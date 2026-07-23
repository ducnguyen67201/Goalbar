use url::Url;

use crate::domain::Platform;
use crate::domain::browser::{BrowserPageKind, BrowserPolicyState};
use crate::error::{AppError, AppResult};

const PLATFORM_ROOTS: &[(&str, Platform)] = &[
    ("x.com", Platform::X),
    ("twitter.com", Platform::X),
    ("reddit.com", Platform::Reddit),
    ("linkedin.com", Platform::Linkedin),
];

pub fn browser_url(value: &str) -> AppResult<Url> {
    let url = Url::parse(value).map_err(|error| AppError::Validation(error.to_string()))?;
    if url.scheme() != "https" {
        return Err(AppError::Validation(
            "browser URL must use HTTPS".to_owned(),
        ));
    }
    if platform_from_url(&url).is_none() {
        return Err(AppError::Validation(
            "browser URL must be on X, Reddit, or LinkedIn".to_owned(),
        ));
    }
    Ok(url)
}

pub fn platform_from_url(url: &Url) -> Option<Platform> {
    let host = url.host_str()?.trim_end_matches('.').to_ascii_lowercase();
    PLATFORM_ROOTS.iter().find_map(|(root, platform)| {
        (host == *root || host.ends_with(&format!(".{root}"))).then_some(*platform)
    })
}

pub fn page_kind(url: &Url) -> BrowserPageKind {
    let path = url.path().to_ascii_lowercase();
    let platform = platform_from_url(url);
    let is_post = (matches!(platform, Some(Platform::X))
        && path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            >= 3
        && path.contains("/status/"))
        || (matches!(platform, Some(Platform::Reddit)) && path.contains("/comments/"))
        || (matches!(platform, Some(Platform::Linkedin))
            && (path.contains("/posts/") || path.contains("/feed/update/")));
    if path.contains("login") || path.contains("signin") {
        BrowserPageKind::Login
    } else if path.contains("challenge") || path.contains("checkpoint") || path.contains("verify") {
        BrowserPageKind::Challenge
    } else if path.contains("messages") || path.contains("messaging") {
        BrowserPageKind::Messages
    } else if path.contains("search") {
        BrowserPageKind::Search
    } else if is_post {
        BrowserPageKind::Post
    } else if path.contains("/in/") || path.contains("/user/") || path.contains("/u/") {
        BrowserPageKind::Profile
    } else if path == "/" || path.contains("/home") || path.contains("/feed") {
        BrowserPageKind::Feed
    } else {
        BrowserPageKind::Unknown
    }
}

pub const fn capture_policy(platform: Platform) -> BrowserPolicyState {
    match platform {
        Platform::X | Platform::Reddit | Platform::Linkedin => BrowserPolicyState::ExplicitCapture,
    }
}

pub const fn collection_policy(platform: Platform) -> BrowserPolicyState {
    match platform {
        // A run is local, visible, user-initiated, explicitly bounded, and read-only.
        // It still pauses for login, verification, host changes, and uncertainty.
        Platform::X | Platform::Reddit | Platform::Linkedin => {
            BrowserPolicyState::BoundedCollection
        }
    }
}

pub fn strip_tracking(mut url: Url) -> Url {
    url.set_query(None);
    url.set_fragment(None);
    url
}

#[cfg(test)]
mod tests {
    use super::{browser_url, page_kind, platform_from_url};
    use crate::domain::Platform;
    use crate::domain::browser::BrowserPageKind;

    #[test]
    fn allows_platform_hosts_and_safe_subdomains() {
        let url = browser_url("https://old.reddit.com/r/rust").expect("allowed");
        assert_eq!(platform_from_url(&url), Some(Platform::Reddit));
        assert!(browser_url("https://reddit.com.evil.test").is_err());
        assert!(browser_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn classifies_supported_pages() {
        let url = browser_url("https://x.com/founder/status/123").expect("url");
        assert_eq!(page_kind(&url), BrowserPageKind::Post);
        let login = browser_url("https://www.linkedin.com/login").expect("url");
        assert_eq!(page_kind(&login), BrowserPageKind::Login);
    }
}
