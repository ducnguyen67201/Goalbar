CREATE TABLE saved_browser_replies (
    id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    target_url TEXT NOT NULL,
    exact_reply TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'prepared' CHECK (status IN ('prepared', 'confirmed_posted')),
    prepared_at TEXT NOT NULL,
    confirmed_posted_at TEXT
);

CREATE INDEX idx_saved_browser_replies_recent
ON saved_browser_replies(platform, prepared_at DESC);
