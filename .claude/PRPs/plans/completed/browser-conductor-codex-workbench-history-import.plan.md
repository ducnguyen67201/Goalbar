# Plan: Browser Conductor, Codex-Inspired Workbench, and History Import

> Status: Implemented and validated on 2026-07-22. See the corresponding report in `.claude/PRPs/reports/`.

## Summary

Turn Goalbar into a light, Codex-inspired desktop workbench with an integrated local browser for X, Reddit, and LinkedIn, while preserving Goalbar’s identity and local-first security model. Add a provider-neutral Browser Conductor that lets the existing Codex or Claude CLI adapters reason over bounded semantic page observations, captures only normalized user-approved content, and never gives remote pages access to Tauri IPC, secrets, or external-write commands.

Bootstrap the founder’s complete personal history through official X, LinkedIn, and Reddit archive imports. Use browser collection only as a bounded, user-initiated supplement for recent or explicitly selected content; do not claim that browser scrolling can produce a complete or platform-approved history.

## User Story

As a solo founder, I want to browse my social channels, import my historical activity, and work with Codex or Claude from one local workspace, so that Goalbar can learn my voice and ICP without forcing me to switch applications, create a hosted account, or hand credentials to a third party.

## Problem → Solution

The current dark dashboard opens platform authentication externally, requires developer application IDs for official integrations, and has no browser/history ingestion surface → a light three-pane workbench provides:

1. local signed-in browsing in an isolated child webview;
2. explicit capture and bounded collection into local SQLite;
3. official archive import for complete personal history;
4. structured Codex/Claude analysis over normalized, provenance-aware records;
5. exact human approval before any outward-facing content is used;
6. optional official API adapters that remain available for stable platform operations.

## Metadata

- **Complexity**: XL
- **Source PRD**: N/A — standalone feature based on `README.md`
- **PRD Phase**: N/A
- **Estimated Files**: 45–55
- **Estimated Tasks**: 13
- **Primary platforms**: macOS, Windows, Linux desktop
- **Mobile scope**: excluded
- **Primary implementation risk**: Tauri multiwebview is still behind the `unstable` Cargo feature

---

## Product Decisions

### Decision 1: Goalbar owns the browser bridge

Do not attempt to call private or undocumented Browser/Computer Use APIs from the Codex desktop app. The current Goalbar integration starts `codex exec` or `claude` as local child processes, and the documented Codex built-in browser is not available in Codex CLI.

Goalbar will own:

- child-webview lifecycle and geometry;
- URL and navigation policy;
- semantic DOM observation;
- deterministic capture, scrolling, deduplication, and checkpoints;
- local persistence and provenance;
- approval and pause states.

Codex or Claude will own:

- deciding what a bounded observation means;
- classifying selected/captured records;
- proposing the next safe Browser Conductor action when deterministic collection cannot proceed;
- producing ICP, voice, content, and reply drafts from normalized context.

### Decision 2: DOM-first, not screenshot-first

Browser Conductor v1 uses bounded semantic DOM snapshots and deterministic platform normalization. This is sufficient for feed/post/profile capture and avoids adding unsafe, platform-specific native screenshot code.

Codex CLI supports image attachments, but cross-platform child-webview screenshot capture is not provided by Tauri’s stable high-level API. Full visual coordinate-based Computer Use is explicitly deferred.

### Decision 3: One isolated browser webview per tab

Use Tauri child webviews created by Rust and attached to the existing main native window. Each browser tab has a unique label and shares the application’s browser profile so the user can remain signed in.

The local React webview renders the browser chrome. Remote child webviews render only remote HTTPS pages. Remote child webviews receive no capabilities and no Tauri IPC.

### Decision 4: Archive import is the history authority

- **Complete personal history**: official account archive import.
- **Recent personal activity**: official API sync when approved, otherwise explicit browser capture.
- **ICP exploration**: user-driven browsing and explicit capture.
- **Browser automation**: bounded and disabled when the platform policy/capability state does not permit it.

### Decision 5: Add an ingestion store without rewriting operational API tables

Keep `connected_accounts`, `remote_content`, `conversations`, and `messages` as the operational API-backed data plane.

Add a provenance-aware ingestion data plane for archives and browser observations:

- `ingestion_sources`
- `ingestion_runs`
- `activity_items`
- `browser_checkpoints`

This avoids risky table rebuilds and allows API, archive, and browser records to be merged through repository read models using stable dedupe keys.

### Decision 6: Browser-assisted publishing remains human-controlled

MVP behavior:

- Goalbar generates the draft beside the browser.
- The exact approved draft can be copied.
- The user manually pastes and clicks Publish/Send.
- `fillDraft` exists as a typed capability but is not enabled for a platform unless its current policy/access state permits non-API assistance.
- No Browser Conductor action can click a final submit, send, publish, purchase, delete, permission, or account-management control.

---

## UX Design

### Visual direction

The reference screenshot establishes the structural direction, not a request to copy Codex branding or proprietary assets.

Use:

- off-white application background;
- white work surfaces;
- subtle gray sidebar and hairline dividers;
- compact system sans-serif typography;
- flat panels with 10–14 px radii;
- active navigation with a quiet gray fill;
- Goalbar lime only for status, progress, and primary intent;
- very limited shadows, reserved for overlays and transient notifications;
- a transparent/overlay macOS title bar with native traffic lights;
- preserved native window decorations on Windows and Linux.

Do not use:

- Codex or OpenAI names, marks, duck art, or product copy;
- large serif display headings;
- dark green gradients;
- floating cards for every section;
- a fake browser drawn with an `<iframe>`.

### Design tokens

```css
--app-bg: #f7f7f5;
--sidebar-bg: #f3f3f1;
--surface: #ffffff;
--surface-subtle: #f7f7f6;
--text: #20201f;
--text-muted: #74746f;
--border: #e4e4e0;
--border-strong: #d7d7d2;
--active: #eaeae7;
--accent: #b8df55;
--accent-ink: #283112;
--danger: #c9423a;
--warning: #a46a18;
--radius-sm: 8px;
--radius-md: 12px;
--radius-lg: 16px;
--titlebar-height: 48px;
--sidebar-width: 244px;
```

### Before

```text
┌──────────────┬───────────────────────────────────────────────┐
│ Dark sidebar │ Large dark dashboard cards                  │
│              │                                               │
│ Today        │ Settings → developer client IDs → OAuth      │
│ Create       │                                               │
│ Inbox        │ Browser opens outside Goalbar                │
│ Growth       │ No archive/history import                    │
│ Memory       │ No browser capture                           │
│ Settings     │                                               │
└──────────────┴───────────────────────────────────────────────┘
```

### After — ordinary route

```text
┌──────────────────────────────────────────────────────────────────────┐
│ Native/overlay title bar · Goalbar · route title · local status    │
├──────────────┬───────────────────────────────────────────────────────┤
│ Goalbar      │ Compact route header                                │
│              ├───────────────────────────────────────────────────────┤
│ Today        │ Existing product page restyled with flat surfaces   │
│ Browser      │                                                       │
│ Create       │                                                       │
│ Inbox        │                                                       │
│ Growth       │                                                       │
│ Memory       │                                                       │
│              │                                                       │
│ Settings     │                                                       │
└──────────────┴───────────────────────────────────────────────────────┘
```

### After — Browser route

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Native/overlay title bar · Goalbar · Browser · Local session               │
├────────────┬────────────────────────────┬─────────────────────────────────────┤
│ Navigation │ Browser Conductor          │ Browser tabs                       │
│            │                            ├─────────────────────────────────────┤
│ Today      │ Objective                  │ ←  →  ↻   https://...          ⋮   │
│ Browser ●  │ Provider: Codex / Claude   ├─────────────────────────────────────┤
│ Create     │                            │                                     │
│ Inbox      │ [Capture visible]          │ Remote X / Reddit / LinkedIn       │
│ Growth     │ [Capture selection]        │ child webview                       │
│ Memory     │ [Collect bounded sample]   │                                     │
│            │                            │                                     │
│ Settings   │ Progress / pauses / items  │                                     │
│            │                            │                                     │
│            │ History import             │                                     │
│            │ [Choose archive]           │                                     │
└────────────┴────────────────────────────┴─────────────────────────────────────┘
```

### Responsive behavior

- At widths ≥ 1280 px: sidebar + conductor pane + browser pane.
- At 960–1279 px: sidebar collapses to icons; conductor pane can collapse; browser remains usable.
- Below the current 960 px desktop minimum: unsupported by configuration, unchanged.
- Browser pane is never hidden behind a remote webview during route changes.
- A keyboard-accessible divider adjusts conductor/browser width and persists the ratio in `app_settings`.
- Reduced-motion preference disables animated pane transitions and progress pulses.

### Interaction Changes

| Touchpoint      | Before                                | After                                              | Notes                                               |
| --------------- | ------------------------------------- | -------------------------------------------------- | --------------------------------------------------- |
| App shell       | Dark fixed sidebar, padded page       | Light compact workbench                            | Preserve route semantics                            |
| Title bar       | Native default title                  | macOS overlay; native Windows/Linux                | Never fake traffic lights                           |
| Browser access  | External system browser only          | New `/browser` workspace                           | System-browser OAuth remains available              |
| Login           | Platform OAuth consent only           | User may sign into platform website locally        | Browser cookies never enter SQLite or agent context |
| Capture         | API sync only                         | Explicit selection/page/visible-item capture       | Normalize before persistence                        |
| Feed collection | None                                  | Bounded run with item/date/step limits             | Pause on login, CAPTCHA, or unknown state           |
| Archive history | None                                  | Choose → preview → commit → summary                | No absolute path shown to frontend                  |
| Draft posting   | API publish or manual switching       | Draft beside browser, copy/manual submit           | No automated submit                                 |
| Settings        | Platform developer IDs appear primary | Browser/local history first; APIs labeled optional | Do not remove official adapters                     |
| Memory          | Founder baseline and hypotheses only  | Shows imported/captured evidence counts            | Agent receives bounded excerpts                     |

---

## Architecture

### End-to-end flow

```text
Local React main webview
    │ typed Tauri commands/events
    ▼
