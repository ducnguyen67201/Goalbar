ALTER TABLE icp_hypotheses
ADD COLUMN version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0);

ALTER TABLE icp_hypotheses
ADD COLUMN parent_id TEXT REFERENCES icp_hypotheses(id) ON DELETE SET NULL;

CREATE INDEX idx_icp_hypotheses_version
ON icp_hypotheses(founder_id, version DESC);

CREATE TABLE growth_actions (
    id TEXT PRIMARY KEY NOT NULL,
    founder_id TEXT NOT NULL REFERENCES founder_profiles(id) ON DELETE CASCADE,
    icp_hypothesis_id TEXT REFERENCES icp_hypotheses(id) ON DELETE SET NULL,
    experiment_id TEXT REFERENCES experiments(id) ON DELETE SET NULL,
    kind TEXT NOT NULL CHECK (kind IN ('research', 'follow', 'comment', 'post')),
    platform TEXT CHECK (platform IS NULL OR platform IN ('x', 'reddit', 'linkedin')),
    title TEXT NOT NULL,
    rationale TEXT NOT NULL,
    target_url TEXT,
    exact_payload TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    hypothesis TEXT NOT NULL,
    success_metric TEXT NOT NULL,
    evaluation_window_days INTEGER NOT NULL DEFAULT 7 CHECK (evaluation_window_days BETWEEN 1 AND 365),
    status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed', 'approved', 'completed', 'failed', 'cancelled', 'measured')),
    scheduled_for TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_growth_actions_queue
ON growth_actions(founder_id, status, scheduled_for, created_at DESC);

CREATE TABLE approvals_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    subject_type TEXT NOT NULL CHECK (subject_type IN ('content_variant', 'reply', 'direct_message', 'growth_action')),
    subject_id TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    approved_at TEXT NOT NULL,
    consumed_at TEXT,
    invalidated_at TEXT
);

INSERT INTO approvals_v2 (
    id,
    subject_type,
    subject_id,
    payload_hash,
    idempotency_key,
    approved_at,
    consumed_at,
    invalidated_at
)
SELECT
    id,
    subject_type,
    subject_id,
    payload_hash,
    idempotency_key,
    approved_at,
    consumed_at,
    invalidated_at
FROM approvals;

DROP TABLE approvals;
ALTER TABLE approvals_v2 RENAME TO approvals;

CREATE TABLE growth_action_executions (
    id TEXT PRIMARY KEY NOT NULL,
    action_id TEXT NOT NULL REFERENCES growth_actions(id) ON DELETE CASCADE,
    approval_id TEXT NOT NULL REFERENCES approvals(id) ON DELETE RESTRICT,
    outcome TEXT NOT NULL CHECK (outcome IN ('succeeded', 'failed')),
    result_url TEXT,
    detail TEXT NOT NULL,
    attempted_at TEXT NOT NULL
);

CREATE INDEX idx_growth_action_executions_action
ON growth_action_executions(action_id, attempted_at DESC);

CREATE TABLE growth_action_metrics (
    id TEXT PRIMARY KEY NOT NULL,
    action_id TEXT NOT NULL REFERENCES growth_actions(id) ON DELETE CASCADE,
    metric_name TEXT NOT NULL,
    value REAL,
    availability TEXT NOT NULL CHECK (availability IN ('available', 'missing', 'restricted', 'delayed')),
    source_definition TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    observed_at TEXT NOT NULL,
    collected_at TEXT NOT NULL,
    UNIQUE (action_id, metric_name, observed_at)
);

CREATE INDEX idx_growth_action_metrics_window
ON growth_action_metrics(metric_name, observed_at DESC);

ALTER TABLE learnings
ADD COLUMN growth_action_id TEXT REFERENCES growth_actions(id) ON DELETE SET NULL;

CREATE INDEX idx_learnings_growth_action
ON learnings(growth_action_id, created_at DESC);
