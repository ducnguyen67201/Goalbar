# Security policy

Report vulnerabilities privately to the repository owner before public disclosure.

## Security boundaries

- Secrets are stored only through the OS keyring abstraction.
- OAuth listeners bind to loopback, accept one callback, validate state/PKCE, time out, and close.
- Child agent processes receive a minimal environment and no platform credentials.
- External URLs are allowlisted to official X, Reddit, and LinkedIn HTTPS hosts.
- Remote `browser-*` child webviews match no Tauri capability; only the local `main` webview can invoke commands.
- Browser observations are initiated by Rust, bounded, normalized, and treated as untrusted data.
- Archive paths stay in an expiring Rust-only selection registry. Parsers never execute archive HTML or JavaScript.
- Archive ZIP members are path-checked and bounded by entry, member, total-expanded, and source-file limits.
- Diagnostics exclude raw private content and credentials by default.

Never include a real token or platform password in an issue. Revoke any credential that may have been exposed.

Local-only operation does not bypass platform terms, automation policies, rate limits, or account restrictions. Browser capture is not represented as official API access or as a complete account-history collector.

Clearing Tagline product memory and clearing integrated-browser sessions are separate destructive actions. The latter requires the exact `CLEAR BROWSER DATA` confirmation.