Rust BrowserManager ────────────────┐
    │ create/show/hide/bounds       │ no IPC capability
    ▼                               ▼
Remote child webview          X / Reddit / LinkedIn
    │ eval_with_callback
    ▼
Semantic BrowserObservation
    │ validate + normalize + dedupe
    ▼
BrowserConductor
    ├── deterministic PageAdapter path
    └── structured Codex/Claude decision fallback
            │
            ▼
      BrowserAction policy gate
            │
            ▼
      capture / scroll / pause / stop
            │
            ▼
SQLite ingestion_sources + activity_items + checkpoints
            │
            ▼
HistoryContextService → ICP / voice / content / learning prompts
```

### Browser tool contract

The provider-neutral contract is implemented in Rust and serialized with `camelCase`.

```text
BrowserObservation
  tabId
  url
  title
  platform?
  pageKind
  viewport
  visibleBlocks[]       // bounded text, links, timestamps, semantic role
  capturedItemKeys[]
  warning?

BrowserAction
  observe
  navigate { url }
  scroll { deltaY }
  captureVisible { ownership }
  captureSelection { ownership }
  requestUserAction { reason, recovery }
  stop { summary }
```

There is intentionally no generic `click`, arbitrary JavaScript, cookie read, network interception, password access, or submit action in the agent-facing contract.

### Browser runtime state

`BrowserManager` is stored in `AppState` and owns runtime-only tab metadata:

```text
BrowserTab
  id
  webviewLabel
  currentUrl
  title
  loadState
  platform?
  createdAt

BrowserBounds
  x
  y
  width
  height
  scaleFactor
```

Do not persist:

- cookies;
- passwords;
- session tokens;
- raw HTML;
- full page screenshots;
- arbitrary page scripts;
- private browser history beyond user-approved Goalbar captures.

The OS webview profile retains website session data. “Clear browser data” calls Tauri’s webview browsing-data API and requires confirmation.

### Page adapters

Add `BrowserPageAdapter` implementations for:

- `XBrowserAdapter`
- `RedditBrowserAdapter`
- `LinkedInBrowserAdapter`

Adapters provide:

- host matching;
- URL → page-kind classification;
- versioned semantic extraction script;
- normalized item parsing;
- canonical URL/remote ID recovery when present;
- login/challenge/rate-limit detection;
- current capability/policy state.

Extraction must prioritize semantic and stable data:

1. canonical and permalink URLs;
2. `article`, ARIA roles, `<time>`, visible links, and text;
3. bounded `data-*` hints only as fallback;
4. never depend on minified class names.

The generic fallback can capture explicit user selection but cannot run automated collection.

### Bounded collection algorithm

```text
1. Validate supported host and user-specified objective.
2. Require one or more hard bounds:
   - maximum items (default 50, maximum 500);
   - earliest date;
   - maximum steps (default 25, maximum 100).
3. Observe and normalize visible records.
4. Compute deterministic dedupe keys.
5. Commit a checkpoint before scrolling.
6. Stop or pause when:
   - objective bound is reached;
   - three observations produce no new records;
   - the page requests login/verification;
   - CAPTCHA, rate limit, or challenge is detected;
   - navigation leaves the supported platform host;
   - the page adapter becomes uncertain;
   - the user cancels.
7. Store only normalized records and a run summary.
```

Use the local agent only when adapter classification is uncertain or the user asks for semantic interpretation. Do not start one expensive `codex exec` process for every ordinary scroll step when the deterministic adapter can proceed.

### Ingestion data model

Create migration `0006_browser_history.sql`.

#### `ingestion_sources`

| Column               | Type          | Notes                                         |
| -------------------- | ------------- | --------------------------------------------- |
| `id`                 | TEXT PK       | UUID                                          |
| `platform`           | TEXT          | `x`, `reddit`, `linkedin`                     |
| `source_kind`        | TEXT          | `archive`, `browser`                          |
| `ownership`          | TEXT          | `own`, `reference`                            |
| `display_name`       | TEXT          | User-visible source label                     |
| `account_handle`     | TEXT nullable | Never infer if absent                         |
| `source_fingerprint` | TEXT          | File hash or browser-run stable key           |
| `metadata_json`      | TEXT          | Parser/run version and non-secret metadata    |
| `created_at`         | TEXT          | RFC 3339                                      |
| unique               |               | `(platform, source_kind, source_fingerprint)` |

#### `ingestion_runs`

| Column         | Type          | Notes                                                              |
| -------------- | ------------- | ------------------------------------------------------------------ |
| `id`           | TEXT PK       | UUID                                                               |
| `source_id`    | TEXT FK       | ingestion source                                                   |
| `status`       | TEXT          | `preview`, `running`, `paused`, `completed`, `failed`, `cancelled` |
| `provider`     | TEXT nullable | `codex`, `claude`, or null for deterministic imports               |
| `objective`    | TEXT          | Bounded user request                                               |
| `limits_json`  | TEXT          | item/date/step limits                                              |
| `counts_json`  | TEXT          | discovered/imported/skipped/failed                                 |
| `pause_reason` | TEXT nullable | Typed pause                                                        |
| `error_code`   | TEXT nullable | Never raw secret-bearing error text                                |
| timestamps     | TEXT          | started/updated/completed                                          |

#### `activity_items`

| Column                | Type          | Notes                                                                      |
| --------------------- | ------------- | -------------------------------------------------------------------------- |
| `id`                  | TEXT PK       | UUID                                                                       |
| `source_id`           | TEXT FK       | ingestion provenance                                                       |
| `platform`            | TEXT          | constrained                                                                |
| `item_kind`           | TEXT          | `post`, `comment`, `reply`, `message`, `reaction`, `connection`, `profile` |
| `ownership`           | TEXT          | `own`, `reference`                                                         |
| `direction`           | TEXT nullable | `inbound`, `outbound`                                                      |
| `remote_id`           | TEXT nullable | only when supplied                                                         |
| `canonical_url`       | TEXT nullable | HTTPS/platform allowlist                                                   |
| `author_handle`       | TEXT nullable | normalized                                                                 |
| `counterparty_handle` | TEXT nullable | normalized                                                                 |
| `body`                | TEXT          | bounded; may be empty for reactions/connections                            |
| `published_at`        | TEXT nullable | parsed source time                                                         |
| `observed_at`         | TEXT          | local collection time                                                      |
| `dedupe_key`          | TEXT          | SHA-256 over normalized identity/content                                   |
| `metadata_json`       | TEXT          | metrics/visibility/parser fields                                           |
| unique                |               | `(platform, dedupe_key)`                                                   |

#### `browser_checkpoints`

| Column             | Type          | Notes                 |
| ------------------ | ------------- | --------------------- |
| `id`               | TEXT PK       | UUID                  |
| `run_id`           | TEXT FK       | browser ingestion run |
| `step`             | INTEGER       | monotonic             |
| `url`              | TEXT          | current supported URL |
| `new_item_count`   | INTEGER       | progress              |
| `total_item_count` | INTEGER       | progress              |
| `last_item_key`    | TEXT nullable | resume hint           |
| `created_at`       | TEXT          | RFC 3339              |
| unique             |               | `(run_id, step)`      |

### Archive import pipeline

```text
Choose file in native dialog
    → opaque selection ID
    → sniff ZIP/CSV/JSON/JS container
    → compute SHA-256
    → parser registry selects platform parser
    → preview counts/warnings/date range
    → user confirms
    → transactional normalized insert/upsert
    → import summary
    → bounded history context becomes available
