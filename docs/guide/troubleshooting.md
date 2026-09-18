# Troubleshooting

## "`claude` was not found on PATH"

Switchyard launches programs with the environment of your **login shell**, which it reads once at
startup. The status bar shows the result: `env: login shell · 34 PATH`.

1. Check the command works in a **new** terminal window: `which claude`.
2. If you installed it while Switchyard was running, restart Switchyard so it re-reads your
   environment.
3. If your `PATH` is set somewhere only some shells read, move it to your shell's profile
   (`~/.zprofile`, `~/.bash_profile`, `~/.config/fish/config.fish`).
4. Or put the full path in the harness's **Command** field —
   [Settings → Harnesses](settings.md#harnesses) shows where a command was found, or that it was not.

If the status bar says `shell environment unavailable`, hover it for the reason — usually a shell
startup file that waits for input or takes more than a few seconds. Switchyard then falls back to
the environment it was started with.

## Windows

- **SmartScreen warning when installing** — builds are not code-signed yet. Choose
  **More info → Run anyway**.
- **git is required** — install [Git for Windows](https://git-scm.com/download/win).
- **Agents that need WSL** — some agents support Windows only through WSL. Running an agent
  inside WSL from Switchyard is not supported yet.
- **Path too long** — keep the [worktree folder](settings.md#workspaces) short (`C:\sy`) and run
  `git config --global core.longpaths true`.

## Linux

- **Blank or flickering window** (mostly NVIDIA on Wayland) — start with
  `WEBKIT_DISABLE_DMABUF_RENDERER=1 switchyard`.
- **Terminal drawing looks wrong** — Switchyard uses the GPU (WebGL) and falls back to a slower
  renderer automatically if the GPU context is lost. The status bar shows which is active
  (`webgl` or `dom`).
- **No notifications** — you need a notification daemon (mako, dunst, or your desktop's own).

## macOS

- **"Switchyard is damaged" / cannot be opened** — that is Gatekeeper reacting to a build that was
  not notarized. Official releases are signed and notarized; for a build you made yourself, run
  `xattr -dr com.apple.quarantine /Applications/Switchyard.app`.

## A workspace says "missing"

Its folder is gone. Use **Restore from its branch** or **Delete** from the workspace's menu — see
[Workspaces](workspaces.md#when-a-workspaces-folder-disappears).

## Resume says the conversation was not found

The agent no longer has that conversation on disk — it was cleaned up, or it never saved one
(a session with no messages has nothing to save). Forget the entry and start a new session.

If this happens for **every** session, check whether you start Switchyard from a terminal that is
itself running inside an agent; versions before 0.1 leaked that agent's session markers into the
agents they launched, which stopped Claude Code from saving transcripts.

## Where Switchyard keeps things

|                                                  | Linux                                        | macOS                                               | Windows                         |
| ------------------------------------------------ | -------------------------------------------- | --------------------------------------------------- | ------------------------------- |
| Database (projects, workspaces, session records) | `~/.local/share/dev.switchyard.app/`         | `~/Library/Application Support/dev.switchyard.app/` | `%APPDATA%\dev.switchyard.app\` |
| Settings                                         | `~/.config/dev.switchyard.app/settings.toml` | same folder as above                                | same folder as above            |
| Worktrees                                        | `~/switchyard/` (configurable)               |                                                     |                                 |

Switchyard stores no credentials and sends nothing anywhere: no telemetry, no account. Agents use
their own logins and talk to their own services.

To start from scratch, quit Switchyard and delete the database folder. Your repositories,
branches and worktrees are not touched.

## Reporting a bug

[Open an issue](https://github.com/joaoh82/switchyard/issues/new/choose) with your OS, the
Switchyard version (status bar, bottom right), the agent and its version, and what you did. If
the app misbehaves at startup, running it from a terminal shows its log.
