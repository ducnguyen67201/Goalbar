CREATE TABLE voice_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    founder_id TEXT NOT NULL REFERENCES founder_profiles(id) ON DELETE CASCADE,
    traits_json TEXT NOT NULL,
    do_rules_json TEXT NOT NULL,
    dont_rules_json TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 0 CHECK (active IN (0, 1)),
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE voice_examples (
    id TEXT PRIMARY KEY NOT NULL,
    voice_profile_id TEXT NOT NULL REFERENCES voice_profiles(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    original_text TEXT NOT NULL,
    approved_text TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE icp_hypotheses (
    id TEXT PRIMARY KEY NOT NULL,
    founder_id TEXT NOT NULL REFERENCES founder_profiles(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    situation TEXT NOT NULL,
    urgent_problem TEXT NOT NULL,
    current_workaround TEXT NOT NULL,
    desired_outcome TEXT NOT NULL,
    objections_json TEXT NOT NULL,
    language_json TEXT NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    status TEXT NOT NULL CHECK (status IN ('proposed', 'active', 'rejected', 'archived')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE icp_evidence (
    id TEXT PRIMARY KEY NOT NULL,
    hypothesis_id TEXT NOT NULL REFERENCES icp_hypotheses(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,
    source_id TEXT,
    summary TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('supports', 'contradicts', 'neutral')),
    weight REAL NOT NULL CHECK (weight >= 0 AND weight <= 1),
    accepted INTEGER NOT NULL DEFAULT 0 CHECK (accepted IN (0, 1)),
    created_at TEXT NOT NULL
);

CREATE TABLE learnings (
    id TEXT PRIMARY KEY NOT NULL,
    founder_id TEXT NOT NULL REFERENCES founder_profiles(id) ON DELETE CASCADE,
    experiment_id TEXT,
    summary TEXT NOT NULL,
    evidence_json TEXT NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    status TEXT NOT NULL CHECK (status IN ('proposed', 'accepted', 'edited', 'rejected')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_icp_evidence_created ON icp_evidence(hypothesis_id, created_at DESC);
CREATE INDEX idx_learnings_created ON learnings(founder_id, created_at DESC);
