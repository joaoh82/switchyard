import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { HarnessInfo } from "@/lib/ipc";
import { project, worktree } from "@/test/fixtures";

const core = vi.hoisted(() => ({
  harnessesList: vi.fn(),
  projectBranches: vi.fn(),
  workspaceCreate: vi.fn(),
  uiStateSave: vi.fn(),
}));
vi.mock("@/lib/ipc", async (original) => ({
  ...(await original<typeof import("@/lib/ipc")>()),
  ipc: core,
}));

import { useHarnessStore } from "@/stores/harnesses";
import { useProjectsStore } from "@/stores/projects";
import { useTerminalStore } from "@/stores/terminals";
import { Composer } from "./Composer";

const harness = (id: string, extra: Partial<HarnessInfo> = {}): HarnessInfo => ({
  id,
  label: id.toUpperCase(),
  command: id,
  baseArgs: [],
  modelArgs: [],
  effortArgs: [],
  sessionArgs: [],
  promptArgs: [],
  resumeArgs: [],
  forkArgs: [],
  efforts: [],
  models: [],
  promptTransport: "argv",
  sessionIdMode: "assigned",
  stdinReadyMs: 1500,
  enabled: true,
  builtin: true,
  modified: false,
  resolvedPath: `/usr/bin/${id}`,
  ...extra,
});

const session = (workspace: string) => ({
  id: "s1",
  program: "/usr/bin/claude",
  args: [],
  cwd: null,
  pid: 1,
  size: { cols: 80, rows: 24 },
  labels: { workspace, harness: "claude" },
  state: { status: "running" as const },
  hasOutput: true,
  idleMs: 0,
});

const app = project("app");

async function renderComposer() {
  render(<Composer project={app} />);
  await screen.findAllByRole("option", { name: "main" });
  return userEvent.setup();
}

