# 03 — Architecture

## Overview

```
┌──────────────────────────── Tauri app ─────────────────────────────┐
│  Webview (React + TS)                                              │
│   sidebar · composer · xterm.js terminals · file tree · diff view  │
│        │  invoke(commands)            ▲  events / channels         │
│ ───────┼──────────────────────────────┼─────────────────────────── │
│        ▼                              │                            │
│  Rust core                                                         │
│   ├─ projects     registry, open/create                            │
│   ├─ workspaces   lifecycle, naming, worktree paths                │
│   ├─ git          GitBackend trait → git CLI                       │
│   ├─ harness      definitions, arg templating, launch plans        │
│   ├─ pty          PTY host: sessions, I/O pump, snapshots (→daemon)│
│   ├─ watch        notify-based fs watcher, debounced               │
│   ├─ env          login-shell environment resolution               │
│   └─ store        SQLite (state) + settings file                   │
└────────────────────────────────────────────────────────────────────┘
        │ spawns                      │ shells out
        ▼                             ▼
   claude / codex / grok / …         git
```

Rule of thumb: **the frontend holds no truth.** All state lives in the Rust core and the store; the
UI renders it and sends intents. This keeps the door open for a headless core later (see
"session persistence" below).

## Stack choices

| Concern | Choice | Why |
|---------|--------|-----|
| Shell | **Tauri 2** | Small, Rust core, all three OSes, good updater/bundler story. |
| Frontend | **React + TypeScript + Vite**, Tailwind, Zustand | Boring and well-trodden; biggest component ecosystem for trees, panels, diff views. |
| JS tooling | **bun** | Already installed; fast. Plain `package.json`, so npm/pnpm still work for contributors. |
| Terminal | **xterm.js** + fit, webgl (with DOM-renderer fallback; the canvas addon was dropped in xterm.js 6), web-links, unicode11 addons | The standard; what VS Code uses. |
| PTY | **`portable-pty`** (wezterm) | One API over Unix PTYs and Windows ConPTY. |
| Git | **`git` CLI** behind a `GitBackend` trait | Worktree support in libgit2/gitoxide is partial; the CLI is the reference implementation and respects the user's config, hooks and credentials. |
| State | **SQLite** via `rusqlite` (bundled) | Projects/workspaces/sessions are relational; bundled build avoids system-lib differences. |
| Settings | TOML file in the OS config dir | Human-editable, easy to back up and diff. Harness definitions live here. |
| Paths | `directories` crate | Correct config/data dirs per OS. |
| FS watching | `notify` + debouncer | Cross-platform; honour `.gitignore` via the `ignore` crate. |
| Diff view | CodeMirror 6 merge view *(tentative)* | Much lighter than Monaco. See open questions. |

## PTY & terminal data path

How it works: the Rust core opens a pseudo-terminal and spawns the harness attached to it, so the
harness believes it is in an ordinary terminal. xterm.js in the webview is the terminal *emulator*:
it parses the escape sequences, keeps the screen grid and draws it. This is the same split VS Code
and every Electron terminal use (xterm.js + `node-pty`); we swap `node-pty` for `portable-pty` and
Chromium for the system webview.

- One `PtySession` per terminal tab: owns the child process, the master PTY, a reader thread, a
  bounded scrollback ring buffer (bytes) and a **headless terminal state** (see below).
- **Output**: reader thread → Tauri `Channel` carrying raw bytes → `xterm.write()`. Use channels,
  not global events: they are ordered, per-session and avoid JSON-encoding the stream. Coalesce
  reads into ~16 ms batches so a TUI that repaints constantly doesn't flood IPC.
- **Input**: `xterm.onData` → `write(session_id, bytes)`.
- **Resize**: fit addon → `resize(cols, rows)`, debounced.
- **Activity signal**: the pump timestamps the last output; the sidebar status dots derive from
  that plus process liveness. We never parse harness output for meaning.
- **Exit**: capture exit code, keep scrollback, surface Resume/Fork actions.

### Rendering & performance expectations

Agent workloads are low-throughput (KB/s); the cost is full-screen repaints while a harness streams.

