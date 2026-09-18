import { create } from "zustand";
import { ipc, type HarnessDef, type HarnessInfo } from "@/lib/ipc";

interface HarnessState {
  harnesses: HarnessInfo[];
  loaded: boolean;
  load: () => Promise<void>;
  /** Re-resolve commands against PATH, e.g. after installing a harness. */
  reload: () => Promise<void>;
  /** Both throw an `IpcError` so the settings form can show it next to the fields. */
  save: (def: HarnessDef) => Promise<void>;
  reset: (id: string) => Promise<void>;
}

/** Every configured harness: built-ins (with the user's changes) and the user's own. */
export const useHarnessStore = create<HarnessState>((set, get) => ({
  harnesses: [],
  loaded: false,
  async load() {
    if (!get().loaded) await get().reload();
  },
  async reload() {
    try {
      set({ harnesses: await ipc.harnessesList(), loaded: true });
    } catch (error) {
      console.error(error);
    }
  },
  save: async (def) => set({ harnesses: await ipc.harnessSave(def) }),
  reset: async (id) => set({ harnesses: await ipc.harnessReset(id) }),
}));

/** Harnesses that can be started right now: switched on, and found on PATH. */
export const launchable = (harnesses: HarnessInfo[]) =>
  harnesses.filter((harness) => harness.enabled && harness.resolvedPath);
