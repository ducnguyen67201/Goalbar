# Tagline

A local-first growth operating system for solo founders. It connects to an existing Codex or Claude Code CLI, keeps durable product memory in local SQLite, and operates approved X, Reddit, and LinkedIn capabilities through Rust adapters.

## Privacy model

- No Tagline cloud account or backend.
- Platform login happens in the official system-browser consent page.
- OAuth returns to a temporary `127.0.0.1` listener on the same machine.
- Platform tokens remain in the operating-system keyring.
- Codex and Claude receive bounded content context but never platform tokens.
- External writes always require a human approval tied to the exact revision.

## Development

Prerequisites: Node.js 20+, pnpm 10+, current stable Rust, and the platform-specific [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
pnpm install
pnpm tauri dev
```

No `.env` is required. Optional diagnostics: `RUST_LOG` and `TAGLINE_HOME`.

## Validation

```bash
pnpm format:check
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
pnpm tauri build --debug
pnpm audit --audit-level high
pnpm audit:rust
```

The scoped RustSec exception used by `audit:rust` is documented in `docs/security-audit.md`.

Live platform tests are opt-in and require approved developer applications and dedicated test accounts. They must never run in ordinary CI.

## Current platform boundary

- X: OAuth 2.0 PKCE, posts, eligible replies, DMs, and permitted metrics.
- Reddit: approved installed-app OAuth, posts, comments, and private messages subject to current Data API terms.
- LinkedIn: approved native PKCE and social capabilities; general member DMs are explicitly unsupported and open in LinkedIn instead.

See `docs/platform-access.md` before attempting a live connection.

## License

AGPL-3.0-or-later. This follows the initial product brief decision and should be reconfirmed before accepting outside contributions.
