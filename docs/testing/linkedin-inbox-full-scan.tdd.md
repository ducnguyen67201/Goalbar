# LinkedIn inbox full scan — TDD evidence

## Source and user journeys

No plan file was supplied. The journeys were derived from the reported live failure:

1. As a Goalbar user, I can open a scanned LinkedIn conversation without Goalbar navigating to a placeholder `/undefined` route.
2. As a Goalbar user, my first inbox scan traverses the conversation list exposed by the website, while later scans stop after reaching known conversations.
3. As an existing user, malformed local LinkedIn scan rows are repaired and re-keyed without duplicating or deleting their previews.
4. As a Goalbar user, I see imported counts when a scan is partial and Goalbar does not claim complete history.

## RED and GREEN evidence

| Guarantee                                                                                              | Test target                                                                                                                            | RED evidence                                                                  | GREEN evidence                                                               |
| ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| LinkedIn placeholder routes fall back to the messaging inbox and dynamic `ember` IDs are not persisted | `src/features/inbox/browser-inbox-script.test.ts` and `browser::inbox::tests::linkedin_placeholder_thread_urls_fall_back_to_the_inbox` | Vitest received `ember47` plus `/undefined/`; Rust retained the malformed URL | Frontend suite: 54/54 passed; Rust production check passed                   |
| A repaired legacy row updates instead of duplicating                                                   | `services::browser_inbox::tests::linkedin_rescan_repairs_legacy_placeholder_rows_without_duplication`                                  | Result was `(1 imported, 0 updated)` instead of `(0, 1)`                      | Repair logic now re-keys the ingestion and conversation in one transaction   |
| Initial scans continue past the old five-batch cap; incremental scans stop at known rows               | `src-tauri/tests/browser_inbox_scan_progress.rs`                                                                                       | Compile-time RED: scan-mode/progress types did not exist                      | 3/3 integration tests passed                                                 |
| Partial scans are typed and their imported counts remain visible                                       | `src/schemas/schemas.test.ts`, `src/features/inbox/InboxPage.test.tsx`, and `migration_fourteen_supports_partial_inbox_scans`          | Zod rejected `partial`; UI displayed “needs attention”                        | Frontend suite passed; migration integration test passed                     |
| Existing malformed local URLs are safely upgraded                                                      | `0014_browser_inbox_full_scan.sql` against a SQLite backup                                                                             | Backup contained 19 malformed LinkedIn ingestion URLs                         | After migration: 0 malformed, 19 safe messaging-inbox URLs, scan state reset |

## Commands run

- RED: `pnpm test -- src/features/inbox/browser-inbox-script.test.ts src/schemas/schemas.test.ts src/features/inbox/InboxPage.test.tsx`
- RED: `cargo test --manifest-path src-tauri/Cargo.toml browser_inbox --all-features`
- GREEN: `pnpm test`
- GREEN: `cargo test --manifest-path src-tauri/Cargo.toml --test browser_inbox_scan_progress --all-features`
- GREEN: `cargo test --manifest-path src-tauri/Cargo.toml --test migrations migration_fourteen_supports_partial_inbox_scans --all-features`
- GREEN: `cargo check --manifest-path src-tauri/Cargo.toml --lib --all-features`
- GREEN: `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --all-features -- -D warnings`
- GREEN: `cargo test --manifest-path src-tauri/Cargo.toml --all-features` (all unit, integration, contract, migration, and doc tests passed)
- COVERAGE: `pnpm exec vitest run --coverage --coverage.reporter=text`

## Coverage and known gaps

- Task-relevant frontend coverage: `inbox-scan.js` 100% statements/branches/functions/lines, `schemas/inbox.ts` 100%, and the Inbox feature 85.31% statements.
- Repository-wide frontend coverage is 32.47% because generated assets and several unrelated application surfaces are included with no tests; the task-relevant paths exceed the 80% target.
- Live LinkedIn DOM behavior remains a manual integration boundary because ordinary CI cannot use the user's signed-in local webview.
- A completed browser scan means Goalbar reached the oldest row exposed by that page; official platform archives remain the completeness path for full message contents.

## Checkpoint evidence

- `dacd2c8` — failing URL, schema, UI, and legacy-row reproducers.
- `fc10bba` — failing initial/incremental scan behavior reproducers.
