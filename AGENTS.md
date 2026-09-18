# Working on Switchyard

Instructions for AI coding agents (and a fair summary for humans). Switchyard is a Tauri 2
desktop app — Rust core, React + TypeScript frontend — that runs terminal coding agents in
parallel git worktrees on Linux, macOS and Windows. Read [CONTRIBUTING.md](CONTRIBUTING.md) for
the code layout and [docs/design/03-architecture.md](docs/design/03-architecture.md) for how it
fits together.

## Documentation is part of the change — always

**Keep the docs up to date. A change that alters what a user sees or does is not finished until
the documentation says so, in the same commit or pull request.**

- `docs/guide/` and `docs/quick-start.md` describe every part of the app in detail. New feature,
  changed behaviour, renamed button, new shortcut, new setting, new error a user can hit → update
  the matching guide. A new area of the app gets a new guide, linked from `docs/README.md`.
- `README.md` — the highlights, install table and quick start must stay true.
- `docs/design/` — architecture, the harness model, the roadmap (tick milestones, record what was
  found) and open questions (strike the ones that get settled).
- Screenshots live in `docs/images/`. Retake them when the UI they show changes noticeably. They
  must never show a real user's name, paths, projects or account details — use a throwaway
  profile (`SWITCHYARD_DATA_DIR`, `SWITCHYARD_WORKTREE_ROOT`) and demo repositories.
- Before finishing, reread the docs you touched against the code. Do not document behaviour you
  have not verified.

## Commands

```sh
just dev              # run the app with hot reload
just check            # formatting, lints, types, all tests — run before every commit
just bindings-check   # generated TS bindings are current — run before pushing
just lint-windows     # clippy the PTY crate for Windows from any OS
just fmt              # format everything
```

`src/lib/bindings.ts` is generated from the Rust commands (`cargo test` or `just dev` rewrites
it). Never edit it by hand; commit it when it changes.

## Rules that are easy to break

- **Three platforms, always.** No feature is done until it works on Linux, macOS and Windows. CI
  runs all three; platform-only code paths (`#[cfg(windows)]`, ConPTY, `Path` vs `PATH`) have
  bitten before. `just lint-windows` catches some of it locally.
- **The frontend holds no truth.** State lives in the Rust core and the SQLite store. Only
  `src/lib/ipc.ts` talks to the core.
- **Never parse agent output.** Status, readiness and notifications derive from PTY _activity_.
- **Programs are spawned with an argv array, never through a shell string.**
- **Never destroy user work silently.** Deleting or archiving keeps the branch; uncommitted
  changes need a second, explicit confirmation, and a test that a "no" is respected.
- **App shortcuts live behind Mod** (`⌘`, or `Ctrl+Shift` elsewhere). Plain `Ctrl`+letter
  belongs to the program in the terminal.
- **Migrations are append-only.** Never edit a shipped file in `src-tauri/migrations/`.
- **Tests exercise the real thing**: real git repositories in temp dirs, real processes in real
  PTYs, UI through Testing Library. A test should fail without the change it covers.
- Zustand selectors must return stable values — select the array, derive (`filter`/`map`)
  outside. A selector that builds a new array re-renders forever.

## Trying the app without touching real data

```sh
SWITCHYARD_DATA_DIR=/tmp/sy SWITCHYARD_WORKTREE_ROOT=/tmp/sy-wt just dev
```

If the app is started from a terminal that is itself inside an agent, that is fine: the launch
environment drops the enclosing agent's session markers (see `src-tauri/src/env.rs`).
