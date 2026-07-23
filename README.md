# Goalbar

A local-first growth operating system for solo founders. It connects to an existing Codex or Claude Code CLI, keeps durable product memory in local SQLite, and puts a controlled X, Reddit, and LinkedIn browser beside the founder’s content, ICP, inbox, and learning workflows.

## Local research browser

The desktop app includes an integrated browser with a React toolbar and Rust-owned native child webviews. Users sign into platform websites on their own machine. Goalbar can preview and normalize explicitly selected or visible content, but it never returns cookies, passwords, tokens, raw HTML, or arbitrary JavaScript to React or an agent.

For the user’s own historical record, import the official X, LinkedIn, or Reddit account archive. Archive import is the completeness path; browser capture is a bounded supplement for recent evidence and ICP research. A confirmed research run can ask the selected local Codex or Claude CLI to interpret normalized visible evidence and choose between one fixed scroll or stopping. It cannot click, message, publish, or access the website session.

Founder chat sits beside the browser and can request bounded research through an explicit approval step. Optional interactive shell, Codex, and Claude terminal infrastructure remains available for future developer workflows. See [agent workbench](docs/agent-workbench.md), [local research browser](docs/browser-conductor.md), and [history import](docs/history-import.md).

## Privacy model

- No Goalbar cloud account or backend.
- Website login can happen inside the local integrated browser. Those website sessions remain in its local browser profile.
- Optional API login happens in the official system-browser consent page.
- OAuth returns to a temporary `127.0.0.1` listener on the same machine.
- Platform tokens remain in the operating-system keyring.
- Codex and Claude receive bounded normalized evidence but never platform tokens, website sessions, or archive paths.
- External writes always require a human approval tied to the exact revision.

## Development

Prerequisites: Node.js 20+, pnpm 10+, current stable Rust, and the platform-specific [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
pnpm install
pnpm tauri dev
```

No `.env` is required. Optional diagnostics: `RUST_LOG` and `GOALBAR_HOME`. If more than one Codex CLI is installed, `GOALBAR_CODEX_PATH` can pin Goalbar to a specific healthy executable.

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

Live API tests are opt-in and require approved developer applications and dedicated test accounts. Integrated-browser tests are also manual and opt-in because platform login, CAPTCHA, and embedded-webview behavior cannot be exercised honestly in ordinary CI.

## Current platform boundary

- Browser: local sign-in, navigation, explicit preview/capture, bounded read-only research, and manual copy/paste publishing when the website permits the embedded engine.
- History: versioned, tolerant official-archive import with provenance, idempotency, and bounded downstream context.
- Official APIs: remain optional for stable posting, sync, replies, and metrics where an app has current approval.

See `docs/platform-access.md` before attempting a live connection.

## License

Goalbar is source-available under the
[Goalbar Personal Local Use License](LICENSE). You may download, run, back up,
and modify it only for your own personal, non-commercial use on devices you own
or control.

Commercial use, workplace or client use, redistribution, shared access, hosted
deployments, and software-as-a-service use are not permitted without a separate
written license. This is not an open-source license.