```

#### Required parsers

**X archive**

- ZIP archive;
- tolerate JavaScript assignment wrappers around JSON arrays;
- identify versioned members by patterns, not one exact archive filename;
- initial supported categories: posts, replies, direct-message records, profile/account metadata;
- ignore media binaries in v1 while preserving referenced media filenames/URLs in metadata;
- never execute archive JavaScript.

**LinkedIn archive**

- ZIP or individual CSV;
- normalize headers case-insensitively;
- initial supported categories: shares/posts, comments, messages, reactions, connections;
- accept missing optional category files;
- parse UTF-8 with BOM;
- preserve visibility and links as metadata.

**Reddit archive**

- ZIP or individual CSV;
- initial supported categories: posts, comments, messages, votes/reactions where present;
- tolerate optional/missing columns;
- report unsupported members in preview rather than failing the entire import.

#### Archive safety limits

- stream ZIP entries; do not unpack the whole archive to disk;
- reject unsafe member paths even though files are not extracted;
- maximum 10,000 entries;
- maximum 1 GiB archive file;
- maximum 128 MiB uncompressed per parsed data member;
- maximum 2 GiB cumulative parsed data;
- skip media/binaries in v1;
- treat CSV formulas as plain text;
- never render raw archive HTML;
- do not persist absolute source paths;
- show partial-import warnings before commit;
- all commit writes occur in one transaction;
- repeated import of the same archive is idempotent by source fingerprint and dedupe key.

### Context integration

Add `HistoryContextService` with purpose-specific, bounded queries:

- `voice_examples(limit, max_chars)` → founder-owned posts/comments only;
- `icp_evidence(limit, max_chars)` → counterpart messages/comments/connections with provenance;
- `content_examples(platform, limit, max_chars)` → founder-owned examples;
- `learning_evidence(window, limit, max_chars)` → normalized activities and available metadata.

Update existing `ContextAssembler` call sites:

- ICP generation: founder + bounded ICP evidence;
- content generation: founder + idea + accepted voice examples + platform guidance;
- weekly learning: deterministic growth score + bounded history evidence;
- reply drafting: current conversation + relevant read-only history.

Imported history never becomes an accepted ICP claim or learning automatically. Agent output remains a hypothesis/draft and the user accepts it through existing flows.

---

## Mandatory Reading

Files that MUST be read before implementing:

| Priority | File                                                |                                               Lines | Why                                                                   |
| -------- | --------------------------------------------------- | --------------------------------------------------: | --------------------------------------------------------------------- |
| P0       | `AGENTS.md`                                         |                                                 all | Local-first, token, approval, Zod, and platform honesty contract      |
| P0       | `src-tauri/src/lib.rs`                              |                                               16–64 | Tauri setup and registered command entry point                        |
| P0       | `src-tauri/src/app_state.rs`                        |                                               12–47 | Runtime manager ownership and test-state construction                 |
| P0       | `src-tauri/capabilities/default.json`               |                                                 all | Current capability targets all webviews in `main`; must be narrowed   |
| P0       | `src-tauri/build.rs`                                |                                                 all | Must adopt explicit app-command manifest restrictions                 |
| P0       | `src-tauri/tauri.conf.json`                         |                                               12–27 | Window, minimum size, and CSP                                         |
| P0       | `src-tauri/src/adapters/agent/mod.rs`               |                                              22–132 | Provider-neutral agent adapter and registry pattern                   |
| P0       | `src-tauri/src/conductor/runner.rs`                 |                                               11–60 | Structured agent run/cancellation pattern                             |
| P0       | `src-tauri/src/conductor/context.rs`                |                                                3–28 | Existing bounded-context contract                                     |
| P0       | `src-tauri/src/error.rs`                            |                                                4–80 | Typed Rust → frontend error contract                                  |
| P0       | `src-tauri/src/db/migrations.rs`                    |                                                7–81 | Immutable embedded migration/checksum pattern                         |
| P0       | `src-tauri/migrations/0004_platforms.sql`           |                                                 all | Existing operational platform/content schema                          |
| P0       | `src-tauri/migrations/0005_relationships_jobs.sql`  |                                                 all | Existing conversations and retryable jobs                             |
| P0       | `src-tauri/src/services/sync.rs`                    |                                               22–82 | Transactional normalized ingestion pattern                            |
| P0       | `src-tauri/src/services/data.rs`                    |                                       17–71, 99–149 | Artifact/export/reset behavior                                        |
| P0       | `src/components/AppShell.tsx`                       |                                                7–50 | Existing routing/navigation shell                                     |
| P0       | `src/styles/globals.css`                            |                           3–154, 362–541, 1071–1102 | Existing tokens, shell, controls, responsive and reduced-motion rules |
| P0       | `src/lib/tauri.ts`                                  |                                                6–34 | Mandatory Zod-validated Tauri boundary                                |
| P1       | `src/features/settings/SettingsPage.tsx`            |                             29–45, 182–313, 316–397 | Existing API and local-data UX patterns                               |
| P1       | `src/features/create/CreatePage.tsx`                |                                               43–99 | Provider selection, mutations, approval, and publish flow             |
| P1       | `src-tauri/src/commands/onboarding.rs`              |                                               54–88 | ICP context and structured-task pattern                               |
| P1       | `src-tauri/src/commands/growth.rs`                  |                                               28–48 | Learning task pattern                                                 |
| P1       | `src-tauri/src/services/content.rs`                 |                                               20–64 | Service + repository + Conductor pattern                              |
| P1       | `src-tauri/src/services/publishing.rs`              |                                              80–145 | External-write transaction and audit pattern                          |
| P1       | `src-tauri/src/services/communication.rs`           |                                              60–145 | Exact-revision approval and send boundary                             |
| P1       | `src-tauri/src/db/repositories/job.rs`              |                                              24–110 | Checkpoint/retry transaction pattern                                  |
| P1       | `src-tauri/src/db/repositories/audit.rs`            |                                               18–35 | Audit event pattern                                                   |
| P1       | `src-tauri/src/validation.rs`                       |                                                7–50 | Text and URL validation                                               |
| P1       | `src/schemas/common.ts`                             |                                                 all | Shared Zod enum/error style                                           |
| P1       | `src/app/bootstrap.tsx`                             |                                                7–46 | Preview-mode fallback pattern                                         |
| P1       | `src/lib/query-keys.ts`                             |                                                 all | React Query key convention                                            |
| P1       | `src-tauri/tests/agent_contract.rs`                 |                                                 all | Agent contract test style                                             |
| P1       | `src-tauri/tests/platform_contract.rs`              |                                                 all | Adapter contract test style                                           |
| P1       | `e2e/navigation.spec.ts`                            |                                                 all | Browser-level navigation test style                                   |
| P2       | `README.md`                                         | product loop, ICP, voice, history-relevant sections | Product intent                                                        |
| P2       | `docs/architecture.md`                              |                                                 all | Existing system boundary statement                                    |
| P2       | `docs/capability-matrix.md`                         |                                                 all | Honest platform capability states                                     |
| P2       | `docs/platform-access.md`                           |                                                 all | Official API access remains a separate path                           |
| P2       | `../codex/codex-rs/utils/cli/src/shared_options.rs` |                                                8–63 | Current Codex CLI image and sandbox options; reference only           |
| P2       | `../codex/codex-rs/exec/src/cli.rs`                 |                                               19–86 | Current Codex exec structured-output/config behavior; reference only  |

---

## External Documentation

| Topic                        | Source                                                                                                                  | Key Takeaway                                                                                      |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Tauri child webviews         | https://docs.rs/tauri/2.11.5/tauri/webview/struct.Webview.html                                                          | Child webview construction is available only with Tauri’s `unstable` feature                      |
| Tauri WebviewBuilder         | https://docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html                                                   | `on_navigation`, initialization scripts, external URLs, and data directory behavior               |
| Tauri capabilities           | https://v2.tauri.app/reference/acl/capability/                                                                          | In a multiwebview window, target the local webview by label and omit `windows`                    |
| Tauri security lifecycle     | https://v2.tauri.app/security/capabilities/                                                                             | Remote content must not receive local IPC permissions                                             |
| Tauri dialog plugin          | https://v2.tauri.app/plugin/dialog/                                                                                     | Native file selection; prefer a dedicated command for stronger scope control                      |
| Tauri window customization   | https://v2.tauri.app/learn/window-customization/                                                                        | Preserve native macOS behavior with overlay/transparent title bar instead of removing decorations |
| Codex built-in browser       | https://learn.chatgpt.com/docs/browser                                                                                  | Browser/Computer Use exists in the desktop app, not Codex CLI                                     |
| X account archive            | https://help.x.com/en/managing-your-account/accessing-your-x-data                                                       | Machine-readable personal archive can include posts, DMs, media, and account data                 |
| LinkedIn account archive     | https://www.linkedin.com/help/linkedin/answer/a1339364/downloading-your-account-data                                    | Personal archive categories include shares, comments, messages, reactions, and connections        |
| Reddit account archive       | https://support.reddithelp.com/hc/en-us/articles/360043048352-How-do-I-request-a-copy-of-my-Reddit-data-and-information | Users can request posts, comments, votes, and other personal data                                 |
| X automation rules           | https://help.x.com/en/rules-and-policies/x-automation                                                                   | Non-API website scripting can trigger enforcement                                                 |
| LinkedIn prohibited software | https://www.linkedin.com/help/linkedin/answer/a1341387                                                                  | Third-party scraping and automation are prohibited                                                |
| Reddit User Agreement        | https://redditinc.com/policies/user-agreement                                                                           | Automated collection/scraping is restricted unless permitted                                      |

### Research Findings

```text
KEY_INSIGHT: Tauri can attach remote child webviews to the main window and control
their bounds, navigation, evaluation, and browsing data.
APPLIES_TO: Integrated browser pane and tab lifecycle.
GOTCHA: Multiwebview remains behind Tauri's `unstable` feature and must be
validated on macOS, Windows WebView2, and Linux WebKitGTK.
```

```text
KEY_INSIGHT: A capability targeting `windows: ["main"]` applies to every child
webview in that window.
APPLIES_TO: Remote-page isolation.
GOTCHA: Change the capability to `webviews: ["main"]`, omit `windows`, and
register an explicit app-command manifest in build.rs.
```

```text
KEY_INSIGHT: `eval_with_callback` can return serialized results from the remote
webview to Rust without enabling remote-to-Rust IPC.
APPLIES_TO: DOM-first BrowserObservation.
GOTCHA: Keep extraction scripts origin-guarded, bounded, and exception-safe;
Windows has callback/exception caveats.
```

```text
KEY_INSIGHT: Codex desktop Browser can click/type/inspect, but Codex CLI does not
expose that built-in browser.
APPLIES_TO: Browser Conductor integration.
GOTCHA: Reuse the current structured CLI adapter, not undocumented desktop APIs.
```

```text
KEY_INSIGHT: Official personal archives are broader and more complete than
virtualized feed scrolling.
APPLIES_TO: History bootstrap and voice/ICP evidence.
GOTCHA: Archive formats and optional categories evolve; parsers must be
tolerant, versioned, previewable, and fixture-driven.
```

---

## Unified Discovery Table

| Category          | File:Lines                                             | Pattern                                                      | Key Snippet                            |
| ----------------- | ------------------------------------------------------ | ------------------------------------------------------------ | -------------------------------------- |
| Entry point       | `src-tauri/src/lib.rs:20-64`                           | Builder setup, managed state, explicit handlers              | `.setup(...).invoke_handler(...)`      |
| Runtime state     | `src-tauri/src/app_state.rs:12-33`                     | Cloneable manager container                                  | `pub struct AppState { ... }`          |
| Frontend boundary | `src/lib/tauri.ts:8-22`                                | Validate input and output                                    | `invokeValidated(...)`                 |
| Naming            | `src-tauri/src/adapters/agent/mod.rs:22-91`            | PascalCase Rust domain types, traits named by responsibility | `pub trait AgentAdapter`               |
| Serialization     | `src-tauri/src/adapters/agent/mod.rs:54-80`            | `camelCase` output, enum-specific casing                     | `#[serde(rename_all = "camelCase")]`   |
| Errors            | `src-tauri/src/error.rs:46-80`                         | `AppError` → `CommandError` with recovery                    | `impl From<AppError> for CommandError` |
| Validation        | `src-tauri/src/validation.rs:25-50`                    | Parse, scheme-check, exact-host allowlist                    | `allowlisted_external_url`             |
| Logging           | `src-tauri/src/services/scheduler.rs:10-17`            | Structured tracing, no payloads                              | `tracing::warn!(error = %error, ...)`  |
| Repository        | `src-tauri/src/db/repositories/job.rs:24-40`           | Repository owns bound SQL                                    | `sqlx::query(...).bind(...)`           |
| Transactions      | `src-tauri/src/services/sync.rs:59-81`                 | Batch writes + cursor commit atomically                      | `let mut transaction = ...`            |
| Idempotency       | `src-tauri/src/services/sync.rs:62-79`                 | SQLite conflict handling                                     | `ON CONFLICT ... DO UPDATE`            |
| Agent task        | `src-tauri/src/conductor/task.rs:6-18`                 | Schema generated from Rust type                              | `structured_task<T: JsonSchema>`       |
| Agent context     | `src-tauri/src/conductor/context.rs:13-27`             | Deterministic char budget                                    | `ContextAssembler::assemble`           |
| Cancellation      | `src-tauri/src/conductor/runner.rs:25-59`              | Cancellation token keyed by job UUID                         | `cancellations.insert(job_id, ...)`    |
| Approval          | `src-tauri/src/services/communication.rs:68-96`        | Hash exact payload, consume once                             | `approval does not match...`           |
| Audit             | `src-tauri/src/db/repositories/audit.rs:18-35`         | Typed kind + bounded JSON detail                             | `record(kind, subject_type, ...)`      |
| UI data           | `src/features/create/CreatePage.tsx:44-99`             | React Query mutation + preview fallback                      | `isTauriRuntime() ? ... : ...`         |
| UI error          | `src/features/settings/SettingsPage.tsx:305-310`       | Inline recovery area                                         | `.inline-error`                        |
| UI shell          | `src/components/AppShell.tsx:7-50`                     | Route list + `NavLink` + `Outlet`                            | `navigation.map(...)`                  |
| UI tests          | `src/features/onboarding/OnboardingFlow.test.tsx:9-31` | User-visible behavior with Testing Library                   | `userEvent.setup()`                    |
| Rust unit tests   | `src-tauri/src/db/repositories/job.rs:113-162`         | In-memory SQLite and semantic assertions                     | `Database::in_memory()`                |
| Contract tests    | `src-tauri/tests/platform_contract.rs:23-96`           | Real adapter + mocked HTTP                                   | `MockServer::start()`                  |
| E2E               | `e2e/navigation.spec.ts:3-10`                          | Role/text navigation assertions                              | `getByRole(...).click()`               |

