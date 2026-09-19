import { create } from "zustand";
import { hasCore, ipc, type AppInfo, type EnvInfo } from "@/lib/ipc";

interface AppState {
  info: AppInfo | null;
  env: EnvInfo | null;
  /** Mirrors the general setting, so event handlers need not ask the core each time. */
  notifyWhenQuiet: boolean;
  checkForUpdates: boolean;
  load: () => Promise<void>;
}

/** Facts about the running app and the environment it launches programs in. */
export const useAppStore = create<AppState>((set) => ({
  info: null,
  env: null,
  notifyWhenQuiet: true,
  // Off until the settings say otherwise, so nothing phones home before they are read.
  checkForUpdates: false,
  async load() {
    if (!hasCore()) return;
    set({ info: await ipc.appInfo() });
    const settings = await ipc.settingsGet();
    set({ notifyWhenQuiet: settings.notifyWhenQuiet, checkForUpdates: settings.checkForUpdates });
    // Slower: the first call waits for the login shell to report its environment.
    set({ env: await ipc.envInfo() });
  },
}));
