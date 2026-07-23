#![allow(clippy::unwrap_used)]

use goalbar_lib::browser::policy::{browser_url, collection_policy, platform_from_url};
use goalbar_lib::domain::Platform;
use goalbar_lib::domain::browser::BrowserPolicyState;

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
fn bounded_read_only_research_requires_the_explicit_collection_path() {
    for platform in [Platform::X, Platform::Reddit, Platform::Linkedin] {
        assert_eq!(
            collection_policy(platform),
            BrowserPolicyState::BoundedCollection
        );
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

#[test]
fn main_window_launches_maximized_and_remains_resizable() {
    for (name, config) in [
        ("base", include_str!("../tauri.conf.json")),
        ("macOS", include_str!("../tauri.macos.conf.json")),
    ] {
        let config: serde_json::Value = serde_json::from_str(config).expect("Tauri config JSON");
        let main_window = &config["app"]["windows"][0];

        assert_eq!(
            main_window["title"],
            serde_json::json!("Goalbar"),
            "{name} title"
        );
        assert_eq!(
            main_window["maximized"],
            serde_json::json!(true),
            "{name} maximize"
        );
        assert_eq!(
            main_window["resizable"],
            serde_json::json!(true),
            "{name} resize"
        );
        assert_eq!(
            main_window["fullscreen"],
            serde_json::Value::Null,
            "{name} fullscreen"
        );
    }
}
