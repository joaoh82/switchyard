import { create } from "zustand";
import { ipc, type HarnessInfo } from "@/lib/ipc";

interface HarnessState {
  harnesses: HarnessInfo[];
  loaded: boolean;
  load: () => Promise<void>;
}

/** The configured harnesses and whether each is installed. Loaded once; M4 makes it editable. */
export const useHarnessStore = create<HarnessState>((set, get) => ({
  harnesses: [],
  loaded: false,
  async load() {
    if (get().loaded) return;
    try {
      set({ harnesses: await ipc.harnessesList(), loaded: true });
    } catch (error) {
      console.error(error);
    }
  },
}));
