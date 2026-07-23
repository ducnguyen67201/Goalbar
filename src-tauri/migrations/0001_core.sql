PRAGMA foreign_keys = ON;

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE founder_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    product_name TEXT NOT NULL,
    offer TEXT NOT NULL,
    expertise TEXT NOT NULL,
    goals_json TEXT NOT NULL,
    boundaries_json TEXT NOT NULL,
    onboarding_completed INTEGER NOT NULL DEFAULT 0 CHECK (onboarding_completed IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE audit_events (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    subject_type TEXT,
    subject_id TEXT,
    detail_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_audit_events_created_at ON audit_events(created_at DESC);
