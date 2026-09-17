import { BenchRunner } from "@/features/terminal/BenchRunner";
import { QUICK_LAUNCH } from "@/features/terminal/quickLaunch";
import { TerminalTabs } from "@/features/terminal/TerminalTabs";
import { TerminalView } from "@/features/terminal/TerminalView";
import { useTerminalSessions } from "@/features/terminal/useTerminalSessions";
import { useAppStore } from "@/stores/app";
import { useTerminalStore } from "@/stores/terminals";

/**
 * Center panel. Until projects and workspaces exist (M2/M3) this is a scratch terminal area:
 * tabs of PTY sessions rooted at the home directory.
 */
export function WorkspacePanel() {
  useTerminalSessions();
  const dev = useAppStore((s) => s.info?.dev);
  const activeId = useTerminalStore((s) => s.activeId);
  const error = useTerminalStore((s) => s.error);
  const dismissError = useTerminalStore((s) => s.dismissError);

  if (dev?.bench) {
    return (
      <main aria-label="Workspace" className="h-full bg-canvas">
        <BenchRunner script={dev.bench} renderer={dev.renderer} />
      </main>
    );
  }

  return (
    <main aria-label="Workspace" className="flex h-full flex-col bg-canvas">
      <TerminalTabs />
      {error && (
        <div
          role="alert"
          className="flex items-start gap-3 border-b border-line bg-raised px-3 py-2"
        >
          <p className="flex-1 text-red-400 select-text">{error}</p>
          <button type="button" onClick={dismissError} className="text-ink-faint hover:text-ink">
            Dismiss
          </button>
        </div>
      )}
      <div className="min-h-0 flex-1">
        {/* Only the active terminal is mounted; the others keep running in the core and are
            repainted from a snapshot when they come back. `key` forces a fresh view per session. */}
        {activeId ? (
          <TerminalView key={activeId} sessionId={activeId} rendererOverride={dev?.renderer} />
        ) : (
          <EmptyState />
        )}
      </div>
    </main>
  );
}

function EmptyState() {
  const open = useTerminalStore((s) => s.open);
  return (
    <div className="flex h-full items-center justify-center">
      <div className="text-center">
        <img src="/icon.svg" alt="" className="mx-auto mb-4 size-16 opacity-90" />
        <h1 className="text-lg font-semibold">Switchyard</h1>
        <p className="mt-1 text-ink-muted">Every agent on its own track.</p>
        <div className="mt-5 flex flex-wrap justify-center gap-2">
          {[undefined, ...QUICK_LAUNCH].map((program) => (
            <button
              key={program ?? "shell"}
              type="button"
              onClick={() => void open(program)}
              className="rounded border border-line px-3 py-1 text-ink-muted hover:border-accent hover:text-ink"
            >
              {program ?? "shell"}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
