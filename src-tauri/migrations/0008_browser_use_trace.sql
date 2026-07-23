CREATE TABLE browser_research_trace_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES ingestion_runs(id) ON DELETE CASCADE,
    step INTEGER NOT NULL CHECK (step >= 0),
    action TEXT NOT NULL CHECK (action IN ('observe', 'scroll', 'open_link', 'go_back', 'finish', 'pause', 'error')),
    message TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at TEXT NOT NULL
);

INSERT INTO browser_research_trace_v2 (id, run_id, step, action, message, url, created_at)
SELECT id, run_id, step, action, message, url, created_at
FROM browser_research_trace;

DROP TABLE browser_research_trace;
ALTER TABLE browser_research_trace_v2 RENAME TO browser_research_trace;

CREATE INDEX idx_browser_research_trace_run ON browser_research_trace(run_id, step, created_at);
