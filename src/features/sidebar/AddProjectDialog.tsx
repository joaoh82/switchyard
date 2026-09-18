import { useEffect, useId, useRef, useState } from "react";
import { native } from "@/lib/native";
import { useProjectsStore } from "@/stores/projects";
import { enterWorkspace, openProjectFromDisk } from "./actions";

/** "Open a folder" or "create a new project" — the two ways a project comes to exist. */
export function AddProjectDialog({ onClose }: { onClose: () => void }) {
  const [mode, setMode] = useState<"choose" | "create">("choose");
  const titleId = useId();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-40 flex items-start justify-center bg-black/50 pt-[18vh]"
      onPointerDown={(event) => event.target === event.currentTarget && onClose()}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="w-[28rem] max-w-[calc(100vw-2rem)] rounded-lg border border-line bg-surface p-5 shadow-2xl shadow-black/50"
      >
        <h2 id={titleId} className="text-base font-semibold">
          {mode === "choose" ? "Add a project" : "Create a project"}
        </h2>
        {mode === "choose" ? (
          <div className="mt-4 grid gap-2">
            <Choice
              autoFocus
              title="Open a folder"
              detail="Add a repository you already have on this computer."
              onClick={async () => {
                onClose();
                await openProjectFromDisk();
              }}
            />
            <Choice
              title="Create a new project"
              detail="Make a new folder with an empty git repository in it."
              onClick={() => setMode("create")}
            />
          </div>
        ) : (
          <CreateForm onBack={() => setMode("choose")} onDone={onClose} />
        )}
      </div>
    </div>
  );
}

function Choice(props: {
  title: string;
  detail: string;
  autoFocus?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      autoFocus={props.autoFocus}
      onClick={props.onClick}
      className="rounded-md border border-line px-4 py-3 text-left outline-none hover:border-accent focus-visible:border-accent"
    >
      <div className="font-medium">{props.title}</div>
      <div className="mt-0.5 text-ink-muted">{props.detail}</div>
    </button>
  );
}

function CreateForm({ onBack, onDone }: { onBack: () => void; onDone: () => void }) {
  const lastParentDir = useProjectsStore((s) => s.lastParentDir);
  const error = useProjectsStore((s) => s.error);
  const [name, setName] = useState("");
  const [parent, setParent] = useState(lastParentDir ?? "");
  const [busy, setBusy] = useState(false);
  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    useProjectsStore.getState().dismiss();
    nameRef.current?.focus();
  }, []);

  const browse = async () => {
    const folder = await native.pickFolder("Create the project in…", parent || undefined);
    if (folder) setParent(folder);
  };

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    const created = await useProjectsStore.getState().createProject(name, parent);
    setBusy(false);
    if (!created) return;
    const selected = useProjectsStore.getState().selectedWorkspaceId;
    if (selected) enterWorkspace(selected);
    onDone();
  };

  const separator = parent.includes("\\") ? "\\" : "/";
  const ready = name.trim() !== "" && parent !== "" && !busy;
  const field =
    "w-full rounded border border-line bg-canvas px-2 py-1.5 outline-none focus:border-accent";

  return (
    <form onSubmit={submit} className="mt-4 grid gap-3">
      <label className="grid gap-1">
        <span className="text-ink-muted">Name</span>
        <input
          ref={nameRef}
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="my-project"
          spellCheck={false}
          autoComplete="off"
          className={`${field} select-text`}
        />
      </label>
      <div className="grid gap-1">
        <span className="text-ink-muted">Location</span>
        <div className="flex gap-2">
          <input
            aria-label="Location"
            value={parent}
            onChange={(event) => setParent(event.target.value)}
            placeholder="Choose a folder…"
            spellCheck={false}
            className={`${field} min-w-0 flex-1 font-mono text-[12px] select-text`}
          />
          <button
            type="button"
            onClick={browse}
            className="rounded border border-line px-3 hover:border-accent"
          >
            Browse…
          </button>
        </div>
      </div>
      <p className="min-h-4 truncate font-mono text-[11px] text-ink-faint">
        {ready || (name.trim() && parent)
          ? `${parent.replace(/[\\/]+$/, "")}${separator}${name.trim()}`
          : ""}
      </p>
      {error && (
        <p role="alert" className="text-red-400 select-text">
          {error}
        </p>
      )}
      <div className="flex justify-between">
        <button
          type="button"
          onClick={onBack}
          className="px-2 py-1.5 text-ink-muted hover:text-ink"
        >
          ← Back
        </button>
        <button
          type="submit"
          disabled={!ready}
          className="rounded bg-accent px-4 py-1.5 font-medium text-canvas disabled:opacity-40"
        >
          {busy ? "Creating…" : "Create project"}
        </button>
      </div>
    </form>
  );
}
