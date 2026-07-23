Brief: Redesign the Goalbar Inbox so selected conversations and the live platform browser feel like a full-screen workbench.

User request:

- Make the inbox list and live browser area use the available screen instead of feeling embedded in a narrow page.
- Improve search/filtering.
- Keep the local-first platform truth model: scanned rows are previews, full threads live in the platform browser, and replies still require explicit approval/copy/open actions.

Constraints:

- No hosted service, telemetry, or platform API calls.
- Preserve existing send-safety behavior.
- Preserve accessibility labels for scan actions, filters, search, and conversation rows.
- Keep the design functional in the Tauri desktop app with a native webview pane on the right.
