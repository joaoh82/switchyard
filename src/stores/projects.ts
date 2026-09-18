import { useMemo } from "react";
import { create } from "zustand";
import { errorMessage, ipc, isIpcError, type AddedProject, type Project } from "@/lib/ipc";

const KEYS = {
  selected: "sidebar.selectedWorkspace",
  collapsed: "sidebar.collapsedProjects",
  lastParent: "projects.lastParentDir",
} as const;

/** Outcome of trying to add a folder that may not be a repository yet. */
export type OpenResult = { status: "added" } | { status: "needs-git" } | { status: "failed" };

interface ProjectsState {
  projects: Project[];
  loaded: boolean;
  selectedWorkspaceId: string | null;
  /** Projects are expanded unless listed here, so new ones start open. */
  collapsed: string[];
  /** Where the last project was created; the next one is offered the same parent. */
  lastParentDir: string | null;
  error: string | null;
  notice: string | null;

  load: () => Promise<void>;
  /** Re-read projects from the core (branches change behind our back). */
  refresh: () => Promise<void>;
  openFolder: (path: string, initGit?: boolean) => Promise<OpenResult>;
  createProject: (name: string, parent: string) => Promise<boolean>;
  remove: (id: string) => Promise<void>;
  move: (id: string, by: -1 | 1) => Promise<void>;
  select: (workspaceId: string | null) => void;
  toggleCollapsed: (projectId: string) => void;
  dismiss: () => void;
}

const save = (key: string, value: unknown) =>
  void ipc.uiStateSave(key, JSON.stringify(value)).catch(console.error);

function parse<T>(raw: string | undefined, fallback: T): T {
  if (raw === undefined) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

const workspaceIds = (projects: Project[]) =>
  new Set(projects.flatMap((project) => project.workspaces.map((workspace) => workspace.id)));

export const useProjectsStore = create<ProjectsState>((set, get) => {
  /** Put a just-added project on screen: in the list, expanded, its `local` selected. */
  const adopt = (added: AddedProject) => {
    const { project } = added;
    const local = project.workspaces[0]?.id ?? null;
    set((state) => ({
      projects: state.projects.some((p) => p.id === project.id)
        ? state.projects.map((p) => (p.id === project.id ? project : p))
        : [...state.projects, project],
      collapsed: state.collapsed.filter((id) => id !== project.id),
      selectedWorkspaceId: local,
      error: null,
      notice: added.alreadyKnown
        ? `${project.name} was already in your projects.`
        : added.openedRootInstead
          ? `That folder is inside a repository, so its root was added: ${project.rootPath}`
          : null,
    }));
    save(KEYS.selected, local);
    save(KEYS.collapsed, get().collapsed);
  };

  return {
    projects: [],
    loaded: false,
    selectedWorkspaceId: null,
    collapsed: [],
    lastParentDir: null,
    error: null,
    notice: null,

    async load() {
      try {
        const [ui, projects] = await Promise.all([ipc.uiStateLoad(), ipc.projectsList()]);
        const selected = parse<string | null>(ui[KEYS.selected], null);
        set({
          projects,
          loaded: true,
          // The selection may point at something removed since it was saved.
          selectedWorkspaceId: selected && workspaceIds(projects).has(selected) ? selected : null,
          collapsed: parse<string[]>(ui[KEYS.collapsed], []),
          lastParentDir: parse<string | null>(ui[KEYS.lastParent], null),
        });
      } catch (error) {
        set({ loaded: true, error: errorMessage(error) });
      }
    },

    async refresh() {
      try {
        set({ projects: await ipc.projectsList() });
      } catch (error) {
        console.error(error);
      }
    },

    async openFolder(path, initGit = false) {
      try {
        adopt(await ipc.projectOpen(path, initGit));
        return { status: "added" };
      } catch (error) {
        if (isIpcError(error) && error.code === "not_a_git_repo") return { status: "needs-git" };
        set({ error: errorMessage(error) });
        return { status: "failed" };
      }
    },

    async createProject(name, parent) {
      try {
        adopt(await ipc.projectCreate(name, parent));
        set({ lastParentDir: parent });
        save(KEYS.lastParent, parent);
        return true;
      } catch (error) {
        set({ error: errorMessage(error) });
        return false;
      }
    },

    async remove(id) {
      const project = get().projects.find((p) => p.id === id);
      if (!project) return;
      try {
        await ipc.projectRemove(id);
      } catch (error) {
        return set({ error: errorMessage(error) });
      }
      const gone = new Set(project.workspaces.map((workspace) => workspace.id));
      set((state) => ({
        projects: state.projects.filter((p) => p.id !== id),
        collapsed: state.collapsed.filter((c) => c !== id),
        selectedWorkspaceId:
          state.selectedWorkspaceId && gone.has(state.selectedWorkspaceId)
            ? null
            : state.selectedWorkspaceId,
      }));
      save(KEYS.selected, get().selectedWorkspaceId);
      save(KEYS.collapsed, get().collapsed);
    },

    async move(id, by) {
      const projects = [...get().projects];
      const from = projects.findIndex((p) => p.id === id);
      const to = from + by;
      if (from < 0 || to < 0 || to >= projects.length) return;
      [projects[from], projects[to]] = [projects[to]!, projects[from]!];
      set({ projects });
      try {
        await ipc.projectsReorder(projects.map((p) => p.id));
      } catch (error) {
        set({ error: errorMessage(error) });
        await get().refresh();
      }
    },

    select(workspaceId) {
      if (get().selectedWorkspaceId === workspaceId) return;
      set({ selectedWorkspaceId: workspaceId });
      save(KEYS.selected, workspaceId);
    },

    toggleCollapsed(projectId) {
      set((state) => ({
        collapsed: state.collapsed.includes(projectId)
          ? state.collapsed.filter((id) => id !== projectId)
          : [...state.collapsed, projectId],
      }));
      save(KEYS.collapsed, get().collapsed);
    },

    dismiss: () => set({ error: null, notice: null }),
  };
});

function findWorkspace(projects: Project[], workspaceId: string | null) {
  for (const project of projects) {
    const workspace = project.workspaces.find((w) => w.id === workspaceId);
    if (workspace) return { project, workspace };
  }
  return null;
}

/** The selected workspace together with the project it belongs to. */
export function useSelectedWorkspace() {
  const projects = useProjectsStore((state) => state.projects);
  const selected = useProjectsStore((state) => state.selectedWorkspaceId);
  return useMemo(() => findWorkspace(projects, selected), [projects, selected]);
}
