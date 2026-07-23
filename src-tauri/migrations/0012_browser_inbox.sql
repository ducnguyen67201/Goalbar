CREATE TABLE browser_inbox_ingestions (
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    remote_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL UNIQUE REFERENCES conversations(id) ON DELETE CASCADE,
    remote_url TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    last_scanned_at TEXT NOT NULL,
    PRIMARY KEY (platform, remote_id)
);

CREATE TABLE browser_inbox_scan_state (
    platform TEXT PRIMARY KEY NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    status TEXT NOT NULL CHECK (
        status IN (
            'completed',
            'needs_browser',
            'login_required',
            'verification_required',
            'unsupported_page'
        )
    ),
    item_count INTEGER NOT NULL DEFAULT 0 CHECK (item_count >= 0),
    last_scanned_at TEXT NOT NULL
);

CREATE INDEX idx_browser_inbox_last_seen
ON browser_inbox_ingestions(platform, last_seen_at DESC);
