import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionInfo } from "@/lib/ipc";

const core = vi.hoisted(() => ({
  ptySpawn: vi.fn(),
  ptyList: vi.fn(),
  ptyClose: vi.fn(),
}));
vi.mock("@/lib/ipc", async (original) => ({
  ...(await original<typeof import("@/lib/ipc")>()),
  ipc: core,
}));

import { useTerminalStore } from "./terminals";

function session(id: string, program: string, exited = false): SessionInfo {
  return {
    id,
    program,
    args: [],
    cwd: null,
    pid: 1,
    size: { cols: 80, rows: 24 },
    state: exited
      ? { status: "exited", exit: { code: 0, success: true, signal: null } }
      : { status: "running" },
    idleMs: 0,
  };
}

describe("terminal store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    core.ptyClose.mockResolvedValue(undefined);
    useTerminalStore.setState({ tabs: [], activeId: null, error: null });
  });

  it("opens a session as the active tab, titled after its program", async () => {
    core.ptySpawn.mockResolvedValue(session("s1", "/usr/bin/zsh"));
    await useTerminalStore.getState().open();
    expect(core.ptySpawn).toHaveBeenCalledWith(expect.objectContaining({ program: null }));
    expect(useTerminalStore.getState().tabs).toEqual([{ id: "s1", title: "zsh", exit: null }]);
    expect(useTerminalStore.getState().activeId).toBe("s1");
  });

  it("strips Windows paths and extensions from titles", async () => {
    core.ptySpawn.mockResolvedValue(session("s1", "C:\\Users\\me\\AppData\\npm\\claude.cmd"));
    await useTerminalStore.getState().open("claude");
    expect(useTerminalStore.getState().tabs[0]!.title).toBe("claude");
  });

  it("surfaces a failed launch instead of opening a tab", async () => {
    core.ptySpawn.mockRejectedValue({ code: "program_not_found", message: "`nope` was not found" });
    await useTerminalStore.getState().open("nope");
    expect(useTerminalStore.getState().tabs).toEqual([]);
    expect(useTerminalStore.getState().error).toBe("`nope` was not found");
  });

  it("keeps an exit that arrives before the tab exists", async () => {
    const exit = { code: 2, success: false, signal: null };
    core.ptySpawn.mockImplementation(async () => {
      useTerminalStore.getState().markExited("fast", exit); // the event overtakes the response
      return session("fast", "false");
    });
    await useTerminalStore.getState().open("false");
    expect(useTerminalStore.getState().tabs[0]!.exit).toEqual(exit);
  });

  it("activates the neighbouring tab when the active one closes", async () => {
    useTerminalStore.setState({
      tabs: ["a", "b", "c"].map((id) => ({ id, title: id, exit: null })),
      activeId: "b",
    });
    await useTerminalStore.getState().close("b");
    expect(useTerminalStore.getState().activeId).toBe("c");
    await useTerminalStore.getState().close("c");
    expect(useTerminalStore.getState().activeId).toBe("a");
    await useTerminalStore.getState().close("a");
    expect(useTerminalStore.getState().activeId).toBeNull();
    expect(core.ptyClose).toHaveBeenCalledTimes(3);
  });

  it("adopts sessions that already exist in the core, without duplicating known ones", async () => {
    useTerminalStore.setState({ tabs: [{ id: "s1", title: "zsh", exit: null }], activeId: "s1" });
    core.ptyList.mockResolvedValue([session("s1", "zsh"), session("s2", "claude", true)]);
    await useTerminalStore.getState().hydrate();
    const { tabs, activeId } = useTerminalStore.getState();
    expect(tabs.map((tab) => tab.id)).toEqual(["s1", "s2"]);
    expect(tabs[1]!.exit?.success).toBe(true);
    expect(activeId).toBe("s1");
  });
});
