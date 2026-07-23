CREATE TABLE ingestion_sources (
    id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    source_kind TEXT NOT NULL CHECK (source_kind IN ('archive', 'browser')),
    ownership TEXT NOT NULL CHECK (ownership IN ('own', 'reference')),
    display_name TEXT NOT NULL,
    account_handle TEXT,
    source_fingerprint TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (platform, source_kind, source_fingerprint)
);

CREATE TABLE ingestion_runs (
    id TEXT PRIMARY KEY NOT NULL,
    source_id TEXT NOT NULL REFERENCES ingestion_sources(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('preview', 'running', 'paused', 'completed', 'failed', 'cancelled')),
    provider TEXT CHECK (provider IS NULL OR provider IN ('codex', 'claude')),
    objective TEXT NOT NULL,
    limits_json TEXT NOT NULL,
    counts_json TEXT NOT NULL,
    pause_reason TEXT,
    error_code TEXT,
    started_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE activity_items (
    id TEXT PRIMARY KEY NOT NULL,
    source_id TEXT NOT NULL REFERENCES ingestion_sources(id) ON DELETE CASCADE,
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    item_kind TEXT NOT NULL CHECK (item_kind IN ('post', 'comment', 'reply', 'message', 'reaction', 'connection', 'profile')),
    ownership TEXT NOT NULL CHECK (ownership IN ('own', 'reference')),
    direction TEXT CHECK (direction IS NULL OR direction IN ('inbound', 'outbound')),
    remote_id TEXT,
    canonical_url TEXT,
    author_handle TEXT,
    counterparty_handle TEXT,
    body TEXT NOT NULL,
    published_at TEXT,
    observed_at TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    UNIQUE (platform, dedupe_key)
);

CREATE TABLE browser_checkpoints (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES ingestion_runs(id) ON DELETE CASCADE,
    step INTEGER NOT NULL CHECK (step >= 0),
    url TEXT NOT NULL,
    new_item_count INTEGER NOT NULL CHECK (new_item_count >= 0),
    total_item_count INTEGER NOT NULL CHECK (total_item_count >= 0),
    last_item_key TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (run_id, step)
);

CREATE INDEX idx_ingestion_runs_status ON ingestion_runs(status, updated_at DESC);
CREATE INDEX idx_activity_items_published ON activity_items(platform, published_at DESC);
CREATE INDEX idx_activity_items_source ON activity_items(source_id, item_kind, observed_at DESC);
CREATE INDEX idx_browser_checkpoints_run ON browser_checkpoints(run_id, step DESC);
