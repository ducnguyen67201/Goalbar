CREATE TABLE relationships (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    icp_confidence REAL CHECK (icp_confidence IS NULL OR (icp_confidence >= 0 AND icp_confidence <= 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE relationship_identities (
    id TEXT PRIMARY KEY NOT NULL,
    relationship_id TEXT NOT NULL REFERENCES relationships(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    handle TEXT NOT NULL,
    profile_url TEXT,
    confirmed INTEGER NOT NULL DEFAULT 0 CHECK (confirmed IN (0, 1)),
    UNIQUE (platform, remote_id)
);

CREATE TABLE conversations (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES connected_accounts(id) ON DELETE CASCADE,
    relationship_id TEXT REFERENCES relationships(id) ON DELETE SET NULL,
    platform TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('comment_thread', 'direct_message')),
    unread_count INTEGER NOT NULL DEFAULT 0 CHECK (unread_count >= 0),
    reply_capability TEXT NOT NULL CHECK (reply_capability IN ('supported', 'unsupported', 'approval_pending', 'unknown')),
    remote_url TEXT,
    updated_at TEXT NOT NULL,
    UNIQUE (platform, account_id, remote_id)
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    remote_id TEXT NOT NULL,
    sender_remote_id TEXT,
    body TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound')),
    sent_at TEXT NOT NULL,
    UNIQUE (conversation_id, remote_id)
);

CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    payload_json TEXT NOT NULL,
    result_json TEXT,
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 3 CHECK (max_attempts > 0),
    lease_expires_at TEXT,
    next_attempt_at TEXT,
    error_code TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE job_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    attempt INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL,
    error_code TEXT,
    duration_ms INTEGER,
    UNIQUE (job_id, attempt)
);

CREATE INDEX idx_conversations_unread ON conversations(unread_count DESC, updated_at DESC);
CREATE INDEX idx_messages_sent ON messages(conversation_id, sent_at);
CREATE INDEX idx_jobs_due ON jobs(status, next_attempt_at, created_at);
