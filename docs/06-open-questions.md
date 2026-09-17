# 06 — Open questions

Decisions still to make. Each has a current lean so work isn't blocked; move items out of here once
settled.

## Product

1. **What exactly does `local` run?** Lean: same as a workspace (harness or shell tabs), rooted at
   the repo's own checkout, on whatever branch is checked out. Should its row offer the composer too,
   or open straight to a shell?
2. **Workspace naming.** Slug from the first message (readable, sometimes silly) vs generated names
   (a rail theme — station names — would suit the product). Lean: generated name at creation,
   one-click rename, optionally ask the harness to title it later.
3. **Multiple sessions per workspace?** Lean: yes, as tabs — the data model already allows it.
4. **Do we ever want our own chat UI?** The brief says the centre is a terminal, and that is v1.
   OpenCode (`opencode acp`) and others expose agent protocols that would allow a native UI later.
   Lean: not before v1; keep the core free of terminal-only assumptions where that's cheap.
5. **What happens to the branch when a workspace is deleted?** Lean: ask each time, default to keep
   if it has unmerged commits.

## Technical

6. **When to ship the terminal daemon.** *Direction settled:* the PTY host is built behind a
   message-shaped boundary from M1 (see [03-architecture](03-architecture.md)), in-process for v1,
   with harness resume args covering restarts. *Still open:* when to move it out of process, and
   the daemon's lifecycle — who starts/stops it, upgrades while sessions are live, one daemon per
   user vs per app instance, and what "background" means on Windows.
7. **Windows harness support.** Several harnesses officially target WSL rather than native Windows.
   Do we support launching harnesses *inside WSL* (`wsl.exe -d <distro> -- claude …`, worktree on
   the WSL filesystem)? Lean: native first; treat WSL as a per-harness command prefix + path
   translation, designed in M4, built when someone needs it.
8. **Worktree root default.** `<data-dir>/worktrees/…` (hidden, tidy) vs `~/switchyard/…` (visible,
   short — matters on Windows). Lean: visible and short, configurable.
9. **Diff viewer.** CodeMirror 6 merge view (light, flexible) vs Monaco (heavier, familiar) vs a
   dedicated React diff component. Lean: CodeMirror; decide with a spike in M5.
10. **Frontend framework.** React is the lean for ecosystem reasons. Solid/Svelte would be lighter.
    Decide before M0 — it's the one choice that is expensive to reverse.
11. **`stdin` transport readiness detection.** Quiet-period heuristic vs per-harness ready regex vs
    fixed delay. Needs the M4 experiments.
12. **Discovering harness-chosen session ids** (Codex, OpenCode) by reading their session stores —
    worth the coupling, or is `latest-in-cwd` enough?

## Project

13. **Licence.** MIT / Apache-2.0 dual (Rust convention) vs something copyleft. Needed before the
    repo goes public.
14. **Name availability.** Check `switchyard` on GitHub, crates.io, npm, the AUR and domains before
    investing in branding. Fallback app id: `dev.switchyard.app`.
15. **Distribution.** Open source from the start? Flatpak/Snap in addition to AppImage/deb/rpm/AUR?