---

## Patterns to Mirror

### NAMING_CONVENTION

```rust
// SOURCE: src-tauri/src/adapters/agent/mod.rs:83-92
#[async_trait]
pub trait AgentAdapter: Debug + Send + Sync {
    fn provider(&self) -> AgentProvider;
    async fn status(&self) -> AgentStatus;
    async fn run(
        &self,
        task: &StructuredAgentTask,
        cancellation: CancellationToken,
    ) -> AppResult<AgentResult>;
}
```

Mirror with `BrowserPageAdapter`, `HistoryArchiveParser`, `BrowserManager`, `HistoryImportService`, and `HistoryRepository`.

### SERIALIZATION_CONVENTION

```rust
// SOURCE: src-tauri/src/adapters/agent/mod.rs:64-80
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredAgentTask {
    pub task_kind: String,
    pub prompt: String,
    pub context: Value,
    pub output_schema: Value,
    pub timeout_seconds: u64,
}
```

All browser/history command inputs must use `deny_unknown_fields`. All outputs must use `camelCase`. TypeScript must mirror them with strict Zod 4 schemas.

### ERROR_HANDLING

```rust
// SOURCE: src-tauri/src/error.rs:46-79
impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        let (code, recovery) = match &error {
            AppError::Validation(_) => ("validation", Some("Review the highlighted fields.")),
            AppError::NotFound(_) => ("not_found", Some("Refresh the local data and try again.")),
            AppError::Unsupported(_) => ("unsupported", None),
            // ...
        };
        Self {
            code,
            message: error.to_string(),
            recovery: recovery.map(str::to_owned),
        }
    }
}
```

Add browser-specific `AppError` variants only if their recovery differs materially. Prefer existing `Unsupported`, `Permission`, `Timeout`, `Cancelled`, `Validation`, and `Platform`.

### LOGGING_PATTERN

```rust
// SOURCE: src-tauri/src/services/scheduler.rs:10-17
pub fn start(state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = process_one(&state).await {
                tracing::warn!(error = %error, "background job pass failed");
            }
        }
    });
}
```

Browser/history logs may include run ID, platform, action kind, counts, duration, and typed error code. They must never include page bodies, selected text, URLs with query strings, cookies, archive paths, tokens, or private message content.

### REPOSITORY_PATTERN

```rust
// SOURCE: src-tauri/src/db/repositories/job.rs:24-40
impl JobRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn enqueue(&self, kind: &str, payload: &Value) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        // bound SQL parameters
        Ok(id)
    }
}
```

`HistoryRepository` owns all ingestion SQL. Services must not duplicate insert/upsert queries except when an existing service is intentionally updated to create a merged read model.

### TRANSACTION_PATTERN

```rust
// SOURCE: src-tauri/src/services/sync.rs:59-82
let now = Utc::now().to_rfc3339();
let mut transaction = self.pool.begin().await?;
for item in &page.items {
    sqlx::query("INSERT ... ON CONFLICT ...")
        .bind(...)
        .execute(&mut *transaction)
        .await?;
}
transaction.commit().await?;
```

Archive commits and each browser checkpoint batch use one transaction. Preview never writes activity records.

### SERVICE_PATTERN

```rust
// SOURCE: src-tauri/src/services/content.rs:20-26
impl ContentService {
    pub fn new(conductor: Conductor, pool: sqlx::SqlitePool) -> Self {
        Self {
            conductor,
            repository: ContentRepository::new(pool),
        }
    }
}
```

`BrowserConductorService` receives `Conductor`, `BrowserManager`, and `HistoryRepository`. `HistoryImportService` receives parser registry and repository.

### FRONTEND_COMMAND_PATTERN

```ts
// SOURCE: src/lib/tauri.ts:8-22
export async function invokeValidated<Input, Output>(
  command: string,
  args: Record<string, unknown>,
  inputSchema: ZodType<Input>,
  outputSchema: ZodType<Output>,
): Promise<Output> {
  const inputKey = Object.keys(args)[0]
  const parsedInput = inputSchema.parse(inputKey ? args[inputKey] : {})
  const validatedArgs = inputKey ? { ...args, [inputKey]: parsedInput } : args
  const result = await invoke<unknown>(command, validatedArgs)
  return outputSchema.parse(result)
}
```

No browser/history component may call raw `invoke`.

### TEST_STRUCTURE

```rust
// SOURCE: src-tauri/src/db/repositories/job.rs:120-127
#[tokio::test]
async fn leases_each_job_once() {
    let database = Database::in_memory().await.expect("database");
    let repository = JobRepository::new(database.pool().clone());
    let id = repository
        .enqueue("sync", &json!({"platform": "x"}))
        .await
        .expect("enqueue");
    // semantic assertions
}
```

Use fixture archives and in-memory SQLite for parser/import contracts. Do not require live platform accounts in CI.

---

## Files to Change

The exact split may be adjusted when a module becomes too large, but implementation must retain these responsibilities.

### Frontend

