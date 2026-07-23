CREATE TABLE browser_research_trace (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES ingestion_runs(id) ON DELETE CASCADE,
    step INTEGER NOT NULL CHECK (step >= 0),
    action TEXT NOT NULL CHECK (action IN ('observe', 'scroll', 'finish', 'pause', 'error')),
    message TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE browser_research_findings (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES ingestion_runs(id) ON DELETE CASCADE,
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    category TEXT NOT NULL CHECK (category IN ('pain', 'goal', 'objection', 'language', 'trigger', 'content_theme', 'counter_evidence')),
    summary TEXT NOT NULL,
    evidence_excerpt TEXT NOT NULL,
    source_url TEXT NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed', 'accepted', 'rejected')),
    dedupe_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (run_id, dedupe_key)
);

CREATE INDEX idx_browser_research_trace_run ON browser_research_trace(run_id, step, created_at);
CREATE INDEX idx_browser_research_findings_run ON browser_research_findings(run_id, status, created_at DESC);
CREATE INDEX idx_browser_research_findings_memory ON browser_research_findings(status, updated_at DESC);
