/** Native OS affordances: folder pickers, confirmation boxes, the file manager. */
import { ask, open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export const native = {
  /** Resolves to the chosen folder, or `null` if the user cancelled. */
  async pickFolder(title: string, defaultPath?: string): Promise<string | null> {
    const picked = await open({ title, defaultPath, directory: true, multiple: false });
    return typeof picked === "string" ? picked : null;
  },

  confirm: (message: string, options: { title: string; okLabel: string }): Promise<boolean> =>
    ask(message, { ...options, kind: "warning", cancelLabel: "Cancel" }),

  revealInFileManager: (path: string): Promise<void> => revealItemInDir(path),
};
