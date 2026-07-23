# Goalbar engineering contract

- Keep the product local-first. Do not add a hosted service or telemetry without an explicit decision.
- Rust owns persistence, credentials, OAuth, platform I/O, child processes, deterministic scoring, and external writes.
- TypeScript owns presentation and validates every Tauri payload with Zod 4.
- Codex and Claude never receive platform tokens and never call platform APIs directly.
- Every publish, reply, or DM requires approval for an exact content revision.
- Never log or persist access tokens, refresh tokens, authorization codes, client secrets, or platform passwords.
- Treat unsupported platform capabilities as typed states. Never scrape to create fake parity.
- Use bound SQL parameters, transactions for multi-record changes, and explicit enum parsing.
- Run `pnpm verify` and the Rust checks documented in `README.md` before handing off changes.
