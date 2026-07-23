# Architecture

Goalbar has eight boundaries:

1. React renders local state and validates all Tauri payloads with Zod.
2. Rust commands call use-case services; UI code never calls a platform directly.
3. Goalbar’s local agent runner gives bounded, schema-constrained tasks to Codex or Claude.
4. Platform adapters execute only approved operations with tokens loaded from the keyring.
5. SQLite owns durable product memory, experiments, metrics, jobs, and audit events.
6. Rust owns native child-webview lifecycle, URL policy, semantic observation, and browser-data clearing. Remote webviews have zero Tauri capabilities.
7. Official account archives enter through an expiring opaque Rust selection, bounded parser registry, preview, and transactional normalized import.
8. Founder chat is the default agent surface. Its tools are schema-constrained requests: the Research add-on cannot observe or scroll until the founder approves its objective and limits. Interactive PTYs remain isolated infrastructure and are not the default browser UI.

The closed loop is Observe → Decide → Draft → Approve → Execute → Measure → Learn. Agents help with semantic drafting and interpretation. Rust deterministically computes permissions, metrics, scores, retries, and writes.

Two data planes remain deliberately separate:

- operational API tables hold connected-account content, conversations, messages, cursors, and metrics;
- provenance-aware ingestion tables hold archive/browser sources, runs, normalized activity items, and browser checkpoints.

Purpose-specific repository queries provide bounded voice, ICP, content, reply, and learning excerpts. Private message bodies are excluded from unrelated contexts, and imported messages are read-only history rather than an actionable inbox.

Browser research adds a review plane: structured agent decisions generate grounded `proposed` findings and an append-only action trace. Accepted findings enter future ICP context; rejected and proposed findings do not.
