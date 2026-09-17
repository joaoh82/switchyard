import { describe, expect, it, vi } from "vitest";
import { createInputWriter } from "./writer";

/** A `send` whose calls stay pending until released, like a slow IPC round-trip. */
function controllableSend() {
  const sent: string[] = [];
  const releases: Array<() => void> = [];
  const send = (data: string) =>
    new Promise<void>((resolve) => {
      sent.push(data);
      releases.push(resolve);
    });
  const releaseNext = async () => {
    releases.shift()?.();
    await Promise.resolve();
    await Promise.resolve();
  };
  return { sent, send, releaseNext };
}

describe("input writer", () => {
  it("sends one write at a time and coalesces what arrives meanwhile", async () => {
    const { sent, send, releaseNext } = controllableSend();
    const write = createInputWriter(send);

    write("a");
    write("b");
    write("c");
    expect(sent).toEqual(["a"]);

    await releaseNext();
    expect(sent).toEqual(["a", "bc"]);

    await releaseNext();
    write("d");
    expect(sent).toEqual(["a", "bc", "d"]);
  });

  it("reports a failed write, drops the backlog and keeps working", async () => {
    const onError = vi.fn();
    const send = vi
      .fn<(data: string) => Promise<void>>()
      .mockRejectedValueOnce(new Error("session exited"))
      .mockResolvedValue(undefined);
    const write = createInputWriter(send, onError);

    write("x");
    write("queued-behind-the-failure");
    await vi.waitFor(() => expect(onError).toHaveBeenCalledOnce());

    write("y");
    await vi.waitFor(() => expect(send).toHaveBeenLastCalledWith("y"));
    expect(send).toHaveBeenCalledTimes(2);
  });
});
