/** True on macOS, where the primary modifier is ⌘ rather than Ctrl. */
export const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.platform || navigator.userAgent);

/** Whether the platform's primary modifier ("Mod") is held for this event. */
export function isModKey(event: KeyboardEvent | MouseEvent): boolean {
  return isMac ? event.metaKey : event.ctrlKey;
}

/** Human-readable shortcut, e.g. `formatShortcut("B")` → "⌘B" or "Ctrl+B". */
export function formatShortcut(key: string, { alt = false } = {}): string {
  if (isMac) return `${alt ? "⌥" : ""}⌘${key}`;
  return `Ctrl+${alt ? "Alt+" : ""}${key}`;
}
