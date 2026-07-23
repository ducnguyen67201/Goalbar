# Architecture

Tagline has five boundaries:

1. React renders local state and validates all Tauri payloads with Zod.
2. Rust commands call use-case services; UI code never calls a platform directly.
3. The Conductor gives bounded, schema-constrained tasks to Codex or Claude.
4. Platform adapters execute only approved operations with tokens loaded from the keyring.
5. SQLite owns durable product memory, experiments, metrics, jobs, and audit events.

The closed loop is Observe → Decide → Draft → Approve → Execute → Measure → Learn. Agents help with semantic drafting and interpretation. Rust deterministically computes permissions, metrics, scores, retries, and writes.
