import { PanelHeader } from "@/features/shell/PanelHeader";

/** Right panel: changed files, file tree and diff. Populated in M5. */
export function ChangesPanel() {
  return (
    <aside aria-label="Changes" className="flex h-full flex-col bg-surface">
      <PanelHeader title="Changes" />
      <p className="p-3 text-ink-faint">Nothing to show.</p>
    </aside>
  );
}
