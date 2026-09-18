import { useEffect, useId, useMemo, useRef, useState } from "react";
import { ipc, type BranchList, type HarnessInfo, type Project } from "@/lib/ipc";
import { formatShortcut } from "@/lib/platform";
import { useHarnessStore } from "@/stores/harnesses";
import { recall, useProjectsStore } from "@/stores/projects";
import { useTerminalStore } from "@/stores/terminals";

/** What was picked last time in a project, offered again next time. */
interface LastPicks {
  harness?: string;
  model?: string;
  effort?: string;
}
const picksKey = (projectId: string) => `composer.last.${projectId}`;

const control =
  "h-8 rounded border border-line bg-canvas px-2 text-ink outline-none focus:border-accent disabled:opacity-50";

/**
 * The start of every workspace: say what you want, pick who does it, press Enter. Nothing exists
 * until then — and if any part of starting fails, nothing is left behind and the message stays.
 */
export function Composer({ project }: { project: Project }) {
  const harnesses = useHarnessStore((s) => s.harnesses);
  const harnessesLoaded = useHarnessStore((s) => s.loaded);
  const ui = useProjectsStore((s) => s.ui);
  const last = useMemo(() => recall<LastPicks>(ui, picksKey(project.id), {}), [ui, project.id]);

  const [message, setMessage] = useState("");
  const [harnessId, setHarnessId] = useState<string | null>(null);
  const [model, setModel] = useState(last.model ?? "");
  const [effort, setEffort] = useState(last.effort ?? "");
  const [branches, setBranches] = useState<BranchList | null>(null);
  const [base, setBase] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const messageRef = useRef<HTMLTextAreaElement>(null);
  const modelsId = useId();

  useEffect(() => void useHarnessStore.getState().load(), []);
  useEffect(() => messageRef.current?.focus(), [project.id]);

  useEffect(() => {
    let stale = false;
    ipc.projectBranches(project.id).then(
      (list) => {
        if (stale) return;
        setBranches(list);
        setBase(list.default ?? list.branches[0] ?? "");
      },
      (reason) => !stale && setError(reason?.message ?? String(reason)),
    );
    return () => {
      stale = true;
    };
  }, [project.id]);

  // Prefer what was used last here, then the first harness that is actually installed.
  const installed = harnesses.filter((h) => h.resolvedPath);
  const harness: HarnessInfo | undefined =
    harnesses.find((h) => h.id === harnessId) ??
    installed.find((h) => h.id === last.harness) ??
    installed[0] ??
    harnesses[0];
  const effortChoice = harness?.efforts.includes(effort) ? effort : "";

  const chooseHarness = (id: string) => {
    setHarnessId(id);
    // A model name means nothing to a different harness.
    if (id !== harness?.id) setModel("");
  };

  const ready = !busy && !!harness?.resolvedPath && base !== "";

  const start = async () => {
    if (!ready || !harness) return;
    setBusy(true);
    setError(null);
    const projects = useProjectsStore.getState();
    const result = await projects.createWorkspace({
      projectId: project.id,
      baseBranch: base,
      harness: {
        id: harness.id,
        model: model.trim() || null,
        effort: effortChoice || null,
        prompt: message.trim() || null,
      },
      size: useTerminalStore.getState().lastSize,
    });
    setBusy(false);
    if ("error" in result) return setError(result.error);
    projects.remember(picksKey(project.id), {
      harness: harness.id,
      model: model.trim(),
      effort: effortChoice,
    } satisfies LastPicks);
    useTerminalStore.getState().adopt(result);
  };

  return (
    <div className="flex h-full items-center justify-center overflow-y-auto p-6">
      <form
        aria-label="New workspace"
        className="w-full max-w-2xl"
        onSubmit={(event) => {
          event.preventDefault();
          void start();
        }}
      >
        <h1 className="mb-3 text-center text-ink-muted">
          New workspace in <span className="font-medium text-ink">{project.name}</span>
        </h1>
        <textarea
          ref={messageRef}
          aria-label="What should the agent work on?"
          placeholder="What should the agent work on?"
          value={message}
          rows={5}
          disabled={busy}
          onChange={(event) => setMessage(event.target.value)}
          onKeyDown={(event) => {
            // Enter sends, Shift+Enter breaks the line — and never while an IME is composing.
            if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
              event.preventDefault();
              void start();
            } else if (event.key === "Escape") {
              useProjectsStore.getState().compose(null);
            }
          }}
          className="w-full resize-none rounded-lg border border-line bg-surface p-3 text-[14px] leading-relaxed outline-none select-text focus:border-accent disabled:opacity-60"
        />

        <div className="mt-2 flex flex-wrap items-center gap-2">
          <select
            aria-label="Harness"
            value={harness?.id ?? ""}
            disabled={busy || !harnessesLoaded}
            onChange={(event) => chooseHarness(event.target.value)}
            className={control}
          >
            {harnesses.map((h) => (
              <option key={h.id} value={h.id} disabled={!h.resolvedPath}>
                {h.label}
                {h.resolvedPath ? "" : " — not installed"}
              </option>
            ))}
          </select>

          <input
            aria-label="Model"
            placeholder="model: default"
            list={modelsId}
            value={model}
            disabled={busy}
            spellCheck={false}
            autoComplete="off"
            onChange={(event) => setModel(event.target.value)}
            className={`${control} w-40 select-text`}
          />
          <datalist id={modelsId}>
            {harness?.models.map((name) => (
              <option key={name} value={name} />
            ))}
          </datalist>

          {harness && harness.efforts.length > 0 && (
            <select
              aria-label="Effort"
              value={effortChoice}
              disabled={busy}
              onChange={(event) => setEffort(event.target.value)}
              className={control}
            >
              <option value="">effort: default</option>
              {harness.efforts.map((level) => (
                <option key={level} value={level}>
                  effort: {level}
                </option>
              ))}
            </select>
          )}

          <label className="ml-auto flex items-center gap-2 text-ink-faint">
            from
            <select
              aria-label="Base branch"
              value={base}
              disabled={busy || !branches}
              onChange={(event) => setBase(event.target.value)}
              className={`${control} max-w-44 font-mono text-[12px]`}
            >
              {branches?.branches.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </label>

          <button
            type="submit"
            disabled={!ready}
            className="h-8 rounded bg-accent px-4 font-medium text-canvas disabled:opacity-40"
          >
            {busy ? "Starting…" : "Start"}
          </button>
        </div>

        <div className="mt-3 min-h-10 text-center">
          {error ? (
            <p role="alert" className="text-red-400 select-text">
              {error}
            </p>
          ) : harnessesLoaded && installed.length === 0 ? (
            <p role="alert" className="text-red-400">
              None of the supported harnesses (claude, codex, grok, opencode) was found on your
              PATH.
            </p>
          ) : (
            <p className="text-ink-faint">
              Enter to start · Shift+Enter for a new line · Esc to cancel · {formatShortcut("N")}{" "}
              opens this again. A new branch and git worktree are created when you start.
            </p>
          )}
        </div>
      </form>
    </div>
  );
}
