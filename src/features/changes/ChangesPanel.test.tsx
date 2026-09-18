import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ChangeSet, FileChange } from "@/lib/ipc";

const core = vi.hoisted(() => ({
  workspaceChanges: vi.fn(),
  workspaceDiff: vi.fn(),
  workspaceFile: vi.fn(),
  workspaceFiles: vi.fn(),
  workspaceWatch: vi.fn(),
  openInEditor: vi.fn(),
  onWorkspaceFilesChanged: vi.fn(),
}));
vi.mock("@/lib/ipc", async (original) => ({
  ...(await original<typeof import("@/lib/ipc")>()),
  hasCore: () => true,
  ipc: core,
}));
// CodeMirror needs a real layout engine; what it is *given* is what matters here.
vi.mock("./CodeView", () => ({
  CodeView: (props: { path: string; text: string; original?: string }) => (
    <pre data-testid="code">{JSON.stringify(props)}</pre>
  ),
}));

import { useChangesStore } from "@/stores/changes";
import { useProjectsStore } from "@/stores/projects";
import { ChangesPanel } from "./ChangesPanel";

const change = (path: string, extra: Partial<FileChange> = {}): FileChange => ({
  path,
  oldPath: null,
  kind: "modified",
  additions: 3,
  deletions: 1,
  ...extra,
});
const text = (value: string) => ({ type: "text" as const, text: value });
const shown = async () => JSON.parse((await screen.findByTestId("code")).textContent!);

let fileSystemChanged: (workspaceId: string) => void;

function setChanges(next: Partial<ChangeSet>) {
  core.workspaceChanges.mockResolvedValue({
    uncommitted: [],
    committed: [],
    base: "main",
    ...next,
  });
}

async function renderPanel() {
  render(<ChangesPanel />);
  await waitFor(() => expect(core.workspaceChanges).toHaveBeenCalled());
  return userEvent.setup();
}

