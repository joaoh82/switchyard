/**
 * The only module that talks to the Rust core. Everything else imports from here, never from
 * `bindings.ts` (generated) or `@tauri-apps/api` directly.
 */
import { Channel, isTauri } from "@tauri-apps/api/core";
import {
  commands,
  events,
  type AppInfo,
  type EnvInfo,
  type ExitInfo,
  type HostEvent,
  type IpcError,
  type SessionId,
  type SessionInfo,
  type SpawnRequest,
  type TermSize,
} from "./bindings";

export type {
  AppInfo,
  EnvInfo,
  ExitInfo,
  HostEvent,
  IpcError,
  SessionId,
  SessionInfo,
  SpawnRequest,
  TermSize,
};

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
