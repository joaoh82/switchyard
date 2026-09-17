import { PanelHeader } from "@/features/shell/PanelHeader";

/** Left panel: projects and their workspaces. Populated in M2. */
export function Sidebar() {
  return (
    <aside aria-label="Projects" className="flex h-full flex-col bg-surface">
      <PanelHeader title="Projects" />
      <p className="p-3 text-ink-faint">No projects yet.</p>
    </aside>
  );
}