| File                                               | Action | Justification                                                |
| -------------------------------------------------- | ------ | ------------------------------------------------------------ |
| `src/App.tsx`                                      | UPDATE | Add `/browser`; preserve existing routes                     |
| `src/components/AppShell.tsx`                      | UPDATE | Codex-inspired workbench shell and compact navigation        |
| `src/components/WorkbenchTitlebar.tsx`             | CREATE | Route title, drag region, local status; native-safe          |
| `src/components/PaneDivider.tsx`                   | CREATE | Keyboard/pointer accessible browser split                    |
| `src/styles/globals.css`                           | UPDATE | Replace dark theme with light tokens and new layout          |
| `src/features/browser/BrowserPage.tsx`             | CREATE | Browser workspace composition                                |
| `src/features/browser/BrowserToolbar.tsx`          | CREATE | Tabs, URL, back, forward, reload, status                     |
| `src/features/browser/BrowserConductorPanel.tsx`   | CREATE | Objective, provider, capture/run controls, progress          |
| `src/features/browser/HistoryImportPanel.tsx`      | CREATE | Select, preview, confirm, summary                            |
| `src/features/browser/useBrowserSurface.ts`        | CREATE | Bounds observer, lifecycle, events, cleanup                  |
| `src/features/browser/BrowserPage.test.tsx`        | CREATE | Preview/runtime UI behavior                                  |
| `src/features/browser/HistoryImportPanel.test.tsx` | CREATE | Preview/confirm/error behavior                               |
| `src/schemas/browser.ts`                           | CREATE | Strict browser contracts                                     |
| `src/schemas/history.ts`                           | CREATE | Strict import/history contracts                              |
| `src/lib/query-keys.ts`                            | UPDATE | Browser tabs/runs/history keys                               |
| `src/lib/tauri.ts`                                 | UPDATE | Optional event helper only if it preserves validation        |
| `src/test/fixtures.ts`                             | UPDATE | Browser/history fixtures                                     |
| `src/features/settings/SettingsPage.tsx`           | UPDATE | Label official APIs optional; add clear-browser-data control |
| `src/App.test.tsx`                                 | UPDATE | New shell/nav assertions                                     |
| `e2e/navigation.spec.ts`                           | UPDATE | Browser route and light workbench                            |
| `e2e/browser-workbench.spec.ts`                    | CREATE | Web-preview fallback and import affordances                  |
| `package.json`                                     | UPDATE | Add official Tauri dialog JS plugin                          |
| `pnpm-lock.yaml`                                   | UPDATE | Lock dependency                                              |

### Rust/Tauri

| File                                                | Action | Justification                                            |
| --------------------------------------------------- | ------ | -------------------------------------------------------- |
| `src-tauri/Cargo.toml`                              | UPDATE | Enable Tauri `unstable`; add dialog, ZIP, and CSV crates |
| `src-tauri/Cargo.lock`                              | UPDATE | Lock dependencies                                        |
| `src-tauri/build.rs`                                | UPDATE | Explicit app command manifest                            |
| `src-tauri/tauri.conf.json`                         | UPDATE | macOS-compatible titlebar baseline and capability IDs    |
| `src-tauri/tauri.macos.conf.json`                   | CREATE | Overlay title bar and traffic-light position             |
| `src-tauri/capabilities/default.json`               | UPDATE | Target local `main` webview, not `main` window           |
| `src-tauri/src/lib.rs`                              | UPDATE | Register plugin, browser/history modules and commands    |
| `src-tauri/src/app_state.rs`                        | UPDATE | Own BrowserManager and HistorySelectionManager           |
| `src-tauri/src/error.rs`                            | UPDATE | Typed browser/import recovery where needed               |
| `src-tauri/src/validation.rs`                       | UPDATE | Browser URL, bounds, run limits, archive limits          |
| `src-tauri/src/domain/mod.rs`                       | UPDATE | Export browser/history domains                           |
| `src-tauri/src/domain/browser.rs`                   | CREATE | Tabs, observations, actions, policies, run status        |
| `src-tauri/src/domain/history.rs`                   | CREATE | Sources, activity, previews, summaries                   |
| `src-tauri/src/browser/mod.rs`                      | CREATE | Browser subsystem exports                                |
| `src-tauri/src/browser/manager.rs`                  | CREATE | Child-webview lifecycle and callbacks                    |
| `src-tauri/src/browser/policy.rs`                   | CREATE | Navigation/action/run safety gates                       |
| `src-tauri/src/browser/extraction.rs`               | CREATE | Eval callback and bounded semantic snapshot              |
| `src-tauri/src/browser/adapters/mod.rs`             | CREATE | BrowserPageAdapter registry                              |
| `src-tauri/src/browser/adapters/x.rs`               | CREATE | X URL/page normalization                                 |
| `src-tauri/src/browser/adapters/reddit.rs`          | CREATE | Reddit URL/page normalization                            |
| `src-tauri/src/browser/adapters/linkedin.rs`        | CREATE | LinkedIn URL/page normalization                          |
| `src-tauri/browser-scripts/semantic-observation.js` | CREATE | Generic origin-guarded semantic snapshot                 |
| `src-tauri/browser-scripts/selection-capture.js`    | CREATE | Explicit selection capture                               |
| `src-tauri/src/conductor/mod.rs`                    | UPDATE | Export Browser Conductor                                 |
| `src-tauri/src/conductor/browser.rs`                | CREATE | Bounded action loop and provider fallback                |
| `src-tauri/src/conductor/prompt.rs`                 | UPDATE | Browser decision and history interpretation prompts      |
| `src-tauri/src/commands/mod.rs`                     | UPDATE | Export browser/history commands                          |
| `src-tauri/src/commands/browser.rs`                 | CREATE | Local UI command boundary                                |
| `src-tauri/src/commands/history.rs`                 | CREATE | Select/preview/commit/list boundary                      |
| `src-tauri/src/services/mod.rs`                     | UPDATE | Export browser/history services                          |
| `src-tauri/src/services/browser.rs`                 | CREATE | Collection/capture orchestration                         |
| `src-tauri/src/services/history.rs`                 | CREATE | Import preview/commit/context                            |
| `src-tauri/src/db/repositories/mod.rs`              | UPDATE | Export history repository                                |
| `src-tauri/src/db/repositories/history.rs`          | CREATE | Provenance, dedupe, checkpoint, read-model queries       |
| `src-tauri/src/db/migrations.rs`                    | UPDATE | Embed immutable migration 6                              |
| `src-tauri/migrations/0006_browser_history.sql`     | CREATE | New ingestion schema                                     |
| `src-tauri/src/services/data.rs`                    | UPDATE | Export/reset ingestion tables, never browser secrets     |
| `src-tauri/src/commands/onboarding.rs`              | UPDATE | Add bounded history evidence to ICP context              |
| `src-tauri/src/services/content.rs`                 | UPDATE | Add accepted/bounded voice examples                      |
| `src-tauri/src/commands/growth.rs`                  | UPDATE | Add bounded history evidence                             |
| `src-tauri/src/commands/inbox.rs`                   | UPDATE | Read-only historical relationship context                |

### Archive adapters/tests/docs

| File                                         | Action | Justification                                                 |
| -------------------------------------------- | ------ | ------------------------------------------------------------- |
| `src-tauri/src/adapters/history/mod.rs`      | CREATE | Parser trait and registry                                     |
| `src-tauri/src/adapters/history/x.rs`        | CREATE | X archive parser                                              |
| `src-tauri/src/adapters/history/linkedin.rs` | CREATE | LinkedIn CSV/ZIP parser                                       |
| `src-tauri/src/adapters/history/reddit.rs`   | CREATE | Reddit CSV/ZIP parser                                         |
| `src-tauri/tests/history_import_contract.rs` | CREATE | Fixture parser and idempotency contracts                      |
| `src-tauri/tests/browser_policy_contract.rs` | CREATE | Navigation/action isolation contracts                         |
| `src-tauri/tests/migrations.rs`              | UPDATE | New constraints and migration-6 preservation                  |
| `src-tauri/tests/fixtures/history/*`         | CREATE | Small synthetic/redacted ZIP/CSV/JS fixtures                  |
| `README.md`                                  | UPDATE | Browser/history workflow and privacy model                    |
| `SECURITY.md`                                | UPDATE | Remote content, prompt injection, archive, and cookie threats |
| `docs/architecture.md`                       | UPDATE | Browser and ingestion boundaries                              |
| `docs/capability-matrix.md`                  | UPDATE | Distinguish API, manual browser, and bounded collection       |
| `docs/platform-access.md`                    | UPDATE | State that local browser is not API parity                    |
| `docs/browser-conductor.md`                  | CREATE | Tool contract, safety states, operational behavior            |
| `docs/history-import.md`                     | CREATE | Supported categories, limits, and archive instructions        |
| `docs/live-test-runbook.md`                  | UPDATE | Manual signed-in browser verification                         |

---

## NOT Building

- Reuse or reverse engineering of Codex desktop’s private Browser implementation.
- A hosted browser, proxy, telemetry service, or Goalbar cloud account.
- Full Chrome embedding, Chrome extensions, password-manager import, or normal-browser profile theft.
- A promise to collect an entire feed/account history through scrolling.
- Background or unattended platform scraping.
- CAPTCHA, challenge, paywall, login, rate-limit, or technical-control bypass.
- Automated following/unfollowing, connection requests, likes, reactions, or unsolicited DMs.
- Automated final Publish/Send clicks.
- A generic agent-facing click or arbitrary-JavaScript tool.
- Raw cookie/token/password access from React, Codex, or Claude.
- Raw HTML, screenshot, network-log, or archive-media retention by default.
- Screenshot-coordinate Computer Use in v1.
- Mobile browser support.
- Removal of existing official X/Reddit/LinkedIn API adapters.
- Automatic acceptance of ICP hypotheses, voice rules, or growth learnings.
- Copying Codex/OpenAI branding or visual assets.

