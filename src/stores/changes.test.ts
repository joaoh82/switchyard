import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ChangeSet, FileChange } from "@/lib/ipc";

const core = vi.hoisted(() => ({
  workspaceChanges: vi.fn(),
  workspaceDiff: vi.fn(),
  workspaceFile: vi.fn(),
  workspaceWatch: vi.fn(),
}));
vi.mock("@/lib/ipc", async (original) => ({
  ...(await original<typeof import("@/lib/ipc")>()),
  ipc: core,
}));

import { useChangesStore } from "./changes";

const change = (path: string, extra: Partial<FileChange> = {}): FileChange => ({
  path,
  oldPath: null,
  kind: "modified",
  additions: 1,
  deletions: 0,
  ...extra,
});
const set = (uncommitted: FileChange[], committed: FileChange[] = []): ChangeSet => ({
  uncommitted,
  committed,
  base: "main",
});
const text = (value: string) => ({ type: "text" as const, text: value });
const store = () => useChangesStore.getState();

describe("changes store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    core.workspaceWatch.mockResolvedValue(undefined);
    core.workspaceChanges.mockResolvedValue(set([change("a.ts")]));
    core.workspaceDiff.mockResolvedValue({ old: text("1"), new: text("2") });
    useChangesStore.setState({
      workspaceId: null,
      changes: null,
      error: null,
      viewing: null,
      diff: null,
      file: null,
      viewError: null,
    });
  });

  it("following a workspace loads its changes and moves the watcher to it", async () => {
    await store().follow("w1");
    expect(core.workspaceWatch).toHaveBeenCalledWith("w1");
    expect(store().changes?.uncommitted.map((c) => c.path)).toEqual(["a.ts"]);

    await store().follow("w1");
    expect(core.workspaceWatch).toHaveBeenCalledTimes(1);

    await store().follow(null);
    expect(core.workspaceWatch).toHaveBeenLastCalledWith(null);
    expect(store().changes).toBeNull();
  });

  it("opens a diff and reloads it when the files change", async () => {
    await store().follow("w1");
    const open = store().changes!.uncommitted[0]!;
    await store().view({ kind: "diff", change: open, scope: "uncommitted" });
    expect(core.workspaceDiff).toHaveBeenCalledWith("w1", open, "uncommitted");
    expect(store().diff?.new).toEqual(text("2"));

    core.workspaceChanges.mockResolvedValue(set([change("a.ts", { additions: 5 })]));
    core.workspaceDiff.mockResolvedValue({ old: text("1"), new: text("3") });
    await store().refresh();
    expect(store().diff?.new).toEqual(text("3"));
    expect(store().viewing).toMatchObject({ change: { additions: 5 } });
  });

  it("closes a diff whose change is gone instead of showing nothing", async () => {
    await store().follow("w1");
    await store().view({ kind: "diff", change: change("a.ts"), scope: "uncommitted" });
    core.workspaceChanges.mockResolvedValue(set([], [change("a.ts")])); // it was committed
    await store().refresh();
    expect(store().viewing).toBeNull();
    expect(store().diff).toBeNull();
  });

  it("drops results that arrive after the user moved on", async () => {
    await store().follow("w1");
    let release!: (value: unknown) => void;
    core.workspaceDiff.mockReturnValueOnce(new Promise((resolve) => (release = resolve)));
    const slow = store().view({ kind: "diff", change: change("a.ts"), scope: "uncommitted" });

    core.workspaceFile.mockResolvedValue(text("readme"));
    await store().view({ kind: "file", path: "README.md" });
    release({ old: text("stale"), new: text("stale") });
    await slow;

    expect(store().file).toEqual(text("readme"));
    expect(store().diff).toBeNull();
  });

  it("reports failures without losing the last good list", async () => {
    await store().follow("w1");
    core.workspaceChanges.mockRejectedValue({ code: "git_failed", message: "git exploded" });
    await store().refresh();
    expect(store().error).toBe("git exploded");
    expect(store().changes?.uncommitted).toHaveLength(1);

    core.workspaceDiff.mockRejectedValue({ code: "bad_path", message: "outside the workspace" });
    await store().view({ kind: "diff", change: change("a.ts"), scope: "uncommitted" });
    expect(store().viewError).toBe("outside the workspace");
  });
});
