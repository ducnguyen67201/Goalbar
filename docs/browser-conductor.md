# Local research browser

Goalbar’s local research browser is a provider-neutral control layer around native Tauri child webviews. The React main webview draws founder chat, tabs, address controls, tool approval, and progress. Rust creates and positions every remote webview and is the only layer allowed to evaluate the fixed semantic-observation scripts.

“Conductor” is an architectural inspiration only. Goalbar does not integrate with or depend on an external Conductor product, SDK, or service.

## Security boundary

- Only HTTPS X, Twitter, Reddit, and LinkedIn host suffixes are accepted; deceptive suffixes and `file:`, `data:`, `javascript:`, and custom schemes are rejected.
- The sole Tauri capability targets `webviews: ["main"]`. Remote labels use `browser-<uuid>` and match no capability.
- React and local agents cannot request arbitrary JavaScript, generic clicks, cookies, passwords, tokens, raw HTML, screenshots, or network logs.
- New-window requests are denied by the remote surface and reported to the visible local toolbar for controlled handling.
- Observation is capped by block count, block text, total text, and links. URLs are canonicalized before storage.

## Operating loop

1. The user navigates and signs in locally.
2. The user asks the persistent founder chat a question.
3. When current evidence is required, chat proposes the Research add-on with an explicit objective, ownership, and hard limits.
4. Only after confirmation does Rust observe a bounded semantic snapshot and normalize it with an X, Reddit, or LinkedIn adapter.
5. For the confirmed research run, Codex or Claude receives the normalized observation plus bounded, previously approved ICP context.
6. The structured decision may propose grounded findings and choose one fixed scroll or stop. Rust rejects evidence excerpts that are not present in the visible observation.
7. Proposed findings return to chat in a review queue. Only explicit acceptance adds them to future ICP context.

Bounded read-only scrolling and checkpoints are enabled only after the user confirms the objective and hard item/step limits. Local execution does not make website automation universally permitted or reliable, so runs stop on login, verification, host changes, or uncertainty. Official archives remain the correct bootstrap for the user’s own complete history.

Browser-assisted publishing stops at an exact copy action. Goalbar provides no action that clicks final Publish, Send, purchase, delete, permission, or account-management controls.
