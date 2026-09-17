import { useEffect } from "react";
import { hasCore, ipc } from "@/lib/ipc";
import { useTerminalStore } from "@/stores/terminals";

/** Keep the tab list in step with the sessions the core owns. Mount once. */
export function useTerminalSessions() {
  useEffect(() => {
    if (!hasCore()) return;
    const { hydrate, markExited } = useTerminalStore.getState();
    void hydrate().catch(console.error);
    const unlisten = ipc.onHostEvent((event) => {
      if (event.type === "exited") markExited(event.id, event.exit);
    });
    return () => void unlisten.then((stop) => stop());
  }, []);
}
