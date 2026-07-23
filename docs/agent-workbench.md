# Founder chat and local agent workbench

The browser workspace defaults to a persistent founder chat on the left and the local browser on the right. The founder chooses Codex or Claude, then asks naturally about ICP, voice, content, or the visible page. The selected CLI must already be installed and authenticated on the machine.

Research is a chat-callable add-on rather than a separate primary interface:

1. Founder chat may answer directly from approved product context.
2. If current browser evidence is required, chat returns a schema-constrained `researchRequest`.
3. Goalbar shows the proposed objective, evidence ownership, and hard item/step limits.
4. Nothing reads or scrolls until the founder explicitly approves those exact bounds.
5. Results and proposed ICP findings return to the same conversation.

## Terminal boundary

- Interactive PTY support remains infrastructure for a future developer-console add-on; it is not the default Browser workspace.
- Rust creates, resizes, writes to, and terminates PTY processes.
- React renders transient terminal output with xterm.js and forwards only the user’s keystrokes.
- Goalbar does not persist terminal output, silently type commands, or convert terminal text into product memory.
- Closing a pane terminates its child process. Starting a Codex or Claude pane does not grant it browser cookies, platform tokens, or Tauri commands.
- Founder chat and its Research add-on use the hardened CLI runner with a minimal environment, read-only sandboxing where supported, a timeout, output cap, and JSON Schema.

## Browser research loop

1. The user opens a supported platform and signs in locally.
2. Founder chat requests the Research add-on when the answer needs current page evidence.
3. The user reviews the generated objective and hard limits, then confirms them.
4. Rust takes a normalized semantic observation and loads bounded accepted ICP context. Private messages are excluded.
5. The selected CLI returns a schema-constrained decision: grounded proposed findings plus `scroll` or `finish`.
6. Rust verifies that every evidence excerpt exists in the visible observation, persists an action trace, and performs at most one fixed scroll.
7. The user accepts or rejects proposed findings. Only accepted findings enter later ICP generation context.

This is a local control layer, not OS-wide Computer Use. Screen-wide observation, arbitrary clicking, and unattended workflows remain outside this MVP boundary.
