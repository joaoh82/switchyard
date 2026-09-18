import { formatShortcut } from "@/lib/platform";
import { useTerminalStore, type TerminalTab } from "@/stores/terminals";
import { bareHarness, QUICK_LAUNCH } from "./quickLaunch";

export function TerminalTabs({ workspaceId }: { workspaceId: string }) {
  const allTabs = useTerminalStore((s) => s.tabs);
  const activeId = useTerminalStore((s) => s.active[workspaceId]);
  const open = useTerminalStore((s) => s.open);
  const tabs = allTabs.filter((tab) => tab.workspaceId === workspaceId);

  return (
    <div className="flex h-9 shrink-0 items-stretch border-b border-line bg-surface">
      <div role="tablist" aria-label="Terminals" className="flex min-w-0 items-stretch">
        {tabs.map((tab) => (
          <Tab key={tab.id} tab={tab} active={tab.id === activeId} />
        ))}
      </div>
      <button
        type="button"
        title={`New shell (${formatShortcut("T")})`}
        aria-label="New shell"
        onClick={() => void open(workspaceId)}
        className="px-3 text-ink-muted hover:bg-raised hover:text-ink"
      >
        +
      </button>
      <div className="ml-auto flex items-center gap-1 pr-2">
        {QUICK_LAUNCH.map((program) => (
          <button
            key={program}
            type="button"
            onClick={() => void open(workspaceId, bareHarness(program))}
            className="rounded px-2 py-0.5 text-[11px] text-ink-faint hover:bg-raised hover:text-ink"
          >
            {program}
          </button>
        ))}
      </div>
    </div>
  );
}

function Tab({ tab, active }: { tab: TerminalTab; active: boolean }) {
  const activate = useTerminalStore((s) => s.activate);
  const close = useTerminalStore((s) => s.close);
  const status = tab.exit ? (tab.exit.success ? "exited" : `exited ${tab.exit.code}`) : null;

  return (
    <div
      className={`group flex items-center border-r border-line ${
        active ? "bg-canvas text-ink" : "text-ink-muted hover:bg-raised"
      }`}
    >
      <button
        type="button"
        role="tab"
        aria-selected={active}
        onClick={() => activate(tab.id)}
        className="flex h-full items-center gap-2 pr-1 pl-3"
      >
        <span
          aria-hidden
          className={`size-1.5 rounded-full ${
            !tab.exit ? "bg-accent" : tab.exit.success ? "bg-ink-faint" : "bg-red-400"
          }`}
        />
        <span className="max-w-40 truncate">{tab.title}</span>
        {status && <span className="text-[11px] text-ink-faint">{status}</span>}
      </button>
      <button
        type="button"
        aria-label={`Close ${tab.title}`}
        onClick={() => void close(tab.id)}
        className="mr-1 rounded px-1 text-ink-faint opacity-0 group-hover:opacity-100 hover:bg-line hover:text-ink focus-visible:opacity-100"
      >
        ×
      </button>
    </div>
  );
}
