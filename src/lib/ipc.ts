/**
 * The only module that talks to the Rust core. Everything else imports from here, never from
 * `bindings.ts` (generated) or `@tauri-apps/api` directly.
 */
import { isTauri } from "@tauri-apps/api/core";
import { commands, type AppInfo } from "./bindings";

export type { AppInfo };

/** False in a plain browser tab and in unit tests, where there is no core to call. */
export const hasCore = isTauri;

export const ipc = {
  appInfo: (): Promise<AppInfo> => commands.appInfo(),
};