| Webview | Expectation |
|---------|-------------|
| WebView2 (Windows) | Chromium — on par with Electron. |
| WKWebView (macOS) | Fast JS and WebGL — no concern. |
| WebKitGTK (Linux) | **The risk.** WebGL is less reliable; NVIDIA + Wayland has known DMABUF blank/slow-window issues. |

Renderer policy: try `@xterm/addon-webgl`; on context loss or init failure fall back to xterm's
built-in DOM renderer (the canvas addon is not available for xterm.js 6). Expose a setting to force
either. Linux mitigations to evaluate in M1: `WEBKIT_DISABLE_DMABUF_RENDERER=1`, preferring the
integrated GPU on hybrid laptops. The planning machine (RTX 3070 Ti + Radeon 680M, WebKitGTK 2.52,
Hyprland) is exactly the hard case, so M1 is a meaningful test.

### The PTY host boundary (daemon-ready)

Reference apps in this space run a **terminal daemon**: a background process owns the PTYs and the
window is just a client, tmux-style. That is not about rendering speed — it is what lets agents keep
working when the window closes, crashes or updates. We want that eventually, so the `pty` module is
written as a **PTY host** with a message-shaped API from day one:

```
spawn(LaunchPlan) -> SessionId          list() -> [SessionInfo]
attach(SessionId) -> Snapshot + stream  detach(SessionId)
write(SessionId, bytes)                 resize(SessionId, cols, rows)
kill(SessionId, signal)                 events: output, exit, activity
```

- **v1:** the host runs in-process; "transport" is a function call plus a Tauri channel.
- **Later:** the same host runs as a separate process (`switchyardd`), the transport becomes a
  local socket (Unix domain socket / Windows named pipe), and the app becomes one client of it.
  Nothing above the boundary changes.
- Rules that keep this cheap: the host depends on nothing from Tauri or the UI; every request and
  event is a serialisable type; sessions are addressed by id, never by handle; the host — not the
  frontend — is the owner of scrollback and terminal state.

**Headless terminal state.** To restore a screen on (re)attach, raw byte replay is not enough for
alt-screen TUIs. The host feeds output through a headless VT parser in Rust (candidates:
`alacritty_terminal`, `vt100`) and can emit a **snapshot** — a byte sequence that repaints the
current screen — followed by the live stream. This pays off immediately in v1 (clean switching
between workspaces without keeping every xterm instance mounted) and is mandatory for the daemon.

- **Re-attach (v1)**: switching workspaces detaches the view; the session keeps running in the
  host. On re-mount: `attach` → write snapshot → stream live.
- **App quit (v1)**: the host dies with the app. Sessions are restored through each harness's
  resume args — see [04-harnesses](04-harnesses.md). With the daemon, quit merely detaches.

## Environment resolution (important)

GUI apps do not inherit the user's interactive shell environment. On macOS, and on Linux when
launched from a desktop launcher, `PATH` will be missing mise/asdf/nvm/cargo/homebrew entries — so
`claude` or `codex` simply won't be found even though they work in the user's terminal. (On the
machine this was planned on, every harness is installed via mise.)

- **Unix**: at startup run the user's login shell once, e.g. `$SHELL -ilc 'env -0'` with a timeout,
  parse it, cache it, and use it as the base environment for every spawn. Provide a "Reload
  environment" action.
- **Windows**: use the process environment; re-read user/system `PATH` from the registry on reload.
- Resolve the harness `command` against that `PATH` ourselves so "not found" becomes a clear,
  actionable error in the UI (with the PATH we searched), not a silent dead terminal.
- Set `TERM=xterm-256color`, `COLORTERM=truecolor`, and `SWITCHYARD_WORKSPACE`, `SWITCHYARD_PROJECT`
  for scripts and hooks.

Harnesses are spawned **directly with an argv array — never through a shell string.** This removes
a whole class of quoting bugs, especially for prompts and especially on Windows.

## Git & worktrees

Operations needed for v1, all via the CLI with `--porcelain` / `-z` output where available:

