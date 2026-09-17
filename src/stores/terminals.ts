import { create } from "zustand";
import type { RendererKind } from "@/features/terminal/renderer";
import {
  errorMessage,
  ipc,
  type ExitInfo,
  type SessionId,
  type SessionInfo,
  type TermSize,
} from "@/lib/ipc";

export interface TerminalTab {
  id: SessionId;
  title: string;
  /** Set once the process has ended. The tab stays so its output can still be read. */
  exit: ExitInfo | null;
}

interface TerminalState {
  tabs: TerminalTab[];
  activeId: SessionId | null;
  error: string | null;
  renderer: RendererKind | null;
  /** Size of the most recently fitted terminal, so new sessions start at the right size. */
  lastSize: TermSize;

  /** Adopt sessions already running in the core (e.g. after the webview reloads). */
  hydrate: () => Promise<void>;
  open: (program?: string) => Promise<void>;
  close: (id: SessionId) => Promise<void>;
  activate: (id: SessionId) => void;
  markExited: (id: SessionId, exit: ExitInfo) => void;
  setRenderer: (kind: RendererKind) => void;
  setLastSize: (size: TermSize) => void;
  dismissError: () => void;
}

/**
 * Exits seen for sessions that have no tab yet. A program can exit before `ptySpawn` has even
 * returned, in which case the event overtakes the tab it belongs to.
 */
const earlyExits = new Map<SessionId, ExitInfo>();

function tabFor(session: SessionInfo): TerminalTab {
  const name = session.program.split(/[\\/]/).pop() ?? session.program;
  return {
    id: session.id,
    title: name.replace(/\.(exe|cmd|bat)$/i, ""),
    exit:
      session.state.status === "exited" ? session.state.exit : (earlyExits.get(session.id) ?? null),
  };
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
  tabs: [],
  activeId: null,
  error: null,
  renderer: null,
  lastSize: { cols: 80, rows: 24 },

  async hydrate() {
    const sessions = await ipc.ptyList();
    const known = new Set(get().tabs.map((tab) => tab.id));
    const adopted = sessions.filter((s) => !known.has(s.id)).map(tabFor);
    if (adopted.length === 0) return;
    set((state) => ({
      tabs: [...state.tabs, ...adopted],
      activeId: state.activeId ?? adopted[0]!.id,
    }));
  },

  async open(program) {
    try {
      const session = await ipc.ptySpawn({
        program: program ?? null,
        args: [],
        cwd: null,
        size: get().lastSize,
      });
      set((state) => ({
        tabs: [...state.tabs, tabFor(session)],
        activeId: session.id,
        error: null,
      }));
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  async close(id) {
    set((state) => {
      const index = state.tabs.findIndex((tab) => tab.id === id);
      const tabs = state.tabs.filter((tab) => tab.id !== id);
      const neighbour = tabs[Math.min(index, tabs.length - 1)]?.id ?? null;
      return { tabs, activeId: state.activeId === id ? neighbour : state.activeId };
    });
    await ipc.ptyClose(id).catch(() => {});
  },

  activate: (id) => set({ activeId: id }),
  markExited: (id, exit) => {
    if (!get().tabs.some((tab) => tab.id === id)) earlyExits.set(id, exit);
    set((state) => ({ tabs: state.tabs.map((tab) => (tab.id === id ? { ...tab, exit } : tab)) }));
  },
  setRenderer: (renderer) => set({ renderer }),
  setLastSize: (lastSize) => set({ lastSize }),
  dismissError: () => set({ error: null }),
}));
