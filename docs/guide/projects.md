# Projects

A **project** is a git repository on your computer that Switchyard knows about. Projects live in
the left panel; everything else hangs off them.

## Adding a project

Press **+** next to _Projects_.

### Open a folder

`Ctrl+Shift+O` / `⌘O` goes straight here. Pick a folder and:

| The folder is…                                  | What happens                                                                                                                                                      |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| a git repository                                | It is added.                                                                                                                                                      |
| _inside_ a repository (say `repo/packages/web`) | The repository's root is added instead, and a note tells you so.                                                                                                  |
| not a repository                                | Switchyard asks whether to initialise git there. Saying yes runs `git init` and makes an empty first commit — your files are not changed. Saying no adds nothing. |
| already one of your projects                    | Nothing is duplicated; the existing project is selected.                                                                                                          |

Switchyard needs a repository because every workspace is a git worktree, and it needs at least one
commit because a worktree has to branch from something.

### Create a new project

Give it a **name** and a **location**. Switchyard creates `<location>/<name>`, runs `git init`
and makes an empty first commit. The location is remembered for next time.

The name becomes a folder name, so characters that are illegal on some system (`/ \ : * ? " < > |`)
are refused, and an existing folder is never touched. If any step fails, the half-made folder is
removed again.

## The `local` workspace

Every project has a **local** entry, always first. It is your repository's own checkout — not a
worktree — and the branch checked out there is shown next to it. Use it for a shell in the project
or to run an agent directly on your working copy. It cannot be renamed, archived or deleted.

The branch label follows reality: switch branches in another tool and it updates the next time
Switchyard's window gets focus.

## The project menu

Hover a project and press **⋯**, or right-click it:

- **New workspace** — same as the **+** on the row. See [Workspaces](workspaces.md).
- **Reveal in file manager**
- **Move up / Move down** — the order is remembered.
- **Remove from Switchyard…** — forgets the project and closes its terminals. **Nothing on disk is
  deleted**: the folder, its branches and its worktrees all stay. Add the folder again and its
  workspaces come back.

Click a project's arrow to collapse it. Collapsed projects, your selection and the panel sizes are
all restored the next time you start Switchyard.

## When a folder goes missing

If a project's folder is moved, deleted or on a drive that is not mounted, the project is shown
struck through and marked **missing**. It is not removed — it comes back by itself when the folder
does. If it is gone for good, remove the project from its menu.