describe("ChangesPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    core.workspaceWatch.mockResolvedValue(undefined);
    core.openInEditor.mockResolvedValue(undefined);
    core.onWorkspaceFilesChanged.mockImplementation((handler) => {
      fileSystemChanged = handler;
      return Promise.resolve(() => {});
    });
    setChanges({
      uncommitted: [
        change("src/app.ts"),
        change("notes.md", { kind: "untracked", additions: null, deletions: null }),
      ],
      committed: [change("src/old name.ts", { kind: "renamed", oldPath: "src/legacy.ts" })],
    });
    core.workspaceDiff.mockResolvedValue({ old: text("before"), new: text("after") });
    useProjectsStore.setState({ selectedWorkspaceId: "w1", composingProjectId: null });
    useChangesStore.setState({
      workspaceId: null,
      changes: null,
      viewing: null,
      diff: null,
      file: null,
      error: null,
    });
  });

  it("groups what is uncommitted and what the branch has committed, with counts", async () => {
    await renderPanel();
    const uncommitted = await screen.findByRole("region", { name: "Uncommitted" });
    expect(
      within(uncommitted)
        .getAllByRole("button")
        .map((b) => b.title),
    ).toEqual(["src/app.ts", "notes.md"]);
    expect(uncommitted).toHaveTextContent("+3");
    const committed = screen.getByRole("region", { name: "On this branch · vs main" });
    expect(within(committed).getByRole("button")).toHaveAttribute(
      "title",
      "src/legacy.ts → src/old name.ts",
    );
    expect(screen.getByRole("tab", { name: "Changes 3" })).toBeInTheDocument();
    expect(core.workspaceWatch).toHaveBeenCalledWith("w1");
  });

  it("opens a diff with both sides, and an added file as all new", async () => {
    const user = await renderPanel();
    await user.click(await screen.findByTitle("src/app.ts"));
    expect(await shown()).toEqual({ path: "src/app.ts", text: "after", original: "before" });
    expect(core.workspaceDiff).toHaveBeenCalledWith(
      "w1",
      expect.objectContaining({ path: "src/app.ts" }),
      "uncommitted",
    );

    core.workspaceDiff.mockResolvedValue({ old: { type: "absent" }, new: text("fresh") });
    await user.click(screen.getByTitle("notes.md"));
    await waitFor(async () =>
      expect(await shown()).toEqual({ path: "notes.md", text: "fresh", original: "" }),
    );
  });

  it("updates the list and the open diff when the files change on disk", async () => {
    const user = await renderPanel();
    await user.click(await screen.findByTitle("src/app.ts"));
    await shown();

    setChanges({
      uncommitted: [
        change("src/app.ts", { additions: 9 }),
        change("brand-new.ts", { kind: "added" }),
      ],
    });
    core.workspaceDiff.mockResolvedValue({ old: text("before"), new: text("after, edited again") });
    act(() => fileSystemChanged("w1"));

    expect(await screen.findByTitle("brand-new.ts")).toBeInTheDocument();
    await waitFor(async () => expect((await shown()).text).toBe("after, edited again"));
    expect(screen.getByRole("region", { name: "Uncommitted" })).toHaveTextContent("+9");
  });

  it("ignores signals about other workspaces", async () => {
    await renderPanel();
    await screen.findByTitle("src/app.ts");
    const before = core.workspaceChanges.mock.calls.length;
    act(() => fileSystemChanged("some-other-workspace"));
    expect(core.workspaceChanges).toHaveBeenCalledTimes(before);
  });

  it("says why a file cannot be shown instead of showing garbage", async () => {
    const user = await renderPanel();
    core.workspaceDiff.mockResolvedValue({ old: { type: "absent" }, new: { type: "binary" } });
    await user.click(await screen.findByTitle("src/app.ts"));
    expect(await screen.findByText("Binary file — not shown.")).toBeInTheDocument();

    core.workspaceDiff.mockResolvedValue({
      old: text(""),
      new: { type: "tooLarge", bytes: 5_242_880 },
    });
    await user.click(screen.getByTitle("notes.md"));
    expect(await screen.findByText("File too large to show (5.0 MB).")).toBeInTheDocument();
  });

  it("browses files lazily and opens one read-only", async () => {
    core.workspaceFiles.mockImplementation(async (_id: string, dir: string) =>
      dir === ""
        ? [
            { name: "src", path: "src", isDir: true },
            { name: "README.md", path: "README.md", isDir: false },
          ]
        : [{ name: "main.rs", path: "src/main.rs", isDir: false }],
    );
    core.workspaceFile.mockResolvedValue(text("fn main() {}"));
    const user = await renderPanel();

    await user.click(screen.getByRole("tab", { name: "Files" }));
    await screen.findByRole("treeitem", { name: "README.md" });
    expect(core.workspaceFiles).toHaveBeenCalledTimes(1);

    await user.click(within(screen.getByRole("treeitem", { name: "src" })).getByRole("button"));
    await user.click(
      within(await screen.findByRole("treeitem", { name: "main.rs" })).getByRole("button"),
    );
    expect(core.workspaceFiles).toHaveBeenCalledWith("w1", "src");
    expect(await shown()).toEqual({ path: "src/main.rs", text: "fn main() {}" });
  });

  it("opens the file in the editor — and the folder, for a deleted file", async () => {
    const user = await renderPanel();
    await user.click(await screen.findByTitle("src/app.ts"));
    await user.click(await screen.findByRole("button", { name: "Open in editor" }));
    expect(core.openInEditor).toHaveBeenLastCalledWith("w1", "src/app.ts");

    setChanges({ uncommitted: [change("gone.ts", { kind: "deleted" })] });
    act(() => fileSystemChanged("w1"));
    await user.click(await screen.findByTitle("gone.ts"));
    await user.click(await screen.findByRole("button", { name: "Open in editor" }));
    expect(core.openInEditor).toHaveBeenLastCalledWith("w1", null);
  });

  it("expands the viewer for comfortable reading and shrinks it back", async () => {
    const user = await renderPanel();
    await user.click(await screen.findByTitle("src/app.ts"));
    await user.click(await screen.findByRole("button", { name: "Expand" }));
    expect(screen.getByRole("dialog", { name: "src/app.ts" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Shrink" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Close viewer" }));
    expect(screen.queryByTestId("code")).not.toBeInTheDocument();
  });

  it("stops watching while a workspace is being composed or none is selected", async () => {
    useProjectsStore.setState({ composingProjectId: "p1" });
    render(<ChangesPanel />);
    expect(await screen.findByText("Select a workspace to see its changes.")).toBeInTheDocument();
    expect(core.workspaceChanges).not.toHaveBeenCalled();
  });
});
