import { create } from "zustand";
import { errorMessage, ipc, type SessionRecord } from "@/lib/ipc";

interface SessionsState {
  /** Harness conversations per workspace, newest first. Absent until loaded. */
  byWorkspace: Record<string, SessionRecord[] | undefined>;
  error: string | null;
  load: (workspaceId: string) => Promise<SessionRecord[]>;
  /** Drop a conversation from the history. The harness's own copy is not touched. */
  forget: (record: SessionRecord) => Promise<void>;
  dismissError: () => void;
}

/** The record history behind Resume and Fork. The core owns it; this is a cache per workspace. */
export const useSessionsStore = create<SessionsState>((set, get) => ({
  byWorkspace: {},
  error: null,

  async load(workspaceId) {
    try {
      const records = await ipc.sessionsList(workspaceId);
      set((state) => ({ byWorkspace: { ...state.byWorkspace, [workspaceId]: records } }));
      return records;
    } catch (error) {
      set({ error: errorMessage(error) });
      return get().byWorkspace[workspaceId] ?? [];
    }
  },

  async forget(record) {
    try {
      await ipc.sessionForget(record.id);
      await get().load(record.workspaceId);
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  dismissError: () => set({ error: null }),
}));
