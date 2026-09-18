import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { added, project, worktree } from "@/test/fixtures";

const core = vi.hoisted(() => ({
  uiStateLoad: vi.fn(),
  uiStateSave: vi.fn(),
  projectsList: vi.fn(),
  projectOpen: vi.fn(),
  projectCreate: vi.fn(),
  projectRemove: vi.fn(),
  projectsReorder: vi.fn(),
  workspaceDelete: vi.fn(),
  ptySpawn: vi.fn(),
  ptyClose: vi.fn(),
}));
const native = vi.hoisted(() => ({
  pickFolder: vi.fn(),
  confirm: vi.fn(),
  revealInFileManager: vi.fn(),
}));
vi.mock("@/lib/ipc", async (original) => ({
  ...(await original<typeof import("@/lib/ipc")>()),
  hasCore: () => true,
  ipc: core,
}));
vi.mock("@/lib/native", () => ({ native }));

import { useProjectsStore } from "@/stores/projects";
import { useTerminalStore } from "@/stores/terminals";
import { Sidebar } from "./Sidebar";

const shellIn = (workspace: string) => ({
  id: `s-${workspace}`,
  program: "/bin/bash",
  args: [],
  cwd: null,
  pid: 1,
  size: { cols: 80, rows: 24 },
  labels: { workspace },
  state: { status: "running" as const },
  idleMs: 0,
});

async function renderSidebar(...names: string[]) {
  core.projectsList.mockResolvedValue(names.map((name) => project(name)));
  render(<Sidebar />);
  if (names[0]) await screen.findByRole("treeitem", { name: names[0] });
  else await screen.findByText("No projects yet.");
}

