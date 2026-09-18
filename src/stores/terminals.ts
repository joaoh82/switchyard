import { create } from "zustand";
import type { RendererKind } from "@/features/terminal/renderer";
import {
  errorMessage,
  ipc,
  WORKSPACE_LABEL,
  type ExitInfo,
  type SessionId,
  type SessionInfo,
  type TermSize,
} from "@/lib/ipc";

export interface TerminalTab {
  id: SessionId;
  /** The workspace this session runs in. */
  workspaceId: string;
  title: string;
  /** Set once the process has ended. The tab stays so its output can still be read. */
  exit: ExitInfo | null;
}

interface TerminalState {
  tabs: TerminalTab[];
  /** The tab showing in each workspace. */
  active: Record<string, SessionId | undefined>;
  error: string | null;
  renderer: RendererKind | null;
  /** Size of the most recently fitted terminal, so new sessions start at the right size. */
  lastSize: TermSize;

  /** Adopt sessions already running in the core (e.g. after the webview reloads). */
  hydrate: () => Promise<void>;
  /** Start `program` — the user's shell when omitted — in a workspace. */
  open: (workspaceId: string, program?: string) => Promise<void>;
  close: (id: SessionId) => Promise<void>;
  /** Close every session of the given workspaces, e.g. when their project is removed. */
  closeWorkspaces: (workspaceIds: string[]) => Promise<void>;
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

function tabFor(session: SessionInfo): TerminalTab | null {
  const workspaceId = session.labels[WORKSPACE_LABEL];
  if (!workspaceId) return null;
  const name = session.program.split(/[\\/]/).pop() ?? session.program;
  return {
    id: session.id,
    workspaceId,
    title: name.replace(/\.(exe|cmd|bat)$/i, ""),
    exit:
      session.state.status === "exited" ? session.state.exit : (earlyExits.get(session.id) ?? null),
  };
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
  tabs: [],
  active: {},
  error: null,
  renderer: null,
  lastSize: { cols: 80, rows: 24 },

  async hydrate() {
    const sessions = await ipc.ptyList();
    const known = new Set(get().tabs.map((tab) => tab.id));
    const adopted = sessions
      .filter((session) => !known.has(session.id))
      .map(tabFor)
      .filter((tab) => tab !== null);
    if (adopted.length === 0) return;
    set((state) => {
      const active = { ...state.active };
      for (const tab of adopted) active[tab.workspaceId] ??= tab.id;
      return { tabs: [...state.tabs, ...adopted], active };
    });
  },

  async open(workspaceId, program) {
    try {
      const session = await ipc.ptySpawn({
        program: program ?? null,
        args: [],
        cwd: null,
        workspaceId,
        size: get().lastSize,
      });
      const tab = tabFor(session);
      if (!tab) throw new Error("The core started a session without a workspace label.");
      set((state) => ({
        tabs: [...state.tabs, tab],
        active: { ...state.active, [workspaceId]: tab.id },
        error: null,
      }));
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  async close(id) {
    set((state) => {
      const closing = state.tabs.find((tab) => tab.id === id);
      if (!closing) return state;
      const siblings = state.tabs.filter((tab) => tab.workspaceId === closing.workspaceId);
      const index = siblings.findIndex((tab) => tab.id === id);
      const remaining = siblings.filter((tab) => tab.id !== id);
      const active = { ...state.active };
      if (active[closing.workspaceId] === id) {
        active[closing.workspaceId] = remaining[Math.min(index, remaining.length - 1)]?.id;
      }
      return { tabs: state.tabs.filter((tab) => tab.id !== id), active };
    });
    await ipc.ptyClose(id).catch(() => {});
  },

  async closeWorkspaces(workspaceIds) {
    const gone = new Set(workspaceIds);
    const closing = get().tabs.filter((tab) => gone.has(tab.workspaceId));
    set((state) => ({
      tabs: state.tabs.filter((tab) => !gone.has(tab.workspaceId)),
      active: Object.fromEntries(Object.entries(state.active).filter(([id]) => !gone.has(id))),
    }));
    await Promise.all(closing.map((tab) => ipc.ptyClose(tab.id).catch(() => {})));
  },

  activate: (id) =>
    set((state) => {
      const tab = state.tabs.find((candidate) => candidate.id === id);
      return tab ? { active: { ...state.active, [tab.workspaceId]: id } } : state;
    }),
  markExited: (id, exit) => {
    if (!get().tabs.some((tab) => tab.id === id)) earlyExits.set(id, exit);
    set((state) => ({ tabs: state.tabs.map((tab) => (tab.id === id ? { ...tab, exit } : tab)) }));
  },
  setRenderer: (renderer) => set({ renderer }),
  setLastSize: (lastSize) => set({ lastSize }),
  dismissError: () => set({ error: null }),
}));
