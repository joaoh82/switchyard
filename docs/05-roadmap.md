# 05 — Roadmap

Ordered by risk first, then by the shortest path to something usable daily. Every milestone must
pass on **Linux, macOS and Windows** before it is done — CI enforces the build, a short manual
checklist covers what CI can't see.

## M0 — Scaffold

- Tauri 2 + React + TS + Vite + bun; lint/format (clippy, rustfmt, eslint, prettier).
- GitHub Actions matrix: ubuntu / macos / windows — build, `cargo test`, frontend tests.
- Typed IPC generation wired up. Empty three-panel shell with resizable panels.

*Exit:* a signed-or-not installer artifact is produced for all three OSes on every push.

## M1 — Terminal spike (highest risk, do first)

- `pty-host` crate on `portable-pty`, with the message-shaped API (spawn / attach / write / resize /
  kill + events) and **no Tauri dependencies** — in-process for now, daemon-ready.
- xterm.js view; raw-byte channel output with batching; input; resize.
- Headless VT state in the host → snapshot on attach. Detach/re-attach across workspace switches.
- Login-shell environment resolution.
- Run a plain shell, then `claude`, in the center panel.
- WebGL renderer with automatic DOM-renderer fallback; evaluate the WebKitGTK/NVIDIA mitigations.

*Exit:* a full-screen TUI (Claude Code, plus `vim`/`htop` as torture tests) is usable — colours,
resize, mouse, paste, unicode — on all three OSes, including Hyprland/Wayland. Throughput test:
`cat` a large file without freezing the UI. Record a go/no-go on Linux webview rendering, with
numbers (frame times while a harness streams, WebGL vs DOM), before starting M2.

## M2 — Projects & sidebar

- SQLite store + migrations. Open project / create project (with `git init` + initial commit).
- Sidebar tree with `local`. Selecting `local` opens a shell tab at the repo root.
- Persist selection, panel sizes, expansion state.

*Exit:* add, reorder and remove projects; restart the app and everything is where you left it.

## M3 — Workspaces (the core loop)

- `git` module: root / default-branch detection, worktree add / list / remove.
- Composer UI: harness, model, effort, base branch, message.
- Start → worktree + branch + harness launch with `argv` prompt transport.
- Built-in harness definitions (hard-coded, no settings UI yet). Naming + slugging.
- Failure handling: nothing half-created is left behind.

*Exit:* from a cold start, create three workspaces in one project on different harnesses and watch
them work in parallel. **This is the first version worth dogfooding.**

## M4 — Harness settings

- Settings file + override model. Settings → Harnesses form, argv preview, PATH detection,
  Test launch, Restore defaults, custom harnesses.
- `stdin` prompt transport with readiness detection.
- Re-verify every default in [04-harnesses](04-harnesses.md) end-to-end on each OS.

*Exit:* a harness Switchyard has never heard of can be added and used without touching code.

## M5 — Right panel

- File watcher (debounced, `.gitignore`-aware). Changes tab (uncommitted + committed vs merge-base).
- Files tab. Diff viewer + read-only file viewer. Open in editor.

*Exit:* while an agent works, the changes list and open diff update live, and stay responsive in a
large repo (test against one with a big `node_modules`).

## M6 — Session lifecycle

- Session records; Resume / Fork / New session; restore-on-launch (lazy: resume when the workspace
  is first opened, not all at once).
- Status dots from PTY activity; desktop notification when a busy agent goes quiet.
- Shell tabs alongside harness tabs. Rename / archive / delete workspace with safety prompts.
- Worktree reconciliation on startup.

*Exit:* quit mid-task, relaunch, and be back in the same conversations within a couple of clicks.

## M7 — Ship

- Packaging: AppImage + deb + rpm + AUR; dmg (universal, signed, notarized); NSIS/MSI (signed).
- Tauri updater. Crash/error log collection that stays local. First-run experience, docs site/README.
- Licence, contribution guide, issue templates.

*Exit:* v0.1.0 public release.

## Later (unordered)

- Commit / push / open PR from the UI; show PR + CI status on the workspace row.
- Per-project setup script and "files to copy into new worktrees" (`.env` etc.); run/dev-server button.
- `switchyardd`: move the PTY host out of process so agents survive closing the window
  (boundary already in place from M1; see open questions for lifecycle).
- Merge / rebase helpers; "apply this workspace onto local".
- Diff comments sent back to the agent as a prompt.
- Multi-repo projects; remote/SSH workspaces.
- Usage / cost view per workspace. MCP config management per harness.
- Command palette; themes; Omarchy theme integration.
