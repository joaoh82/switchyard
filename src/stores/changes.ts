import { create } from "zustand";
import {
  errorMessage,
  ipc,
  type ChangeSet,
  type Content,
  type FileChange,
  type FileDiff,
  type Scope,
} from "@/lib/ipc";

/** What the lower half of the right panel is showing. */
export type Viewing =
  { kind: "diff"; change: FileChange; scope: Scope } | { kind: "file"; path: string };

interface ChangesState {
  workspaceId: string | null;
  changes: ChangeSet | null;
  error: string | null;
  viewing: Viewing | null;
  /** The loaded content for `viewing`; `null` while loading. */
  diff: FileDiff | null;
  file: Content | null;
  viewError: string | null;

  /** Point the panel at a workspace (or none): load its changes and start watching it. */
  follow: (workspaceId: string | null) => Promise<void>;
  /** Ask git again, and reload whatever is open. Called on every file-system signal. */
  refresh: () => Promise<void>;
  view: (viewing: Viewing | null) => Promise<void>;
}

const sameChange = (a: FileChange, b: FileChange) => a.path === b.path && a.oldPath === b.oldPath;

export const useChangesStore = create<ChangesState>((set, get) => {
  /** Load the open file or diff. Results for a view the user has left are dropped. */
  async function loadView() {
    const { workspaceId, viewing } = get();
    if (!workspaceId || !viewing) return;
    const stillCurrent = () => get().workspaceId === workspaceId && get().viewing === viewing;
    try {
      if (viewing.kind === "diff") {
        const diff = await ipc.workspaceDiff(workspaceId, viewing.change, viewing.scope);
        if (stillCurrent()) set({ diff, file: null, viewError: null });
      } else {
        const file = await ipc.workspaceFile(workspaceId, viewing.path);
        if (stillCurrent()) set({ file, diff: null, viewError: null });
      }
    } catch (error) {
      if (stillCurrent()) set({ viewError: errorMessage(error) });
    }
  }

  return {
    workspaceId: null,
    changes: null,
    error: null,
    viewing: null,
    diff: null,
    file: null,
    viewError: null,

    async follow(workspaceId) {
      if (get().workspaceId === workspaceId) return;
      set({
        workspaceId,
        changes: null,
        error: null,
        viewing: null,
        diff: null,
        file: null,
        viewError: null,
      });
      // Watching is best-effort: without it the panel still refreshes on focus and on demand.
      void ipc.workspaceWatch(workspaceId).catch(console.error);
      await get().refresh();
    },

    async refresh() {
      const { workspaceId } = get();
      if (!workspaceId) return;
      try {
        const changes = await ipc.workspaceChanges(workspaceId);
        if (get().workspaceId !== workspaceId) return;
        set({ changes, error: null });

        // Keep the open diff pointing at the same file; close it if the change is gone
        // (committed, reverted) rather than showing a diff of nothing.
        const { viewing } = get();
        if (viewing?.kind === "diff") {
          const list = viewing.scope === "uncommitted" ? changes.uncommitted : changes.committed;
          const current = list.find((change) => sameChange(change, viewing.change));
          if (!current) return set({ viewing: null, diff: null });
          if (current !== viewing.change) set({ viewing: { ...viewing, change: current } });
        }
        await loadView();
      } catch (error) {
        if (get().workspaceId === workspaceId) set({ error: errorMessage(error) });
      }
    },

    async view(viewing) {
      set({ viewing, diff: null, file: null, viewError: null });
      await loadView();
    },
  };
});
