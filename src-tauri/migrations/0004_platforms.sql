CREATE TABLE connected_accounts (
    id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    client_id TEXT NOT NULL,
    remote_account_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    secret_ref TEXT NOT NULL UNIQUE,
    scopes_json TEXT NOT NULL,
    capabilities_json TEXT NOT NULL,
    token_expires_at TEXT,
    status TEXT NOT NULL CHECK (status IN ('connected', 'reauth_required', 'revoked', 'error')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (platform, remote_account_id)
);

CREATE TABLE oauth_transactions (
    id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL,
    state_hash TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'received', 'complete', 'failed', 'expired')),
    error_code TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE sync_cursors (
    account_id TEXT NOT NULL REFERENCES connected_accounts(id) ON DELETE CASCADE,
    resource TEXT NOT NULL,
    cursor TEXT,
    last_success_at TEXT,
    next_attempt_at TEXT,
    PRIMARY KEY (account_id, resource)
);

CREATE TABLE remote_content (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES connected_accounts(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    content_type TEXT NOT NULL,
    body TEXT NOT NULL,
    remote_url TEXT,
    author_remote_id TEXT,
    published_at TEXT,
    collected_at TEXT NOT NULL,
    raw_json TEXT,
    UNIQUE (platform, account_id, remote_id)
);

CREATE TABLE metric_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    remote_content_id TEXT NOT NULL REFERENCES remote_content(id) ON DELETE CASCADE,
    metric_name TEXT NOT NULL,
    value REAL,
    availability TEXT NOT NULL CHECK (availability IN ('available', 'missing', 'restricted', 'delayed')),
    source_definition TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    collected_at TEXT NOT NULL,
    UNIQUE (remote_content_id, metric_name, observed_at)
);

CREATE INDEX idx_remote_content_published ON remote_content(account_id, published_at DESC);
CREATE INDEX idx_metric_snapshots_window ON metric_snapshots(metric_name, observed_at DESC);