describe("Composer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    core.uiStateSave.mockResolvedValue(undefined);
    core.projectBranches.mockResolvedValue({
      branches: ["develop", "main", "sy/kept-earlier", "sy/in-use"],
      default: "main",
      checkedOut: ["main", "sy/in-use"],
    });
    core.harnessesList.mockResolvedValue([
      harness("claude", { efforts: ["low", "high"], models: ["opus", "sonnet"] }),
      harness("codex", { efforts: ["low", "medium"] }),
      harness("opencode"),
      harness("grok", { resolvedPath: null }),
    ]);
    useHarnessStore.setState({ harnesses: [], loaded: false });
    useProjectsStore.setState({
      projects: [app],
      selectedWorkspaceId: "w-app",
      composingProjectId: "p-app",
      ui: {},
    });
    useTerminalStore.setState({ tabs: [], active: {}, lastSize: { cols: 100, rows: 30 } });
  });

  it("starts a workspace with the message on Enter and lands in its terminal", async () => {
    const created = worktree("app", "fix-login");
    core.workspaceCreate.mockResolvedValue({ workspace: created, session: session(created.id) });
    const user = await renderComposer();

    await user.type(screen.getByRole("textbox", { name: /work on/ }), "Fix the login bug{Enter}");

    expect(core.workspaceCreate).toHaveBeenCalledWith({
      projectId: "p-app",
      baseBranch: "main",
      existingBranch: null,
      harness: { id: "claude", model: null, effort: null, prompt: "Fix the login bug" },
      size: { cols: 100, rows: 30 },
    });
    const projects = useProjectsStore.getState();
    expect(projects.projects[0]!.workspaces.map((w) => w.name)).toEqual(["local", "fix-login"]);
    expect(projects.selectedWorkspaceId).toBe(created.id);
    expect(projects.composingProjectId).toBeNull();
    expect(useTerminalStore.getState().active[created.id]).toBe("s1");
  });

  it("passes the chosen harness, model, effort and base branch", async () => {
    const created = worktree("app", "x");
    core.workspaceCreate.mockResolvedValue({ workspace: created, session: session(created.id) });
    const user = await renderComposer();

    await user.selectOptions(screen.getByRole("combobox", { name: "Harness" }), "codex");
    await user.type(screen.getByRole("combobox", { name: "Model" }), " gpt-next ");
    await user.selectOptions(screen.getByRole("combobox", { name: "Effort" }), "medium");
    await user.selectOptions(screen.getByRole("combobox", { name: "Branch" }), "new:develop");
    await user.click(screen.getByRole("button", { name: "Start" }));

    expect(core.workspaceCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        baseBranch: "develop",
        harness: { id: "codex", model: "gpt-next", effort: "medium", prompt: null },
      }),
    );
    expect(JSON.parse(core.uiStateSave.mock.calls.at(-1)![1])).toEqual({
      harness: "codex",
      model: "gpt-next",
      effort: "medium",
    });
  });

  it("opens an existing branch instead of creating one, offering only branches nobody has checked out", async () => {
    const created = worktree("app", "kept-earlier");
    core.workspaceCreate.mockResolvedValue({ workspace: created, session: session(created.id) });
    const user = await renderComposer();

    const group = screen.getByRole("group", { name: "Open existing branch" });
    expect(
      within(group)
        .getAllByRole("option")
        .map((o) => o.textContent),
    ).toEqual(["develop", "sy/kept-earlier"]);

    await user.selectOptions(
      screen.getByRole("combobox", { name: "Branch" }),
      "open:sy/kept-earlier",
    );
    expect(screen.getByText(/Enter to start/)).toHaveTextContent(
      'Opens the existing branch "sy/kept-earlier"',
    );
    await user.click(screen.getByRole("button", { name: "Start" }));

    expect(core.workspaceCreate).toHaveBeenCalledWith(
      expect.objectContaining({ baseBranch: null, existingBranch: "sy/kept-earlier" }),
    );
  });

  it("Shift+Enter breaks the line instead of sending", async () => {
    const user = await renderComposer();
    const box = screen.getByRole("textbox", { name: /work on/ });
    await user.type(box, "line one{Shift>}{Enter}{/Shift}line two");
    expect(box).toHaveValue("line one\nline two");
    expect(core.workspaceCreate).not.toHaveBeenCalled();
  });

  it("hides effort for harnesses without it and forgets a model when the harness changes", async () => {
    const user = await renderComposer();
    await user.type(screen.getByRole("combobox", { name: "Model" }), "opus");
    expect(screen.getByRole("combobox", { name: "Effort" })).toBeInTheDocument();

    await user.selectOptions(screen.getByRole("combobox", { name: "Harness" }), "opencode");
    expect(screen.queryByRole("combobox", { name: "Effort" })).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Model" })).toHaveValue("");
  });

  it("offers harnesses that are not installed as disabled, and recalls the last picks", async () => {
    useProjectsStore.setState({
      ui: {
        "composer.last.p-app": JSON.stringify({ harness: "codex", model: "m", effort: "low" }),
      },
    });
    await renderComposer();
    expect(screen.getByRole("option", { name: /GROK — not installed/ })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Harness" })).toHaveValue("codex");
    expect(screen.getByRole("combobox", { name: "Model" })).toHaveValue("m");
    expect(screen.getByRole("combobox", { name: "Effort" })).toHaveValue("low");
  });

  it("does not offer harnesses that are switched off", async () => {
    core.harnessesList.mockResolvedValue([harness("claude", { enabled: false }), harness("codex")]);
    await renderComposer();
    const picker = screen.getByRole("combobox", { name: "Harness" });
    expect(
      within(picker)
        .getAllByRole("option")
        .map((o) => o.textContent),
    ).toEqual(["CODEX"]);
  });

  it("keeps the message and explains when starting fails", async () => {
    core.workspaceCreate.mockRejectedValue({
      code: "no_commits",
      message: "This repository has no commits yet.",
    });
    const user = await renderComposer();
    const box = screen.getByRole("textbox", { name: /work on/ });
    await user.type(box, "important thoughts{Enter}");

    expect(await screen.findByRole("alert")).toHaveTextContent("no commits yet");
    expect(box).toHaveValue("important thoughts");
    expect(box).toBeEnabled();
    expect(useProjectsStore.getState().composingProjectId).toBe("p-app");
  });

  it("cannot start without an installed harness, and says why", async () => {
    core.harnessesList.mockResolvedValue([harness("claude", { resolvedPath: null })]);
    await renderComposer();
    expect(screen.getByRole("button", { name: "Start" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent(/No enabled harness was found/);
  });

  it("Escape cancels composing", async () => {
    const user = await renderComposer();
    await user.type(screen.getByRole("textbox", { name: /work on/ }), "{Escape}");
    expect(useProjectsStore.getState().composingProjectId).toBeNull();
  });
});
