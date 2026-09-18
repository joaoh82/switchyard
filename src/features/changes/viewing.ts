import type { Viewing } from "@/stores/changes";

export const titleOf = (viewing: Viewing) =>
  viewing.kind === "file" ? viewing.path : viewing.change.path;

/** Identifies what is being viewed across refreshes, which replace the `Viewing` object. */
export const keyOf = (viewing: Viewing) =>
  viewing.kind === "file" ? `file:${viewing.path}` : `${viewing.scope}:${viewing.change.path}`;
