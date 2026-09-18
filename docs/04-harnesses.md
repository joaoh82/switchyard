# 04 — Harnesses

A **harness** is a terminal coding agent. To Switchyard it is pure configuration: a command plus
argument templates for the handful of things we need to do with it. Every supported agent has the
same shape because they all run in a terminal and all take roughly the same startup options.

## What we need from a harness

| Action                                                 | When                                                    |
| ------------------------------------------------------ | ------------------------------------------------------- |
| **Start** with an initial prompt (plus model / effort) | New workspace, new session                              |
| **Start** with no prompt                               | Empty composer, `local`                                 |
| **Resume** a previous session                          | App restarted, or harness exited and user clicks Resume |
| **Fork** a previous session                            | User wants to branch the conversation                   |

## Definition

Stored in the settings TOML; this is what the Settings → Harnesses form edits.

```toml
[[harness]]
id          = "claude"                 # stable key, referenced by sessions
label       = "Claude"                 # shown in the UI
command     = "claude"                 # resolved against the login-shell PATH
enabled     = true

base_args   = []                                   # always passed
model_args  = ["--model", "{model}"]               # omitted when model = default
effort_args = ["--effort", "{effort}"]             # omitted when effort = default
efforts     = ["low", "medium", "high", "xhigh", "max"]
models      = ["fable", "opus", "sonnet"]          # suggestions only; free text allowed

session_args = ["--session-id", "{session_id}"]    # omitted unless session_id_mode = "assigned"
prompt_args = ["{prompt}"]                         # omitted when there is no opening message
resume_args = ["--resume", "{session_id}"]
fork_args   = ["--resume", "{session_id}", "--fork-session", "--session-id", "{new_session_id}"]

prompt_transport = "argv"              # "argv" | "stdin"
session_id_mode  = "assigned"          # "assigned" | "latest-in-cwd"

[harness.env]                          # optional extra environment
```

The user's description of the reference app's settings maps directly: _label_, _command_,
_prompt-only args_ → `prompt_args`, _resume args_, _fork args_, _prompt transport_, _restore
defaults_. `model_args` / `effort_args` are our addition so the composer's pickers work without
every harness needing hand-written templates.

### Templating rules

- Args are an **array**; each element is one argv entry. Placeholders are substituted _inside_ an
  element and never re-split, so a prompt with spaces, quotes or newlines is always exactly one
  argument. No shell is involved.
- The settings form may show/edit args as a single shell-like line for convenience, parsed with
  shlex rules into the array — but the array is what is stored and executed.
- Placeholders: `{prompt}` `{model}` `{effort}` `{session_id}` `{new_session_id}` `{workspace}`
  `{worktree}` `{branch}`.
- An arg group whose placeholder has no value is dropped whole (no model chosen → no `--model`).
- Final argv to start = `command` + `base_args` + `model_args` + `effort_args` + `session_args` +
  `prompt_args`; to resume or fork, `resume_args` / `fork_args` take the place of the last two.
  The session id has a group of its own so that an empty prompt drops only the prompt.
- Braces that are not one of our placeholders are literal, so JSON can be passed in an argument.
- Built-in definitions are compiled in. User edits are stored as overrides, so **Restore defaults**
  is just "delete the override", and new app versions can ship corrected defaults.

### Prompt transport

- **`argv`** — the prompt is substituted into `prompt_args`. Simple and reliable. Default.
- **`stdin`** — the harness is started without the prompt, then the prompt is written to the PTY as
  a bracketed paste followed by Enter once the TUI is ready. For harnesses with no prompt argument,
  and for very long prompts: Windows caps a command line at ~32 K characters.
  "Ready" detection is the fiddly part: wait for first output then a short quiet period, with a
  configurable delay as the fallback. Needs care per harness.

Automatic fallback: if transport is `argv` and the built command line would exceed the platform
limit, switch to `stdin` for that launch.

### Session ids

Resume and fork need the harness's own session id. Two strategies:

- **`assigned`** — we generate a UUID and pass it at start. Deterministic; preferred where supported.
- **`latest-in-cwd`** — the harness picks its own id, so we resume "the most recent session in this
  directory". This is safe _because every workspace has a unique worktree path_. Its limit: with
  several sessions in one workspace only the newest is addressable. Later we can recover the real
  id from the harness's session store.

## Verified defaults

Checked against the CLIs installed on the planning machine on 2026-09-17 (`--help` output, not yet
exercised end-to-end). Re-verify during M4; these flags move.

|                   | **Claude Code** 2.1.273        | **Codex** 0.154.0                       | **Grok** 1.0.30                | **OpenCode** 1.18.31            |
| ----------------- | ------------------------------ | --------------------------------------- | ------------------------------ | ------------------------------- |
| command           | `claude`                       | `codex`                                 | `grok`                         | `opencode`                      |
| model             | `--model {model}`              | `-m {model}`                            | `-m {model}`                   | `-m {model}` (`provider/model`) |
| effort            | `--effort {effort}`            | `-c model_reasoning_effort="{effort}"`  | `--reasoning-effort {effort}`  | — (no flag)                     |
| effort values     | low, medium, high, xhigh, max  | _verify_                                | _verify_                       | n/a                             |
| prompt            | positional `{prompt}`          | positional `{prompt}`                   | positional `{prompt}`          | `--prompt {prompt}`             |
| assign session id | `--session-id {uuid}`          | —                                       | `--session-id {uuid}`          | —                               |
| resume            | `--resume {session_id}`        | `resume {session_id}` / `resume --last` | `--resume {session_id}`        | `--session {id}` / `--continue` |
| fork              | `--resume {id} --fork-session` | `fork {session_id}` / `fork --last`     | `--resume {id} --fork-session` | `--continue --fork`             |
| session_id_mode   | assigned                       | latest-in-cwd                           | assigned                       | latest-in-cwd                   |

Notes:

- Codex `resume` / `fork` are **subcommands**, not flags — which is exactly why resume and fork are
  full arg lists rather than "extra flags appended to the start args". `resume --last` filters by
  cwd by default (there is an `--all` flag to disable that), which is what makes `latest-in-cwd` work.
- Claude and Grok both have their own `--worktree` flag. We don't use it: Switchyard owns worktree
  creation so behaviour is identical across harnesses.
- **Gemini CLI** (0.60.0) is also installed and fits the same shape — `-m`, `-i {prompt}` for
  "prompt then stay interactive", `--session-id`, `--resume latest`. Cheap fifth default.
- Permission / approval modes (`--permission-mode`, `-a`, `--always-approve`, `--auto`) are
  deliberately **not** in the defaults. Users who want them add them to `base_args`.

## Settings UI

- List of harnesses (built-ins + custom), each with an enabled toggle and a "found at <path>" /
  "not found on PATH" indicator.
- Form fields per the definition above. Args fields show a live **preview of the exact argv** that
  would be run for a sample prompt — the fastest way to debug a template.
- **Test launch** opens the harness in a scratch terminal.
- **Restore defaults** per harness. **Add custom harness** for anything else that runs in a terminal.
