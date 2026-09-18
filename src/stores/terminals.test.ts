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

import { useTerminalStore, type TerminalTab } from "./terminals";

function session(id: string, program: string, workspace: string | null = "ws", exited = false) {
  return {
    id,
    program,
    args: [],
    cwd: null,
    pid: 1,
    size: { cols: 80, rows: 24 },
    labels: workspace ? { workspace } : {},
    state: exited
      ? { status: "exited", exit: { code: 0, success: true, signal: null } }
      : { status: "running" },
    idleMs: 0,
  } satisfies SessionInfo;
}

const tab = (id: string, workspaceId = "ws"): TerminalTab => ({
  id,
  workspaceId,
  title: id,
  exit: null,
});

describe("terminal store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    core.ptyClose.mockResolvedValue(undefined);
    useTerminalStore.setState({ tabs: [], active: {}, error: null });
  });

  it("opens a session in its workspace, titled after its program", async () => {
    core.ptySpawn.mockResolvedValue(session("s1", "/usr/bin/zsh"));
    await useTerminalStore.getState().open("ws");
    expect(core.ptySpawn).toHaveBeenCalledWith(
      expect.objectContaining({ program: null, workspaceId: "ws" }),
    );
    expect(useTerminalStore.getState().tabs).toEqual([
      { id: "s1", workspaceId: "ws", title: "zsh", exit: null },
    ]);
    expect(useTerminalStore.getState().active).toEqual({ ws: "s1" });
  });

  it("strips Windows paths and extensions from titles", async () => {
    core.ptySpawn.mockResolvedValue(session("s1", "C:\\Users\\me\\AppData\\npm\\tool.cmd"));
    await useTerminalStore.getState().open("ws");
    expect(useTerminalStore.getState().tabs[0]!.title).toBe("tool");
  });

  it("starts a harness when asked and titles the tab after it", async () => {
    const spawned = session("s1", "/opt/bin/claude-1.2.3");
    core.ptySpawn.mockResolvedValue({
      ...spawned,
      labels: { ...spawned.labels, harness: "claude" },
    });
    const harness = { id: "claude", model: "opus", effort: null, prompt: null };
    await useTerminalStore.getState().open("ws", harness);
    expect(core.ptySpawn).toHaveBeenCalledWith(expect.objectContaining({ harness, program: null }));
    expect(useTerminalStore.getState().tabs[0]!.title).toBe("claude");
  });

  it("surfaces a failed launch instead of opening a tab", async () => {
    core.ptySpawn.mockRejectedValue({ code: "program_not_found", message: "`nope` was not found" });
    await useTerminalStore.getState().open("ws");
    expect(useTerminalStore.getState().tabs).toEqual([]);
    expect(useTerminalStore.getState().error).toBe("`nope` was not found");
  });

  it("keeps an exit that arrives before the tab exists", async () => {
    const exit = { code: 2, success: false, signal: null };
    core.ptySpawn.mockImplementation(async () => {
      useTerminalStore.getState().markExited("fast", exit); // the event overtakes the response
      return session("fast", "false");
    });
    await useTerminalStore.getState().open("ws");
    expect(useTerminalStore.getState().tabs[0]!.exit).toEqual(exit);
  });

  it("tracks the active tab per workspace", () => {
    useTerminalStore.setState({
      tabs: [tab("a1", "a"), tab("a2", "a"), tab("b1", "b")],
      active: { a: "a1", b: "b1" },
    });
    useTerminalStore.getState().activate("a2");
    expect(useTerminalStore.getState().active).toEqual({ a: "a2", b: "b1" });
  });

  it("activates the neighbouring tab of the same workspace when the active one closes", async () => {
    useTerminalStore.setState({
      tabs: [tab("a1", "a"), tab("b1", "b"), tab("a2", "a"), tab("a3", "a")],
      active: { a: "a2", b: "b1" },
    });
    await useTerminalStore.getState().close("a2");
    expect(useTerminalStore.getState().active).toEqual({ a: "a3", b: "b1" });
    await useTerminalStore.getState().close("a3");
    expect(useTerminalStore.getState().active.a).toBe("a1");
    await useTerminalStore.getState().close("a1");
    expect(useTerminalStore.getState().active.a).toBeUndefined();
    expect(useTerminalStore.getState().tabs).toEqual([tab("b1", "b")]);
  });

  it("closes every session of removed workspaces, in the core too", async () => {
    useTerminalStore.setState({
      tabs: [tab("a1", "a"), tab("b1", "b"), tab("a2", "a")],
      active: { a: "a1", b: "b1" },
    });
    await useTerminalStore.getState().closeWorkspaces(["a"]);
    expect(useTerminalStore.getState().tabs).toEqual([tab("b1", "b")]);
    expect(useTerminalStore.getState().active).toEqual({ b: "b1" });
    expect(core.ptyClose.mock.calls.map(([id]) => id).sort()).toEqual(["a1", "a2"]);
  });

  it("adopts labelled sessions from the core, without duplicating known ones", async () => {
    useTerminalStore.setState({ tabs: [tab("s1")], active: { ws: "s1" } });
    core.ptyList.mockResolvedValue([
      session("s1", "zsh"),
      session("s2", "claude", "other", true),
      session("stray", "bash", null),
    ]);
    await useTerminalStore.getState().hydrate();
    const { tabs, active } = useTerminalStore.getState();
    expect(tabs.map((t) => t.id)).toEqual(["s1", "s2"]);
    expect(tabs[1]!.exit?.success).toBe(true);
    expect(active).toEqual({ ws: "s1", other: "s2" });
  });
});
