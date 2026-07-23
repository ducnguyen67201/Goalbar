# Security policy

Report vulnerabilities privately to the repository owner before public disclosure.

## Security boundaries

- Secrets are stored only through the OS keyring abstraction.
- OAuth listeners bind to loopback, accept one callback, validate state/PKCE, time out, and close.
- Child agent processes receive a minimal environment and no platform credentials.
- External URLs are allowlisted to official X, Reddit, and LinkedIn HTTPS hosts.
- Diagnostics exclude raw private content and credentials by default.

Never include a real token or platform password in an issue. Revoke any credential that may have been exposed.
