import { native } from "@/lib/native";
import type { Project, Workspace } from "@/lib/ipc";
import { useProjectsStore } from "@/stores/projects";
import { useTerminalStore } from "@/stores/terminals";

/**
 * Select a workspace because the user asked for it. A workspace with nothing running gets a
 * shell, so that choosing one always lands somewhere useful. (Restoring the selection at startup
 * goes through the store directly and spawns nothing.)
 */
export function enterWorkspace(workspaceId: string) {
  useProjectsStore.getState().select(workspaceId);
  const terminals = useTerminalStore.getState();
  if (!terminals.tabs.some((tab) => tab.workspaceId === workspaceId)) {
    void terminals.open(workspaceId);
  }
}

/** Pick a folder and add it, offering to initialise git if it is not a repository yet. */
export async function openProjectFromDisk(): Promise<boolean> {
  const folder = await native.pickFolder("Open project");
  if (!folder) return false;

  const projects = useProjectsStore.getState();
  let result = await projects.openFolder(folder);
  if (result.status === "needs-git") {
    const agreed = await native.confirm(
      `${folder}\n\nis not a git repository. Switchyard needs one: every workspace is a git worktree.\n\nInitialise git here? This runs "git init" and creates an empty first commit. Your files are not changed.`,
      { title: "Initialise git?", okLabel: "Initialise git" },
    );
    if (!agreed) return false;
    result = await projects.openFolder(folder, true);
  }
  if (result.status !== "added") return false;

  const selected = useProjectsStore.getState().selectedWorkspaceId;
  if (selected) enterWorkspace(selected);
  return true;
}

export async function removeProject(project: Project) {
  const running = useTerminalStore
    .getState()
    .tabs.filter((tab) => !tab.exit && project.workspaces.some((w) => w.id === tab.workspaceId));
  const agreed = await native.confirm(
    `Remove "${project.name}" from Switchyard?\n\nNothing on disk is deleted — the folder and its git history stay exactly as they are.` +
      (running.length > 0
        ? `\n\n${running.length} running terminal session${running.length === 1 ? "" : "s"} in this project will be closed.`
        : ""),
    { title: "Remove project", okLabel: "Remove" },
  );
  if (!agreed) return;
  await useTerminalStore.getState().closeWorkspaces(project.workspaces.map((w) => w.id));
  await useProjectsStore.getState().remove(project.id);
}

/** Open the composer for a new workspace in the project the user is currently looking at. */
export function composeInCurrentProject() {
  const { projects, selectedWorkspaceId, composingProjectId, compose } =
    useProjectsStore.getState();
  const current =
    projects.find((p) => p.workspaces.some((w) => w.id === selectedWorkspaceId)) ??
    projects.find((p) => p.id === composingProjectId) ??
    projects[0];
  if (current && !current.missing) compose(current.id);
}

/**
 * Delete a worktree workspace: its folder goes, its branch stays. Uncommitted work is never
 * destroyed without a second, explicit confirmation that says so.
 */
export async function deleteWorkspace(workspace: Workspace) {
  const branch = workspace.head && !workspace.head.detached ? workspace.head.label : null;
  const agreed = await native.confirm(
    `Delete workspace "${workspace.name}"?\n\nThis removes its folder:\n${workspace.path}\n\n` +
      (branch
        ? `The branch "${branch}" and all its commits are kept.`
        : "Its commits stay in the repository.") +
      " Terminals running in this workspace will be closed.",
    { title: "Delete workspace", okLabel: "Delete" },
  );
  if (!agreed) return;

  await useTerminalStore.getState().closeWorkspaces([workspace.id]);
  const projects = useProjectsStore.getState();
  if ((await projects.deleteWorkspace(workspace.id)) !== "dirty") return;

  const force = await native.confirm(
    `"${workspace.name}" has uncommitted changes or untracked files.\n\nDeleting it now destroys that work for good — it is in no commit and cannot be recovered.`,
    { title: "Uncommitted work will be lost", okLabel: "Delete anyway" },
  );
  if (force) await projects.deleteWorkspace(workspace.id, true);
}
