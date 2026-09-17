# Switchyard

A cross-platform desktop app for running AI coding agents in parallel, each on its own track.

Switchyard gives every task its own **workspace** — a git worktree plus a terminal session running
the coding harness of your choice (Claude Code, Codex, Grok, OpenCode, …). You see all your projects
and workspaces on the left, the live agent terminal in the middle, and the files and diff on the right.

It is modeled after Superset and Conductor, with one hard requirement they don't meet:
**Linux, macOS and Windows are all first-class from day one.**

> Status: early development — M0 (scaffold) done, M1 (terminal) next. See the [roadmap](docs/05-roadmap.md).

## Why "Switchyard"

A switchyard is where rail cars are sorted onto parallel tracks and later joined back into one train.
That is exactly the job: fan work out onto parallel branches, then merge it back to main.

## Stack

- [Tauri 2](https://tauri.app) — Rust core, system webview
- React + TypeScript + Vite frontend, [xterm.js](https://xtermjs.org) terminal
- `portable-pty` for pseudo-terminals (ConPTY on Windows)
- `git` CLI for worktree management
- SQLite for app state

## Developing

Prerequisites: [Rust](https://rustup.rs) (stable), [Bun](https://bun.sh), `git`, and the
[Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS.

```sh
bun install
bun tauri dev        # run the app with hot reload
bun run check        # format check, lint, typecheck, frontend tests
cargo test           # Rust tests (also regenerates src/lib/bindings.ts)
bun tauri build      # produce installers for the current OS
```

`src/lib/bindings.ts` is generated from the Rust commands by `tauri-specta` — never edit it by hand.
It is rewritten whenever the app runs in dev or `bun run bindings` is called, and CI fails if the
checked-in copy is stale.

## Docs

Start at [`docs/README.md`](docs/README.md).
