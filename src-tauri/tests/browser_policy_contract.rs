#![allow(clippy::unwrap_used)]

use tagline_lib::browser::policy::{browser_url, collection_policy, platform_from_url};
use tagline_lib::domain::Platform;
use tagline_lib::domain::browser::BrowserPolicyState;

#[test]
fn navigation_policy_rejects_unsafe_and_deceptive_urls() {
    for unsafe_url in [
        "file:///etc/passwd",
        "data:text/html,hello",
        "javascript:alert(1)",
        "http://x.com",
        "https://x.com.evil.test",
    ] {
        assert!(
            browser_url(unsafe_url).is_err(),
            "{unsafe_url} was accepted"
        );
    }
    let supported = browser_url("https://www.linkedin.com/feed/").expect("supported URL");
    assert_eq!(platform_from_url(&supported), Some(Platform::Linkedin));
}

#[test]
fn automated_collection_defaults_to_manual_only() {
    for platform in [Platform::X, Platform::Reddit, Platform::Linkedin] {
        assert_eq!(collection_policy(platform), BrowserPolicyState::ManualOnly);
    }
}

#[test]
fn remote_browser_labels_receive_no_tauri_capability() {
    let manifest = include_str!("../capabilities/default.json");
    let capability: serde_json::Value = serde_json::from_str(manifest).expect("capability JSON");
    assert_eq!(capability["webviews"], serde_json::json!(["main"]));
    assert!(!manifest.contains("browser-*"));
    assert!(capability.get("windows").is_none());
}
