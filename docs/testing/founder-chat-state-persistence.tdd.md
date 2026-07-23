# Founder chat state persistence — TDD evidence

## Source and journey

The journey was derived from the reported Browser workspace regression:

> As a founder, I want my Codex chat and active reply to remain available after I leave and return to
> Browser, so I can continue where I stopped without seeing an initialization error.

## Task report

Goalbar now keeps each Codex thread as a durable chat instead of treating the panel as one
component-local transcript. A newly started chat stays in the chat list while Codex catches up its
persisted thread index, saved chats continue to request their turns even when Codex does not expose
a local path, and only the exact unmaterialized-thread response falls back to
`includeTurns: false`. After the first accepted turn, subsequent reads include the persisted
transcript. Running turns, Browser Use context, streaming output, and cancellation are keyed by
thread, so creating a new chat starts an independent Codex turn without stopping or hiding the
previous one.

- RED: `cargo test --manifest-path src-tauri/Cargo.toml unmaterialized_chat_is_read_without_turns --all-features`
  failed to compile because `thread_read_params` did not exist.
- GREEN: the same command passed after the protocol selection was implemented.
- Additional RED/GREEN cases cover the Codex index delay, saved chats whose `path` is null, the
  exact retryable app-server error, and switching between a blank new chat and saved transcripts.
- Concurrent-chat RED: the React test found a global `Stop Codex response` button on the new idle
  chat, while the Rust test failed to compile because no per-thread active-turn registry existed.
- Concurrent-chat GREEN: the new chat exposes its own composer, both thread submissions start before
  either finishes, switching tabs preserves each spinner, and Stop sends the selected thread ID.
- Rust regression suite: `cargo test --manifest-path src-tauri/Cargo.toml --all-features` passed 96
  unit tests and 24 integration-contract tests.
- Focused UI suite:
  `pnpm exec vitest run src/features/browser/FounderChatPanel.test.tsx --coverage` passed all five
  chat-state integration tests.
- Full frontend suite: `pnpm test` passed 64 tests across 15 files.
- Production web build: `pnpm build` passed.

## Test specification

| What is guaranteed                                                          | Test or command                                                                                                                                 | Type              | Result |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | ------ |
| A fresh, unmaterialized thread is read without requesting unavailable turns | `unmaterialized_chat_is_read_without_turns`                                                                                                     | Rust unit         | PASS   |
| A materialized thread still requests and restores its turns                 | `unmaterialized_chat_is_read_without_turns` and `persisted_codex_turns_restore_as_a_goalbar_chat_transcript`                                    | Rust unit         | PASS   |
| A locally started chat stays switchable while Codex indexes it              | `locally_started_chats_remain_switchable_until_the_persisted_index_catches_up`                                                                  | Rust unit         | PASS   |
| A saved chat with no exposed local path still restores turns                | `persisted_chat_without_an_exposed_path_still_restores_its_turns`                                                                               | Rust unit         | PASS   |
| Only Codex's exact pre-first-message error triggers the no-turns retry      | `only_the_codex_unmaterialized_error_retries_without_turns`                                                                                     | Rust unit         | PASS   |
| Leaving and remounting Browser restores the selected transcript             | `FounderChatPanel persistence > hydrates the current Codex transcript every time the browser panel mounts`                                      | React integration | PASS   |
| Switching chats while another reply is active does not discard either chat  | `FounderChatPanel persistence > can switch chats while another Codex turn keeps running`                                                        | React integration | PASS   |
| Creating a chat and switching away/back preserves both chat states          | `FounderChatPanel persistence > keeps new and saved chats switchable without clearing either transcript`                                        | React integration | PASS   |
| A new chat can run while the previous chat remains active                   | `FounderChatPanel persistence > starts a second Codex chat while the first chat keeps running`                                                  | React integration | PASS   |
| Stop affects only the selected running thread                               | `starts a second Codex chat while the first chat keeps running` and `different_codex_threads_can_run_concurrently_but_each_thread_has_one_turn` | React + Rust      | PASS   |
| Browser context and cancellation are isolated per running thread            | `different_codex_threads_can_run_concurrently_but_each_thread_has_one_turn`                                                                     | Rust unit         | PASS   |

## Coverage and known gaps

The focused coverage run reports 46.20% statement coverage for `FounderChatPanel.tsx`; the
repository-wide number is not meaningful for this focused command because coverage also includes
generated browser scripts, build output, and unrelated screens. The Rust protocol branches added
here are exercised directly. The repository does not currently define a Rust coverage command.

The full `pnpm verify` gate currently stops at one unrelated pre-existing Prettier warning in
`src-tauri/browser-scripts/inbox-scan.js`. The full frontend test suite, TypeScript production
build, Rust tests, and Rust Clippy checks pass. Repository-wide `cargo fmt --check` is also blocked
by a pre-existing formatting change in `src-tauri/src/services/browser_inbox.rs`; the changed Rust
files pass a file-scoped Rustfmt check.

`pnpm audit --audit-level high` reports no known dependency vulnerabilities, and the scoped diff
scan found no credential patterns or console logging.

The debug application bundle passed with
`pnpm tauri build --debug --bundles app`. The broader `pnpm tauri build --debug` also built the binary
and `.app`, but the local Tauri `bundle_dmg.sh` step could not eject its temporary writable volume.
That packaging-only failure is unrelated to chat persistence.

No checkpoint commits were created because the worktree already contained the user's in-progress
changes across the same files.