---

## Step-by-Step Tasks

### Task 1: Establish the secure multiwebview foundation

- **ACTION**: Add Tauri child-webview support and close the capability leak before loading any remote URL.
- **IMPLEMENT**:
  - enable `tauri` feature `unstable`;
  - replace capability target `windows: ["main"]` with `webviews: ["main"]`;
  - do not define any capability matching `browser-*`;
  - update `build.rs` with `AppManifest::commands` listing every registered application command;
  - add dialog plugin initialization;
  - create one proof child webview from Rust using a remote HTTPS URL;
  - enforce `https` plus explicit permitted local preview URLs; reject `file:`, `data:`, `javascript:`, and unknown custom schemes;
  - add `on_navigation`, `on_page_load`, title-change, and new-window behavior;
  - route `window.open` to the same/new controlled browser tab rather than the OS without user visibility;
  - store no cookie values; expose only clear-all-browser-data.
- **MIRROR**: `src-tauri/src/lib.rs:20-64`, `src-tauri/src/app_state.rs:12-47`, `src-tauri/src/validation.rs:25-50`.
- **IMPORTS**: `tauri::{AppHandle, Manager, Webview, WebviewBuilder, WebviewUrl}`, `url::Url`, `tokio::sync`, `uuid`.
- **GOTCHA**:
  - Tauri multiwebview is unstable;
  - creating webviews in synchronous commands can deadlock on Windows;
  - capability `windows: ["main"]` would grant local permissions to all child webviews;
  - remote-page content is untrusted even after the user allows a host.
- **VALIDATE**:
  - `cargo check --all-features`;
  - browser label has no matched capability;
  - a remote page cannot invoke `get_bootstrap_state`;
  - disallowed schemes fail with typed recovery;
  - child view hides cleanly when leaving `/browser`.

### Task 2: Replace the dark shell with the Codex-inspired Goalbar workbench

- **ACTION**: Apply the light structural redesign across the application.
- **IMPLEMENT**:
  - replace global dark tokens/gradients with the documented light tokens;
  - add compact sidebar, route header/titlebar, active nav, status footer;
  - add Browser navigation between Today and Create;
  - add macOS overlay configuration with native traffic lights;
  - retain native title bars on Windows/Linux;
  - convert large serif headings/cards to compact sans-serif surfaces;
  - preserve all accessible labels and route behavior;
  - add collapsible sidebar and conductor pane at narrower widths;
  - implement keyboard-accessible pane divider;
  - preserve reduced-motion behavior.
- **MIRROR**: `src/components/AppShell.tsx:7-50`, `src/styles/globals.css:63-154`, UI primitives in `src/components/ui/`.
- **IMPORTS**: React Router, Lucide icons, existing `cn`, existing `Badge`/`Button`.
- **GOTCHA**:
  - do not use temporary screenshot assets;
  - native traffic-light positioning is macOS-specific;
  - child-webview coordinates must include titlebar/sidebar/pane geometry;
  - do not regress current 960 px minimum window support.
- **VALIDATE**:
  - existing routes remain reachable;
  - keyboard focus is visible;
  - no horizontal overflow at 960, 1280, 1440, and 1920 px;
  - light-theme contrast is acceptable;
  - screenshot comparison is structurally similar to the reference while clearly branded Goalbar.

### Task 3: Define strict browser and history contracts

- **ACTION**: Add mirrored Rust domain types and Zod schemas before commands/UI.
- **IMPLEMENT**:
  - browser tab, bounds, load state, page kind, platform, observation block, action, policy state, run limits, run progress, pause reason;
  - history selection token, preview, category count, warning, activity item, import result, overview;
  - use `deny_unknown_fields` for command inputs and strict Zod objects;
  - use existing `Platform` enum;
  - version observation and parser schemas.
- **MIRROR**: `src-tauri/src/adapters/agent/mod.rs:22-80`, `src/schemas/common.ts:1-20`.
- **IMPORTS**: `serde`, `schemars`, `chrono`, `uuid`, `zod`.
- **GOTCHA**:
  - distinguish capability states from run states;
  - timestamps are RFC 3339 strings at the Tauri boundary;
  - URLs may be absent, but never accept malformed values when present.
- **VALIDATE**:
  - Rust serialization fixtures parse through Zod;
  - malformed/unknown input fields are rejected;
  - schema tests cover all enum values.

### Task 4: Implement BrowserManager and frontend surface lifecycle

- **ACTION**: Add real browser tabs with a React-rendered toolbar and native child webviews.
- **IMPLEMENT**:
  - create, list, activate, navigate, reload, back, forward, show, hide, close, and clear-data commands;
  - cap browser tabs at 5;
  - share local website session data while keeping Goalbar data separate;
  - use a `ResizeObserver` and `requestAnimationFrame` coalescing to send logical bounds;
  - update URL/title/load state from Rust events;
  - keep the address bar honest during redirects;
  - display browser engine/readiness state;
  - preview mode renders a safe browser placeholder for Vitest/Playwright.
- **MIRROR**: `src/lib/tauri.ts:8-34`, `src/app/bootstrap.tsx:39-46`, React Query patterns in current features.
- **IMPORTS**: `@tanstack/react-query`, Tauri event APIs where required, Zod schemas.
- **GOTCHA**:
  - child webviews are native surfaces layered above React;
  - hiding/closing must happen before route teardown;
  - bounds must use logical pixels consistently;
  - the frontend must never construct remote webviews directly.
- **VALIDATE**:
  - open X, Reddit, and LinkedIn home pages manually;
  - switch between two tabs without losing session;
  - resize the app and divider without stale overlays;
  - navigate away and confirm no remote surface remains visible;
  - clear browser data requires confirmation and signs sessions out.

### Task 5: Add semantic observation and explicit capture

- **ACTION**: Read bounded visible state without enabling remote IPC.
- **IMPLEMENT**:
  - generic semantic observation script;
  - selection capture script;
  - platform adapters for host/page detection and normalization;
  - call scripts only from Rust via `eval_with_callback`;
  - cap block count, per-block text, total text, link count, and nesting depth;
  - strip query/fragment from stored URLs unless required for canonical identity;
  - sanitize control characters and normalize whitespace;
  - compute dedupe keys in Rust;
  - show capture preview before persistence;
  - mark records `own` or `reference` explicitly.
- **MIRROR**: Rust validation and service/repository patterns, not arbitrary frontend JavaScript injection.
- **IMPORTS**: `sha2`, `url`, `serde_json`, existing validation helpers.
- **GOTCHA**:
  - do not expose `cookies()` or browser internals;
  - cross-origin frames remain unreadable and should appear as bounded unavailable regions;
  - virtualized feeds remove old elements, so each batch must persist before scroll;
  - treat all page text as prompt-injection-capable untrusted data.
- **VALIDATE**:
  - fixture DOM tests for platform URL normalization;
  - explicit selection capture works;
  - oversized DOM content truncates deterministically;
  - page text containing “ignore previous instructions” is stored as data, never executed as instruction.

### Task 6: Add the ingestion schema and repository

- **ACTION**: Create migration 6 and repository APIs for archive/browser provenance.
- **IMPLEMENT**:
  - create four tables and indexes exactly as designed;
  - add `HistoryRepository` methods for source upsert, run lifecycle, batch item upsert, checkpoint, overview, and bounded context;
  - use deterministic SHA-256 dedupe keys;
  - add a union/read-model query that can combine `activity_items` with existing `remote_content` for founder-owned content without rewriting operational tables;
  - include new tables in reset/export;
  - export normalized data and provenance but never local archive paths or browser session data.
- **MIRROR**: `src-tauri/src/db/migrations.rs:13-79`, `JobRepository`, `SyncService`.
- **IMPORTS**: `sqlx`, `chrono`, `uuid`, `serde_json`, `sha2`.
- **GOTCHA**:
  - never edit migrations 1–5;
  - migration SQL must be safe on existing databases;
  - nullable source fields require deterministic fallback dedupe inputs;
  - raw SQL dynamic table names remain limited to the existing hardcoded reset list.
- **VALIDATE**:
  - fresh and existing fixture databases migrate;
  - duplicate records do not multiply;
  - deleting an ingestion source cascades only its ingestion records;
  - operational API records remain unchanged.

### Task 7: Implement bounded Browser Conductor runs

- **ACTION**: Add the user-initiated observe/capture/scroll/pause/stop loop.
- **IMPLEMENT**:
  - create a run only after user confirms objective and limits;
  - deterministic platform collector is primary;
  - call the selected agent only for semantic decision fallback;
  - pass untrusted page content in a clearly delimited data field;
  - persist a checkpoint before every scroll;
  - stream progress events to local `main` webview;
  - support cancellation through existing cancellation/job patterns;
  - pause on challenge, login, unsupported page, host change, policy state, or uncertainty;
  - resume only after explicit user action;
  - emit audit events with counts/IDs only.
