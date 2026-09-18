import type { HarnessRequest } from "@/lib/ipc";

/** A harness with every choice left to its own defaults, and no opening message. */
export const bareHarness = (id: string): HarnessRequest => ({
  id,
  model: null,
  effort: null,
  prompt: null,
});
