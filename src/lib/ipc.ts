/**
 * The only module that talks to the Rust core. Everything else imports from here, never from
 * `bindings.ts` (generated) or `@tauri-apps/api` directly.
 */
import { Channel, isTauri } from "@tauri-apps/api/core";
import {
  commands,
  events,
  type AddedProject,
  type AppInfo,
  type BranchList,
  type CreatedWorkspace,
  type EnvInfo,
  type ExitInfo,
  type HarnessDef,
  type HarnessInfo,
  type HarnessPreview,
  type HarnessRequest,
  type HostEvent,
  type IpcError,
  type NewWorkspace,
  type Project,
  type SessionId,
  type SessionInfo,
  type SettingsInfo,
  type SpawnRequest,
  type TermSize,
  type Workspace,
  type WorkspaceSettingsDto,
} from "./bindings";

export type {
  AddedProject,
  AppInfo,
  BranchList,
  CreatedWorkspace,
  EnvInfo,
  ExitInfo,
  HarnessDef,
  HarnessInfo,
  HarnessPreview,
  HarnessRequest,
  HostEvent,
  IpcError,
  NewWorkspace,
  Project,
  SessionId,
  SessionInfo,
  SettingsInfo,
  SpawnRequest,
  TermSize,
  Workspace,
  WorkspaceSettingsDto,
};

/** Session labels: the workspace a session belongs to, and the harness it runs (if any). */
export const WORKSPACE_LABEL = "workspace";
export const HARNESS_LABEL = "harness";

/** False in a plain browser tab and in unit tests, where there is no core to call. */
export const hasCore = isTauri;

export function isIpcError(value: unknown): value is IpcError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as IpcError).code === "string" &&
    typeof (value as IpcError).message === "string"
  );
}

/** A message fit to show the user, whatever was thrown. */
export function errorMessage(error: unknown): string {
  if (isIpcError(error)) return error.message;
  return error instanceof Error ? error.message : String(error);
}

type Outcome<T> = { status: "ok"; data: T } | { status: "error"; error: IpcError };

/** Generated commands return a result object; the app prefers exceptions. Throws `IpcError`. */
async function unwrap<T>(outcome: Promise<Outcome<T>>): Promise<T> {
  const result = await outcome;
  if (result.status === "error") throw result.error;
  return result.data;
}

/** For commands that return nothing (Rust's `()` arrives as `null`). */
async function done(outcome: Promise<Outcome<null>>): Promise<void> {
  await unwrap(outcome);
}

export const ipc = {
  appInfo: (): Promise<AppInfo> => commands.appInfo(),
  benchReport: async (report: string): Promise<void> => void (await commands.benchReport(report)),
  envInfo: (reload = false) => unwrap(commands.envInfo(reload)),

  projectsList: () => unwrap(commands.projectsList()),
  /** Rejects with code `not_a_git_repo` unless `initGit` is set. */
  projectOpen: (path: string, initGit = false) => unwrap(commands.projectOpen(path, initGit)),
  projectCreate: (name: string, parent: string) => unwrap(commands.projectCreate(name, parent)),
  projectRemove: (id: string) => done(commands.projectRemove(id)),
  projectsReorder: (orderedIds: string[]) => done(commands.projectsReorder(orderedIds)),
  uiStateLoad: () => unwrap(commands.uiStateLoad()),
  uiStateSave: (key: string, value: string) => done(commands.uiStateSave(key, value)),

  harnessesList: () => unwrap(commands.harnessesList()),
  /** Save a definition; built-ins keep only their differences. Resolves to the new list. */
  harnessSave: (def: HarnessDef) => unwrap(commands.harnessSave(def)),
  /** Restore a built-in, or delete a custom harness. Resolves to the new list. */
  harnessReset: (id: string) => unwrap(commands.harnessReset(id)),
  harnessPreview: (def: HarnessDef) => unwrap(commands.harnessPreview(def)),
  /** Start a (possibly unsaved) definition with no prompt, to see whether it comes up. */
  harnessTest: (def: HarnessDef, size: TermSize) => unwrap(commands.harnessTest(def, size)),
  settingsGet: () => unwrap(commands.settingsGet()),
  settingsSaveWorkspaces: (workspaces: WorkspaceSettingsDto) =>
    unwrap(commands.settingsSaveWorkspaces(workspaces)),
  projectBranches: (projectId: string) => unwrap(commands.projectBranches(projectId)),
  /** Worktree + branch + harness in one step; leaves nothing behind if any part fails. */
  workspaceCreate: (request: NewWorkspace) => unwrap(commands.workspaceCreate(request)),
  /** Rejects with code `worktree_dirty` unless `force` is set. The branch is always kept. */
  workspaceDelete: (id: string, force = false) => done(commands.workspaceDelete(id, force)),

  ptySpawn: (request: SpawnRequest) => unwrap(commands.ptySpawn(request)),
  ptyList: () => unwrap(commands.ptyList()),
  ptyWrite: (id: SessionId, data: string) => done(commands.ptyWrite(id, data)),
  ptyResize: (id: SessionId, size: TermSize) => done(commands.ptyResize(id, size)),
  ptyKill: (id: SessionId) => done(commands.ptyKill(id)),
  ptyClose: (id: SessionId) => done(commands.ptyClose(id)),
  ptyDetach: (id: SessionId, attachment: number) => done(commands.ptyDetach(id, attachment)),

  /**
   * Stream a session's output: one snapshot that repaints the terminal, then live bytes.
   * Resolves to the attachment id to pass to `ptyDetach`.
   *
   * The core sends raw bytes, which arrive as an `ArrayBuffer`; the generated binding believes
   * they are `number[]` (see `RawBytes` in `terminal.rs`), hence the one cast below.
   */
  ptyAttach(id: SessionId, onOutput: (bytes: Uint8Array) => void): Promise<number> {
    const channel = new Channel<ArrayBuffer>();
    channel.onmessage = (buffer) => onOutput(new Uint8Array(buffer));
    return unwrap(commands.ptyAttach(id, channel as unknown as Channel<number[]>));
  },

  onHostEvent: (handler: (event: HostEvent) => void) =>
    events.ptyHostEvent.listen((event) => handler(event.payload)),
};