- **MIRROR**: `Conductor::run`, `ContextAssembler`, `JobRepository`, scheduler error codes.
- **IMPORTS**: existing conductor/domain/job modules, `tokio_util::sync::CancellationToken`.
- **GOTCHA**:
  - no generic click action;
  - current platform policies may force manual-only mode;
  - repeated CLI invocations are expensive—avoid them for deterministic steps;
  - prevent infinite scrolling with hard limits and no-new-items termination.
- **VALIDATE**:
  - fake BrowserPageAdapter reaches item, date, no-new-item, cancellation, and max-step stops;
  - checkpoints make duplicate-safe resume possible;
  - policy-disabled collection produces typed manual-capture recovery;
  - no run can continue after app route teardown without visible status.

### Task 8: Implement secure archive selection and preview

- **ACTION**: Let users choose official archive files without exposing arbitrary filesystem paths to the UI.
- **IMPLEMENT**:
  - Rust opens native file dialog;
  - store selected path in an in-memory opaque selection registry with expiry;
  - return selection ID, display filename, size, and inferred container only;
  - stream SHA-256 and enforce size limits;
  - sniff platform/parser from contents, not filename alone;
  - return preview category counts, date range, account handle when supplied, warnings, unsupported members, and estimated normalized records;
  - preview does not persist activity items.
- **MIRROR**: OAuth manager’s temporary-session concept and typed command errors.
- **IMPORTS**: `tauri-plugin-dialog`, `tokio::fs`, `sha2`, `zip`, `csv`, `tempfile` only if streaming requires it.
- **GOTCHA**:
  - user cancellation is not an error;
  - selected paths expire and never enter logs/SQLite/frontend;
  - no archive HTML or JavaScript execution;
  - protect against ZIP bombs and oversized CSV cells.
- **VALIDATE**:
  - cancel dialog leaves UI unchanged;
  - wrong-platform selection returns warning/reselection path;
  - oversized/unsafe archive fails before parsing;
  - preview is deterministic for fixtures.

### Task 9: Implement X, LinkedIn, and Reddit archive parsers

- **ACTION**: Normalize official archive records into common activity items.
- **IMPLEMENT**:
  - parser trait: `probe`, `preview`, `parse`;
  - X: strip known JS assignment envelope and parse JSON as inert bytes;
  - LinkedIn: ZIP/CSV, case-insensitive headers, BOM handling, optional files;
  - Reddit: ZIP/CSV, optional columns/categories;
  - parser version in source metadata;
  - per-row warnings with source member and row number, no private body in errors;
  - category-specific dedupe keys;
  - transactionally commit a confirmed preview;
  - import identical archive idempotently.
- **MIRROR**: Platform adapter registry and contract tests in `src-tauri/tests/platform_contract.rs`.
- **IMPORTS**: `async_trait` if parsing is async, `zip`, `csv`, `serde_json`.
- **GOTCHA**:
  - archive formats are not guaranteed stable;
  - optional categories must not fail the import;
  - timestamps/time zones vary;
  - DMs/messages are read-only history and never actionable inbox items without a connected account.
- **VALIDATE**:
  - fixture archives cover all supported categories;
  - malformed rows are counted/skipped with warnings;
  - duplicate imports are no-ops;
  - no raw source path/body appears in log or command error.

### Task 10: Integrate history into ICP, voice, content, and learning context

- **ACTION**: Make imported/captured evidence useful without auto-promoting it to truth.
- **IMPLEMENT**:
  - bounded context queries by purpose;
  - own posts/comments → voice examples;
  - counterpart messages/comments/connections → ICP evidence;
  - founder-owned platform examples → content guidance;
  - history metadata → weekly learning evidence;
  - provenance included beside each excerpt;
  - UI history overview and source filters in Browser Conductor/Memory;
  - imported DMs stay read-only and cannot call `send_reply`.
- **MIRROR**: `ContextAssembler` and current ICP/content/learning command call sites.
- **IMPORTS**: `HistoryContextService`, existing domain services.
- **GOTCHA**:
  - preserve context budgets;
  - never send whole archives to the agent;
  - default-exclude message bodies from content generation unless the user explicitly enables relevant private evidence;
  - no automatic accepted ICP or learning.
- **VALIDATE**:
  - context stays within configured budgets;
  - only allowed categories reach each task;
  - provenance survives serialization;
  - empty history preserves current behavior.

### Task 11: Add browser-assisted drafting with exact approval

- **ACTION**: Place generated drafts beside the browser without enabling automated submit.
- **IMPLEMENT**:
  - draft from current explicit capture or selected history items;
  - display target platform, ICP, provenance, and exact revision;
  - reuse existing approval hash/idempotency flow;
  - provide Copy exact draft;
  - represent `fillDraft` as a capability state and leave disabled/manual where policy requires;
  - no generic page mutation/click tool.
- **MIRROR**: `CreatePage` approval map and `PublishingService`/`CommunicationService` payload-hash enforcement.
- **IMPORTS**: existing approval schemas/services and clipboard API only through permitted local frontend behavior.
- **GOTCHA**:
  - editing invalidates prior approval;
  - copying does not mean published;
  - do not infer a remote post ID before official API confirmation or later capture.
- **VALIDATE**:
  - edited text requires reapproval;
  - copy output exactly matches approved body;
  - Browser Conductor cannot submit;
  - capability badges honestly explain manual recovery.

### Task 12: Update settings, documentation, and data lifecycle

- **ACTION**: Explain the new hierarchy and privacy model.
- **IMPLEMENT**:
  - Browser/local history presented as the initial path;
  - official APIs labeled optional for stable sync/publish/metrics;
  - clear browser data button with destructive confirmation;
  - remove browser/archives from “credentials in keyring” wording;
  - update architecture, capabilities, access ledger, security threats, import docs, and live runbook;
  - document that local-only does not override platform terms;
  - update export/reset table lists and verification docs.
- **MIRROR**: Current README and docs tone; current `DataSettings`.
- **IMPORTS**: None beyond existing UI controls.
- **GOTCHA**:
  - clearing Goalbar data and clearing browser data are distinct operations;
  - factory reset must clear browser data only when explicitly confirmed;
  - JSON export excludes cookies, tokens, selection paths, and raw archive.
- **VALIDATE**:
  - docs match actual capability states;
  - reset/export tests include new tables;
  - no claim of API parity or complete browser scraping.

### Task 13: Complete the verification matrix

- **ACTION**: Add automated and manual coverage proportional to the security risk.
- **IMPLEMENT**:
  - frontend schema/component tests;
  - Rust policy, parser, repository, migration, idempotency, and agent-loop tests;
  - synthetic fixture archives;
  - preview-mode Playwright tests;
  - manual signed-in Tauri runbook for each platform;
  - platform build checks in CI where available.
- **MIRROR**: Existing Vitest, Rust in-module/integration tests, Playwright tests, README validation commands.
- **IMPORTS**: Existing test dependencies; avoid adding a test framework unless necessary.
- **GOTCHA**:
  - web Playwright cannot exercise native child webviews;
  - live platform tests remain opt-in;
  - fixture content must be synthetic or redacted.
- **VALIDATE**: Run every command in Validation Commands and complete the manual checklist.

---

## Testing Strategy

### Unit Tests

| Test                   | Input                                       | Expected Output                       | Edge Case? |
| ---------------------- | ------------------------------------------- | ------------------------------------- | ---------- |
| Browser URL policy     | HTTPS X/Reddit/LinkedIn                     | allowed                               | No         |
| Browser URL policy     | `file:`, `javascript:`, deceptive subdomain | rejected                              | Yes        |
| Capability isolation   | local and remote labels                     | only `main` matches                   | Security   |
| Bounds validation      | normal/negative/oversized values            | accept/reject deterministically       | Yes        |
| Observation truncation | huge DOM fixture                            | bounded stable snapshot               | Yes        |
| Prompt injection text  | malicious page copy                         | serialized as data only               | Security   |
| Dedupe                 | repeated visible item                       | one activity item                     | No         |
| Stop condition         | three no-new observations                   | completed                             | Yes        |
| Run limits             | excessive requested count/steps             | validation failure/clamp per contract | Yes        |
| Cancellation           | active run                                  | cancelled + checkpoint retained       | Yes        |
| Archive probe          | valid/invalid ZIP/CSV/JS                    | correct parser/typed error            | Yes        |
| ZIP safety             | traversal/oversized/bomb-like fixture       | rejected                              | Security   |
| X JS envelope          | inert assignment-wrapped JSON               | parsed records                        | Yes        |
| LinkedIn CSV           | BOM/missing category/column order           | normalized records                    | Yes        |
| Reddit CSV             | optional columns                            | normalized with warnings              | Yes        |
| Idempotent import      | same archive twice                          | no duplicate activity items           | No         |
| Partial import         | malformed row                               | valid rows committed, warning counted | Yes        |
| Context privacy        | mixed messages/posts                        | purpose-specific bounded subset       | Security   |
| Approval revision      | changed draft after approval                | rejected                              | Security   |
| Export                 | ingestion history present                   | normalized records, no secrets/paths  | Security   |
| Reset                  | ingestion history present                   | tables cleared safely                 | No         |

### Integration Tests

| Test                        | Scope                                                            |
| --------------------------- | ---------------------------------------------------------------- |
| `history_import_contract`   | Parser registry → preview → transaction → dedupe                 |
| `browser_policy_contract`   | Navigation/action policy and capability labels                   |
| migration upgrade           | Migrations 1–5 fixture → migration 6; operational rows preserved |
| Conductor fallback          | Fake adapter + structured BrowserAction + cancellation           |
| Tauri command serialization | Rust responses accepted by Zod fixtures                          |