| Need | Command |
|------|---------|
| Is repo / find root | `git rev-parse --show-toplevel` |
| Default branch | `git symbolic-ref --short refs/remotes/origin/HEAD`, else current branch, else `init.defaultBranch` |
| Create workspace | `git worktree add -b <branch> <path> <base>` |
| List / reconcile | `git worktree list --porcelain` |
| Remove workspace | `git worktree remove [--force] <path>` (+ optional `git branch -D`) |
| Changes | `git status --porcelain=v2 -z`, `git diff --name-status -z <merge-base>` |
| Diff content | `git diff <merge-base> -- <file>`, `git show <rev>:<file>` |

Decisions:

- **Worktree location**: outside the repo, under a Switchyard-owned root —
  `<data-dir>/worktrees/<project-slug>/<workspace-slug>`; configurable. Keeping it outside avoids
  polluting the repo and confusing tools that walk the tree. Keep the path **short** — Windows'
  260-char limit bites deep `node_modules` trees (also recommend `core.longpaths=true` there).
- **Branch naming**: `<prefix>/<workspace-slug>`, prefix default `sy`, configurable.
- **Reconciliation**: on startup and on focus, compare the DB with `git worktree list`. Worktrees
  deleted behind our back are marked *missing*, not silently dropped.
- **Deleting** a workspace with uncommitted or unmerged work requires explicit confirmation that
  names what will be lost.
- Untracked-but-needed files (`.env`, etc.) don't exist in a fresh worktree. v1: document it.
  Later: per-project "copy these files" list / setup script.

## Data model (SQLite)

```
projects    id, name, root_path (unique), default_branch, created_at, sort_order
workspaces  id, project_id, name, slug, branch, base_branch, worktree_path,
            kind ('local' | 'worktree'), status ('active' | 'archived' | 'missing'),
            created_at, last_opened_at
sessions    id, workspace_id, harness_id, model, effort, harness_session_id,
            initial_prompt, state ('running' | 'exited'), exit_code,
            started_at, ended_at, forked_from_session_id
ui_state    key, value            -- panel sizes, last selection, per-project last-used picks
```

`local` is a real row (`kind = 'local'`, `worktree_path = root_path`) so the rest of the code never
special-cases it. Harness definitions are *not* in the DB; they live in the settings file.

## Cross-platform notes & risks

| Platform | Watch out for |
|----------|---------------|
| **Linux** | WebKitGTK is the weakest webview: xterm.js WebGL can be flaky → auto-fallback to the DOM renderer. Known blank-window issues on NVIDIA/Wayland (`WEBKIT_DISABLE_DMABUF_RENDERER=1`). Test on Hyprland (tiling, fractional scaling). Ship AppImage + deb + rpm, plus an AUR package. |
| **macOS** | PATH resolution (above). Code signing + notarization needed for a painless install. Universal binary. |
| **Windows** | ConPTY quirks (resize reflow, exit detection), needs Win10 1809+. WebView2 runtime bootstrapper. Path length. `git` must be installed — detect and guide. Harness CLIs may be `.cmd` shims (npm) which need `cmd /c` to spawn. Some harnesses officially support Windows only via WSL — see open questions. |
| **All** | Keybindings: `Mod` = Cmd on macOS, Ctrl elsewhere — but Ctrl+C/V/etc. belong to the TUI. Copy/paste in the terminal needs per-OS conventions (Ctrl+Shift+C/V on Linux/Windows). |

The PTY + webview terminal path is the highest-risk piece and the one most likely to differ per OS,
which is why the [roadmap](05-roadmap.md) proves it on all three platforms before anything else.

## Proposed repo layout

```
switchyard/
├─ docs/
├─ src/                      # frontend
│  ├─ app/  components/  features/{sidebar,composer,terminal,changes,settings}/
│  ├─ lib/ipc.ts             # typed wrappers around invoke/channels
│  └─ stores/
├─ src-tauri/
│  ├─ src/{projects,workspaces,git,harness,watch,env,store}/
│  ├─ crates/pty-host/       # no Tauri deps; in-process now, `switchyardd` later
│  ├─ src/commands.rs        # the IPC surface, thin
│  ├─ migrations/
│  └─ tauri.conf.json
└─ .github/workflows/        # build + test matrix: ubuntu, macos, windows
```

Generate the TypeScript IPC types from Rust (`tauri-specta` or `ts-rs`) so the boundary can't drift.
