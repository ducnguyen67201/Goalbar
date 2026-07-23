use std::collections::HashSet;

use goalbar_lib::browser::inbox::{
    BrowserInboxScanMode, BrowserInboxScanProgress, BrowserInboxScanStop,
};

#[test]
fn initial_scan_continues_past_the_legacy_batch_limit() {
    let mut progress = BrowserInboxScanProgress::new(BrowserInboxScanMode::Initial, HashSet::new());

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
fn incremental_scan_stops_at_a_known_conversation() {
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

#[test]
fn a_stalled_full_scan_reports_a_partial_stop() {
    let mut progress = BrowserInboxScanProgress::new(BrowserInboxScanMode::Initial, HashSet::new());

    assert_eq!(progress.observe(["conversation"], true), None);
    for _ in 0..5 {
        assert_eq!(progress.observe(["conversation"], true), None);
    }

    let stop = progress
        .observe(["conversation"], true)
        .expect("six stalled batches stop the scan");
    assert_eq!(stop, BrowserInboxScanStop::Stalled);
    assert!(stop.is_partial());
}
