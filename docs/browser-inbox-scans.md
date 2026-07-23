# Browser inbox scans

Goalbar can import recent conversation-list previews from X, Reddit, and LinkedIn without platform developer applications or paid API calls.

## Setup

1. Open the platform in Goalbar **Browser**.
2. Sign in directly on the platform website. The website session remains in the local webview profile.
3. Open **Inbox** and choose **Scan X**, **Scan Reddit**, or **Scan LinkedIn**.
4. If Goalbar reports that sign-in or verification is required, finish that step visibly in **Browser**, then scan again.
5. Select an imported conversation to open its real platform thread in the live browser pane beside the inbox.

## Scan behavior

- Every scan is explicit, read-only, bounded to five mounted conversation-list batches, and capped at 100 normalized rows.
- Goalbar may navigate an existing local platform tab to its messages page. It never creates a platform account, enters credentials, or bypasses a login or verification challenge.
- A scan stores the platform, stable row identifier where available, display name, preview, unread marker, timestamp, and same-platform conversation link.
- Repeated scans update existing rows instead of creating duplicates.
- Conversation-list HTML is undocumented and can change. A completed scan means Goalbar processed supported rows that were visible to the local webview; it does not guarantee complete historical coverage.

## Trust and write boundary

Browser previews are incomplete and untrusted. Goalbar can use a preview to draft and record approval for exact text, but it cannot send from a browser-scanned row. Selecting a row reuses the signed-in local platform tab in the right-side pane so the user can verify the real context, copy the approved text, and send manually on the platform.

Cookies, passwords, website tokens, raw HTML, and arbitrary page JavaScript are not stored or passed to Codex or Claude.

Automatic background scans are intentionally not enabled. A future background mode must preserve the same local-session, bounded-read, typed-pause, and no-send boundaries.
