# Security

## Reporting a vulnerability

Please **do not open a public issue** for security problems. Use GitHub's private reporting
instead: [**Report a vulnerability**](https://github.com/joaoh82/yardsort/security/advisories/new).

Include what you found, how to reproduce it, and the version and OS. You will get an
acknowledgement within a few days, and credit in the fix's release notes unless you prefer
otherwise.

Only the latest release is supported with security fixes.

## What Yardsort does and does not do

Knowing the design helps judge what is a vulnerability:

- Yardsort **launches programs you configured** — coding agents and your shell — in folders you
  chose, with your login shell's environment. An agent can do whatever its own permission model
  allows; that is the agent's security boundary, not Yardsort's.
- It stores **no credentials** and has **no network features of its own**: no account, no
  telemetry, no update check yet. Agents use their own logins.
- Programs are started with an argument list, **never through a shell**, so prompts and settings
  cannot inject commands.
- When browsing or diffing a workspace, the webview names the workspace by id and gives a
  relative path; the core resolves it and refuses anything that leaves the workspace folder.

Reports about breaking those guarantees — path traversal out of a workspace, command injection
through a prompt or a harness field, a webview escape — are exactly what we want to hear about.
