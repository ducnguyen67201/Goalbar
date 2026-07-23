ALTER TABLE browser_inbox_scan_state RENAME TO browser_inbox_scan_state_legacy;

CREATE TABLE browser_inbox_scan_state (
    platform TEXT PRIMARY KEY NOT NULL CHECK (platform IN ('x', 'reddit', 'linkedin')),
    status TEXT NOT NULL CHECK (
        status IN (
            'completed',
            'partial',
            'needs_browser',
            'login_required',
            'verification_required',
            'unsupported_page'
        )
    ),
    item_count INTEGER NOT NULL DEFAULT 0 CHECK (item_count >= 0),
    last_scanned_at TEXT NOT NULL
);

DROP TABLE browser_inbox_scan_state_legacy;

UPDATE conversations
SET remote_url = 'https://www.linkedin.com/messaging/'
WHERE platform = 'linkedin'
  AND account_id = '00000000-0000-4000-8000-000000000203'
  AND LOWER(COALESCE(remote_url, '')) LIKE '%/undefined%';

UPDATE browser_inbox_ingestions
SET remote_url = 'https://www.linkedin.com/messaging/'
WHERE platform = 'linkedin'
  AND LOWER(remote_url) LIKE '%/undefined%';
