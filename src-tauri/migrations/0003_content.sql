CREATE TABLE content_ideas (
    id TEXT PRIMARY KEY NOT NULL,
    founder_id TEXT NOT NULL REFERENCES founder_profiles(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    insight TEXT NOT NULL,
    icp_hypothesis_id TEXT REFERENCES icp_hypotheses(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE experiments (
    id TEXT PRIMARY KEY NOT NULL,
    idea_id TEXT NOT NULL REFERENCES content_ideas(id) ON DELETE CASCADE,
    hypothesis TEXT NOT NULL,
    success_metric TEXT NOT NULL,
    window_days INTEGER NOT NULL CHECK (window_days BETWEEN 1 AND 365),
    status TEXT NOT NULL CHECK (status IN ('draft', 'running', 'measuring', 'complete', 'cancelled')),
    starts_at TEXT,
    ends_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE content_variants (
    id TEXT PRIMARY KEY NOT NULL,
    idea_id TEXT NOT NULL REFERENCES content_ideas(id) ON DELETE CASCADE,
    experiment_id TEXT REFERENCES experiments(id) ON DELETE SET NULL,
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    revision INTEGER NOT NULL CHECK (revision > 0),
    body TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    provider TEXT,
    provider_version TEXT,
    status TEXT NOT NULL CHECK (status IN ('draft', 'approved', 'publishing', 'published', 'failed')),
    created_at TEXT NOT NULL,
    UNIQUE (idea_id, platform, revision)
);

CREATE TABLE approvals (
    id TEXT PRIMARY KEY NOT NULL,
    subject_type TEXT NOT NULL CHECK (subject_type IN ('content_variant', 'reply', 'direct_message')),
    subject_id TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    approved_at TEXT NOT NULL,
    consumed_at TEXT,
    invalidated_at TEXT
);

CREATE INDEX idx_content_variants_status ON content_variants(status, created_at DESC);
CREATE INDEX idx_experiments_status ON experiments(status, ends_at);
