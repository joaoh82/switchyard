import { useEffect, type RefObject } from "react";

/**
 * Give a dialog the keyboard while it is open, and hand it back afterwards.
 *
 * Without this a dialog opened by a shortcut leaves focus where it was — in the terminal, which
 * swallows every key: Escape and Tab would go to the agent behind the dialog instead of to it.
 * The dialog element needs `tabIndex={-1}` so it can hold focus itself.
 */
export function useModalFocus(dialog: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const element = dialog.current;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    // Respect a field that focused itself (autoFocus, or an effect that ran before this one).
    if (element && !element.contains(document.activeElement)) element.focus();
    return () => {
      if (previous?.isConnected) previous.focus();
    };
  }, [dialog]);
}
