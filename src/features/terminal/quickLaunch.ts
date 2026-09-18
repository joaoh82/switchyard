import type { HarnessRequest } from "@/lib/ipc";

/** Harnesses offered for a one-click launch in an existing workspace. */
export const QUICK_LAUNCH = ["claude", "codex", "grok", "opencode"] as const;

/** A harness with every choice left to its own defaults, and no opening message. */
export const bareHarness = (id: string): HarnessRequest => ({
  id,
  model: null,
  effort: null,
  prompt: null,
});
