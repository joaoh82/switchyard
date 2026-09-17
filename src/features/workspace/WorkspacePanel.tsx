/** Center panel: the composer, then the harness terminal. Terminal lands in M1. */
export function WorkspacePanel() {
  return (
    <main aria-label="Workspace" className="flex h-full items-center justify-center bg-canvas">
      <div className="text-center">
        <img src="/icon.svg" alt="" className="mx-auto mb-4 size-16 opacity-90" />
        <h1 className="text-lg font-semibold">Switchyard</h1>
        <p className="mt-1 text-ink-muted">Every agent on its own track.</p>
      </div>
    </main>
  );
}