describe("Sidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    core.uiStateLoad.mockResolvedValue({});
    for (const fn of [core.uiStateSave, core.projectRemove, core.projectsReorder, core.ptyClose]) {
      fn.mockResolvedValue(undefined);
    }
    core.ptySpawn.mockImplementation(async ({ workspaceId }) => shellIn(workspaceId));
    useProjectsStore.setState({
      projects: [],
      loaded: false,
      selectedWorkspaceId: null,
      composingProjectId: null,
      collapsed: [],
      error: null,
      notice: null,
    });
    useTerminalStore.setState({ tabs: [], active: {}, error: null });
  });

  it("shows each project with its local workspace and branch", async () => {
    await renderSidebar("alpha", "beta");
    const alpha = screen.getByRole("treeitem", { name: "alpha" });
    const local = within(alpha).getByRole("treeitem", { name: "local" });
    expect(local).toHaveTextContent("main");
    expect(local).toHaveAttribute("aria-selected", "false");
  });

  it("entering a workspace selects it and opens a shell there — once", async () => {
    const user = userEvent.setup();
    await renderSidebar("alpha");
    const local = () => within(screen.getByRole("treeitem", { name: "local" })).getByRole("button");

    await user.click(local());
    expect(useProjectsStore.getState().selectedWorkspaceId).toBe("w-alpha");
    expect(core.ptySpawn).toHaveBeenCalledWith(expect.objectContaining({ workspaceId: "w-alpha" }));

    await user.click(local());
    expect(core.ptySpawn).toHaveBeenCalledTimes(1);
  });

  it("collapses and expands a project", async () => {
    const user = userEvent.setup();
    await renderSidebar("alpha");
    await user.click(screen.getByRole("button", { name: "alpha" }));
    expect(screen.queryByRole("treeitem", { name: "local" })).not.toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: "alpha" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("opening a plain folder asks before initialising git, and respects a no", async () => {
    const user = userEvent.setup();
    await renderSidebar();
    native.pickFolder.mockResolvedValue("/tmp/plain");
    native.confirm.mockResolvedValue(false);
    core.projectOpen.mockRejectedValue({ code: "not_a_git_repo", message: "not a repo" });

    await user.click(screen.getByRole("button", { name: "Add project" }));
    await user.click(screen.getByRole("button", { name: /Open a folder/ }));

    expect(native.confirm).toHaveBeenCalledWith(
      expect.stringContaining("/tmp/plain"),
      expect.objectContaining({ okLabel: "Initialise git" }),
    );
    expect(core.projectOpen).toHaveBeenCalledTimes(1);
    expect(core.projectOpen).not.toHaveBeenCalledWith("/tmp/plain", true);
  });

  it("creates a project from the dialog and lands in its local workspace", async () => {
    const user = userEvent.setup();
    await renderSidebar();
    native.pickFolder.mockResolvedValue("/code");
    core.projectCreate.mockResolvedValue(added("fresh"));

    await user.click(screen.getByRole("button", { name: "Add project" }));
    await user.click(screen.getByRole("button", { name: /Create a new project/ }));
    const create = screen.getByRole("button", { name: "Create project" });
    expect(create).toBeDisabled();

    await user.type(screen.getByLabelText("Name"), "fresh");
    await user.click(screen.getByRole("button", { name: "Browse…" }));
    expect(screen.getByText("/code/fresh")).toBeInTheDocument();
    await user.click(create);

    expect(core.projectCreate).toHaveBeenCalledWith("fresh", "/code");
    expect(await screen.findByRole("treeitem", { name: "fresh" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(core.ptySpawn).toHaveBeenCalledWith(expect.objectContaining({ workspaceId: "w-fresh" }));
  });

  it("keeps the dialog open and shows why when creation fails", async () => {
    const user = userEvent.setup();
    await renderSidebar();
    core.projectCreate.mockRejectedValue({
      code: "invalid_name",
      message: 'Project name cannot contain "/".',
    });

    await user.click(screen.getByRole("button", { name: "Add project" }));
    await user.click(screen.getByRole("button", { name: /Create a new project/ }));
    await user.type(screen.getByLabelText("Name"), "a/b");
    await user.type(screen.getByLabelText("Location"), "/code");
    await user.click(screen.getByRole("button", { name: "Create project" }));

    expect(await screen.findByRole("alert")).toHaveTextContent('cannot contain "/"');
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("removing a project confirms first, then closes its sessions and forgets it", async () => {
    const user = userEvent.setup();
    await renderSidebar("alpha", "beta");
    const alpha = screen.getByRole("treeitem", { name: "alpha" });
    await user.click(
      within(within(alpha).getByRole("treeitem", { name: "local" })).getByRole("button"),
    );
    expect(core.ptySpawn).toHaveBeenCalledTimes(1);

    native.confirm.mockResolvedValue(false);
    await user.click(screen.getByRole("button", { name: "More actions for alpha" }));
    await user.click(screen.getByRole("menuitem", { name: /Remove from Switchyard/ }));
    expect(native.confirm).toHaveBeenCalledWith(
      expect.stringMatching(/Nothing on disk is deleted[\s\S]*1 running terminal session /),
      expect.anything(),
    );
    expect(core.projectRemove).not.toHaveBeenCalled();

    native.confirm.mockResolvedValue(true);
    await user.click(screen.getByRole("button", { name: "More actions for alpha" }));
    await user.click(screen.getByRole("menuitem", { name: /Remove from Switchyard/ }));
    expect(core.ptyClose).toHaveBeenCalledWith("s-w-alpha");
    expect(core.projectRemove).toHaveBeenCalledWith("p-alpha");
    expect(screen.queryByRole("treeitem", { name: "alpha" })).not.toBeInTheDocument();
  });

  it("reorders from the menu, with the ends disabled", async () => {
    const user = userEvent.setup();
    await renderSidebar("alpha", "beta");
    await user.click(screen.getByRole("button", { name: "More actions for alpha" }));
    expect(screen.getByRole("menuitem", { name: "Move up" })).toBeDisabled();
    await user.click(screen.getByRole("menuitem", { name: "Move down" }));
    expect(core.projectsReorder).toHaveBeenCalledWith(["p-beta", "p-alpha"]);
  });

  it("the project's + opens the composer for it, and picking a workspace closes it again", async () => {
    const user = userEvent.setup();
    await renderSidebar("alpha", "beta");
    await user.click(screen.getByRole("button", { name: "New workspace in beta" }));
    expect(useProjectsStore.getState().composingProjectId).toBe("p-beta");
    expect(
      within(screen.getByRole("treeitem", { name: "beta" })).getByText("new workspace…"),
    ).toBeVisible();

    const alpha = screen.getByRole("treeitem", { name: "alpha" });
    await user.click(
      within(within(alpha).getByRole("treeitem", { name: "local" })).getByRole("button"),
    );
    expect(useProjectsStore.getState().composingProjectId).toBeNull();
  });

  describe("deleting a workspace", () => {
    async function openDeleteMenu() {
      const user = userEvent.setup();
      const app = project("app");
      app.workspaces.push(worktree("app", "fix-login"));
      core.projectsList.mockResolvedValue([app]);
      render(<Sidebar />);
      await screen.findByRole("treeitem", { name: "fix-login" });
      await user.click(screen.getByRole("button", { name: "More actions for fix-login" }));
      await user.click(screen.getByRole("menuitem", { name: /Delete workspace/ }));
      return user;
    }

    it("says the branch is kept, then removes it", async () => {
      native.confirm.mockResolvedValue(true);
      core.workspaceDelete.mockResolvedValue(undefined);
      await openDeleteMenu();

      expect(native.confirm).toHaveBeenCalledTimes(1);
      expect(native.confirm.mock.calls[0]![0]).toMatch(
        /branch "sy\/fix-login" and all its commits are kept/,
      );
      expect(core.workspaceDelete).toHaveBeenCalledWith("w-app-fix-login", false);
      expect(screen.queryByRole("treeitem", { name: "fix-login" })).not.toBeInTheDocument();
    });

    it("never destroys uncommitted work without a second, explicit yes", async () => {
      core.workspaceDelete.mockRejectedValueOnce({ code: "worktree_dirty", message: "dirty" });
      native.confirm.mockResolvedValueOnce(true).mockResolvedValueOnce(false);
      await openDeleteMenu();

      expect(native.confirm).toHaveBeenCalledTimes(2);
      expect(native.confirm.mock.calls[1]![0]).toMatch(/destroys that work for good/);
      expect(core.workspaceDelete).toHaveBeenCalledTimes(1);
      expect(screen.getByRole("treeitem", { name: "fix-login" })).toBeInTheDocument();
    });

    it("forces only after that second yes", async () => {
      core.workspaceDelete.mockRejectedValueOnce({ code: "worktree_dirty", message: "dirty" });
      core.workspaceDelete.mockResolvedValueOnce(undefined);
      native.confirm.mockResolvedValue(true);
      await openDeleteMenu();

      expect(core.workspaceDelete).toHaveBeenLastCalledWith("w-app-fix-login", true);
      expect(screen.queryByRole("treeitem", { name: "fix-login" })).not.toBeInTheDocument();
    });

    it("local has no delete", async () => {
      await renderSidebar("alpha");
      expect(
        screen.queryByRole("button", { name: "More actions for local" }),
      ).not.toBeInTheDocument();
    });
  });

  it("a project whose folder vanished is marked and cannot be entered", async () => {
    core.projectsList.mockResolvedValue([project("ghost", { missing: true })]);
    render(<Sidebar />);
    const ghost = await screen.findByRole("treeitem", { name: "ghost" });
    expect(ghost).toHaveTextContent("missing");
    expect(
      within(screen.getByRole("treeitem", { name: "local" })).getByRole("button"),
    ).toBeDisabled();
  });
});
