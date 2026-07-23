ALTER TABLE conversations
ADD COLUMN source TEXT NOT NULL DEFAULT 'platform_api'
CHECK (source IN ('platform_api', 'email_notification'));

ALTER TABLE conversations
ADD COLUMN content_state TEXT NOT NULL DEFAULT 'complete'
CHECK (content_state IN ('complete', 'notification_excerpt', 'link_only'));

ALTER TABLE conversations
ADD COLUMN notification_display_name TEXT;

ALTER TABLE conversations
ADD COLUMN seen_at TEXT;

CREATE TABLE email_notification_ingestions (
    source_message_id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    classification TEXT NOT NULL CHECK (classification IN ('comment_thread', 'direct_message')),
    received_at TEXT NOT NULL,
    ingested_at TEXT NOT NULL
);

CREATE INDEX idx_email_notifications_received
ON email_notification_ingestions(received_at DESC);

CREATE INDEX idx_conversations_source_unread
ON conversations(source, unread_count DESC, updated_at DESC);