### Frontend Tests

- Browser route appears in navigation.
- Browser preview placeholder is accessible outside Tauri.
- Toolbar validates and submits URLs.
- Run controls require explicit limits and confirmation.
- Pause/cancel/recovery states render.
- History choose/preview/commit summary renders.
- Empty/partial/failed imports remain recoverable.
- Clear browsing data requires confirmation.
- Shell and all existing route tests pass after the light redesign.

### Edge Cases Checklist

- [ ] No local agent installed
- [ ] Codex binary exists but is broken/incompatible
- [ ] Claude installed but unauthenticated
- [ ] Browser engine unavailable
- [ ] Child webview creation fails
- [ ] Website blocks embedded webview
- [ ] Website redirects to login/SSO domain
- [ ] CAPTCHA or verification challenge
- [ ] Network offline or slow
- [ ] Cross-origin iframe content
- [ ] Empty page/selection
- [ ] Virtualized feed repeats records
- [ ] Deleted/private/unavailable posts
- [ ] Maximum tabs reached
- [ ] Window resized while tab loads
- [ ] Route changes while collection runs
- [ ] App closes during checkpoint
- [ ] Archive selection cancelled
- [ ] Wrong platform archive
- [ ] Archive exceeds limits
- [ ] ZIP path traversal or compression bomb
- [ ] CSV with BOM, multiline fields, formula text, invalid UTF-8
- [ ] Optional archive categories absent
- [ ] Duplicate archive import
- [ ] Partial parser failure
- [ ] Imported history without founder profile
- [ ] Imported message has no counterparty
- [ ] Private message excluded from unrelated prompt
- [ ] Permission denied
- [ ] User clears browser data
- [ ] User factory-resets local data

---

## Validation Commands

### Static Analysis

```bash
pnpm format:check
pnpm lint
pnpm typecheck
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```

EXPECT: Zero formatting, lint, type, or Clippy errors.

### Unit and Integration Tests

```bash
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

EXPECT: All tests pass, including new archive/browser contracts.

### Browser Preview Tests

```bash
pnpm test:e2e
```

EXPECT: New workbench/browser placeholder and every existing route pass in preview mode.

### Production Builds

```bash
pnpm build
pnpm tauri build --debug
```

EXPECT: Frontend and native debug bundle build successfully.

### Dependency Audit

```bash
pnpm audit --audit-level high
pnpm audit:rust
```

EXPECT: No unreviewed high-severity dependency issue. Preserve the documented scoped RustSec exception only if still applicable.

### Full Existing Shortcut

```bash
pnpm verify
```

EXPECT: Zero regressions.

### Database Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test migrations --all-features
cargo test --manifest-path src-tauri/Cargo.toml --test history_import_contract --all-features
```

EXPECT: Existing rows survive migration 6; fixture imports are idempotent.

### Manual Tauri Validation

- [ ] Start `pnpm tauri dev`.
- [ ] Confirm light Goalbar workbench and native titlebar behavior.
- [ ] Open Browser and load X, Reddit, and LinkedIn.
- [ ] Confirm URL remains visible through redirects.
- [ ] Sign in manually; restart app; verify expected local session persistence.
- [ ] Confirm remote page cannot invoke a Goalbar Tauri command.
- [ ] Resize the window and divider repeatedly; verify browser bounds.
- [ ] Switch routes; verify remote webview hides immediately.
- [ ] Capture explicit selection and preview normalized content.
- [ ] Start a small bounded run; cancel; resume from checkpoint.
- [ ] Verify CAPTCHA/login/rate-limit states pause.
- [ ] Verify no final Publish/Send automation exists.
- [ ] Import one synthetic archive per platform; preview and commit.
- [ ] Reimport the same archive; verify no duplicates.
- [ ] Generate an ICP hypothesis/content draft with bounded history context.
- [ ] Verify JSON export includes normalized history and no path/cookie/token.
- [ ] Clear browser data and verify website session removal.

---

## Acceptance Criteria

- [ ] Goalbar uses the documented light, Codex-inspired workbench structure while retaining Goalbar branding.
- [ ] `/browser` displays a React browser toolbar and an integrated native child webview.
- [ ] X, Reddit, and LinkedIn can be browsed and signed into locally when their sites permit the embedded engine.
- [ ] Remote child webviews match no Tauri capability and cannot invoke application commands.
- [ ] Browser URLs are always visible and unsafe schemes are blocked.
- [ ] Browser cookies/passwords/tokens are never returned to React, persisted in SQLite, logged, or sent to an agent.
- [ ] Users can explicitly capture selection/visible content with preview and provenance.
- [ ] Bounded collection has item/date/step limits, dedupe, checkpoints, cancellation, pause states, and deterministic termination.
- [ ] Current platform policy states can disable automated collection and provide manual recovery.
- [ ] No Browser Conductor action can submit, publish, send, purchase, delete, or change permissions.
- [ ] Official X, LinkedIn, and Reddit archive files can be selected, previewed, and imported through tolerant versioned parsers.
- [ ] Archive import is streamed, limited, path-safe, idempotent, and transactional.
- [ ] Imported/captured history is available through purpose-specific bounded context for ICP, voice, content, replies, and learning.
- [ ] Imported history never becomes an accepted ICP claim or learning without user confirmation.
- [ ] Existing official API adapters, publishing approval, keyring, and platform capability behavior continue to work.
- [ ] Existing and new tests pass.
- [ ] Documentation states the technical, policy, and completeness limits accurately.

## Completion Checklist

- [ ] Code follows discovered patterns
- [ ] Every Tauri input/output is mirrored by strict Zod
- [ ] Remote browser webviews have zero capabilities
- [ ] `build.rs` restricts registered app commands
- [ ] No hardcoded secrets, cookies, archive paths, or session tokens
- [ ] No raw page body/private message logging
- [ ] Migrations 1–5 remain byte-for-byte unchanged
- [ ] Migration 6 is tested on fresh and existing databases
- [ ] Browser and archive operations use typed error recovery
- [ ] Agent context remains bounded and purpose-specific
- [ ] External writes retain exact-revision approval
- [ ] UI matches the defined design tokens/layout
- [ ] Reduced-motion and keyboard behavior verified
- [ ] README/security/architecture/capability docs updated
- [ ] No unnecessary hosted or mobile scope
- [ ] Full validation matrix passes

---

## Risks

| Risk                                                     | Likelihood |   Impact | Mitigation                                                                          |
| -------------------------------------------------------- | ---------: | -------: | ----------------------------------------------------------------------------------- |
| Tauri multiwebview API changes                           |       High |     High | Isolate behind `BrowserManager`; pin lockfile; typed unavailable fallback           |
| Remote webview accidentally inherits capability          |     Medium | Critical | Target capability by `webviews: ["main"]`; explicit command manifest; contract test |
| Prompt injection from page content                       |       High |     High | Treat as data, bounded schemas, no generic click/JS, policy gate                    |
| Site blocks embedded webview/login                       |     Medium |     High | Typed blocked state; open system browser/manual capture fallback                    |
| Platform policy prohibits automation                     |       High |     High | Capability state, manual-only defaults, archive/API alternatives                    |
| DOM changes break adapters                               |       High |   Medium | Semantic extraction, versioned adapters, generic selection fallback, fixtures       |
| Virtualized feed loops/duplicates                        |       High |   Medium | Checkpoint before scroll, dedupe, hard limits, no-new-item stop                     |
| Archive format drift                                     |       High |   Medium | Probe/preview, tolerant headers/members, parser versions, warnings                  |
| Malicious/huge archive                                   |     Medium |     High | Streaming limits, no extraction/execution, safe paths, transactional writes         |
| Sensitive history reaches wrong prompt                   |     Medium | Critical | Purpose-specific HistoryContextService, private-category defaults, tests            |
| Repeated CLI calls are slow/costly                       |     Medium |   Medium | Deterministic collector primary; agent fallback only                                |
| Child webview overlays stale UI                          |     Medium |   Medium | Lifecycle hook, route cleanup, resize coalescing, manual tests                      |
| Light redesign regresses existing screens                |     Medium |   Medium | Keep markup contracts, update unit/E2E screenshots/roles, responsive matrix         |
| Installed Codex path is present but executable is broken |     Medium |   Medium | Improve readiness detection to `incompatible`; show recovery, do not start run      |

---

## Notes

- This plan deliberately separates “browser exists in the app” from “automation is permitted.” A local browser improves workflow and privacy but does not grant platform API rights or waive platform terms.
- “Complete history” refers to what an official personal archive actually contains. Each import preview must state the categories and date range found rather than asserting completeness.
- Browser collection is best treated as a current-session research tool, not the system of record.
- A future phase can add a screenshot backend behind a `BrowserCaptureBackend` trait and pass images to Codex CLI with `--image`; it must not be mixed into this MVP unless a safe cross-platform capture path is proven.
- A future plugin/MCP distribution can expose the same browser/history tools to Codex desktop. This plan keeps the core standalone and provider-neutral.

## Confidence Score

**8/10** for single-pass implementation.

The product, data, security, and test boundaries are fully specified. The remaining uncertainty is concentrated in Tauri’s unstable multiwebview behavior and live website compatibility across operating systems; both have explicit fallbacks and manual gates.
