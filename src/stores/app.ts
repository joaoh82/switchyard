import { create } from "zustand";
import { hasCore, ipc, type AppInfo, type EnvInfo } from "@/lib/ipc";

interface AppState {
  info: AppInfo | null;
  env: EnvInfo | null;
  load: () => Promise<void>;
}

/** Facts about the running app and the environment it launches programs in. */
export const useAppStore = create<AppState>((set) => ({
  info: null,
  env: null,
  async load() {
    if (!hasCore()) return;
    set({ info: await ipc.appInfo() });
    // Slower: the first call waits for the login shell to report its environment.
    set({ env: await ipc.envInfo() });
  },
}));
