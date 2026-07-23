# Local research browser

Tagline’s local research browser is a provider-neutral control layer around native Tauri child webviews. The React main webview draws tabs, address controls, capture controls, and progress. Rust creates and positions every remote webview and is the only layer allowed to evaluate the fixed semantic-observation scripts.

“Conductor” is an architectural inspiration only. Tagline does not integrate with or depend on an external Conductor product, SDK, or service.

## Security boundary

- Only HTTPS X, Twitter, Reddit, and LinkedIn host suffixes are accepted; deceptive suffixes and `file:`, `data:`, `javascript:`, and custom schemes are rejected.
- The sole Tauri capability targets `webviews: ["main"]`. Remote labels use `browser-<uuid>` and match no capability.
- React and local agents cannot request arbitrary JavaScript, generic clicks, cookies, passwords, tokens, raw HTML, screenshots, or network logs.
- New-window requests are denied by the remote surface and reported to the visible local toolbar for controlled handling.
- Observation is capped by block count, block text, total text, and links. URLs are canonicalized before storage.

## Operating loop

1. The user navigates and signs in locally.
2. The user chooses visible capture or selects exact text.
3. Rust observes a bounded semantic snapshot and normalizes it with an X, Reddit, or LinkedIn adapter.
4. Tagline displays a preview and explicit `own`/`reference` ownership.
5. On confirmation, SQLite stores only normalized items and provenance.
6. Codex or Claude may reason over a bounded purpose-specific excerpt, never the website session.

Bounded deterministic scrolling and checkpoints are implemented, but the shipping platform policy is `manual_only` for all three websites. This is deliberate: local execution does not make website automation permitted or reliable. Official archives are the correct bootstrap for the user’s own complete history.

Browser-assisted publishing stops at an exact copy action. Tagline provides no action that clicks final Publish, Send, purchase, delete, permission, or account-management controls.
