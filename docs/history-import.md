# Official history import

Goalbar imports the user’s official X, LinkedIn, or Reddit account export into a normalized, provenance-aware local store.

## Workflow

1. Rust opens the native file chooser.
2. The absolute path is held only in an in-memory selection registry with an expiry.
3. React receives an opaque selection ID, display filename, byte size, container, and expiry.
4. Rust fingerprints and inspects inert bytes, selects a versioned parser from content, and returns a deterministic preview.
5. The user reviews platform, categories, estimated records, date range, warnings, and unsupported members.
6. Commit runs transactionally with platform-level deterministic dedupe keys.

Reimporting the same source is safe. Optional categories and malformed rows produce bounded warnings without including private row bodies. Source paths, raw archives, media, cookies, and tokens are not stored in SQLite or JSON export.

## Limits

- archive file: 1 GiB;
- ZIP entries: 10,000;
- single supported text member: 128 MiB;
- total declared expanded data: 2 GiB;
- unsupported non-media member names retained in preview: 100.

ZIP members must have enclosed paths. Parsers accept supported CSV, JSON, JavaScript-assignment, and text members as inert data and never execute them.

Archive formats can change. Parsers are tolerant and versioned, and a preview warning is preferable to silently inventing parity. Imported direct messages remain read-only historical evidence and are excluded from content/ICP/reply prompts unless a purpose-specific query explicitly allows the category; the current queries exclude private messages.
