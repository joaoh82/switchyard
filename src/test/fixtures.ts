import type { AddedProject, Project, Workspace } from "@/lib/ipc";

/** A project with its `local` workspace, as the core would describe it. */
export function project(name: string, overrides: Partial<Project> = {}): Project {
  const id = `p-${name}`;
  return {
    id,
    name,
    rootPath: `/code/${name}`,
    missing: false,
    workspaces: [
      {
        id: `w-${name}`,
        projectId: id,
        kind: "local",
        name: "local",
        path: `/code/${name}`,
        head: { label: "main", detached: false, unborn: false },
        missing: false,
      },
    ],
    ...overrides,
  };
}

export const added = (name: string, flags: Partial<AddedProject> = {}): AddedProject => ({
  project: project(name),
  alreadyKnown: false,
  openedRootInstead: false,
  ...flags,
});

/** A worktree workspace belonging to `project(projectName)`. */
export const worktree = (projectName: string, name: string): Workspace => ({
  id: `w-${projectName}-${name}`,
  projectId: `p-${projectName}`,
  kind: "worktree",
  name,
  path: `/worktrees/${projectName}/${name}`,
  head: { label: `sy/${name}`, detached: false, unborn: false },
  missing: false,
});
