/**
 * Ordered, coalescing input for one session.
 *
 * Each IPC call is dispatched to a thread pool on the other side, so two calls made back to back
 * may run in either order — unacceptable for keystrokes, fatal for a paste split into chunks.
 * This sends one write at a time and, while it is in flight, gathers whatever else is typed into
 * the next one.
 */
export function createInputWriter(
  send: (data: string) => Promise<void>,
  onError: (error: unknown) => void = console.error,
) {
  let pending = "";
  let inFlight = false;

  async function drain() {
    inFlight = true;
    try {
      while (pending) {
        const data = pending;
        pending = "";
        await send(data);
      }
    } catch (error) {
      pending = "";
      onError(error);
    } finally {
      inFlight = false;
    }
  }

  return (data: string) => {
    pending += data;
    if (!inFlight) void drain();
  };
}
