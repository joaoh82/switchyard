import { useCallback, useEffect, useState } from "react";
import { errorMessage, ipc, type FileEntry } from "@/lib/ipc";
import { useChangesStore } from "@/stores/changes";

/**
 * The workspace's files, one folder at a time: a folder's contents are only asked for when it is
 * opened, so the size of the repository never matters. `revision` bumps on every file-system
 * signal and makes open folders reload.
 */
export function FileTree({ workspaceId, revision }: { workspaceId: string; revision: number }) {
  return (
    <ul role="tree" aria-label="Files" className="min-h-0 flex-1 overflow-y-auto py-1">
      <Folder workspaceId={workspaceId} dir="" depth={0} revision={revision} />
    </ul>
  );
}

function Folder(props: { workspaceId: string; dir: string; depth: number; revision: number }) {
  const { workspaceId, dir, depth, revision } = props;
  const [entries, setEntries] = useState<FileEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let stale = false;
    ipc.workspaceFiles(workspaceId, dir).then(
      (list) => !stale && (setEntries(list), setError(null)),
      (reason) => !stale && setError(errorMessage(reason)),
    );
    return () => {
      stale = true;
    };
  }, [workspaceId, dir, revision]);

  if (error) return <li className="px-3 py-1 text-red-400">{error}</li>;
  if (!entries) return null;
  if (entries.length === 0 && depth === 0) return <li className="p-3 text-ink-faint">No files.</li>;
  return entries.map((entry) => (
    <Entry
      key={entry.path}
      entry={entry}
      workspaceId={workspaceId}
      depth={depth}
      revision={revision}
    />
  ));
}

function Entry(props: { entry: FileEntry; workspaceId: string; depth: number; revision: number }) {
  const { entry, depth } = props;
  const [open, setOpen] = useState(false);
  const selected = useChangesStore(
    (s) => s.viewing?.kind === "file" && s.viewing.path === entry.path,
  );
  const view = useChangesStore((s) => s.view);

  const activate = useCallback(() => {
    if (entry.isDir) setOpen((value) => !value);
    else void view({ kind: "file", path: entry.path });
  }, [entry, view]);

  return (
    <li
      role="treeitem"
      aria-expanded={entry.isDir ? open : undefined}
      aria-selected={selected}
      aria-label={entry.name}
    >
      <button
        type="button"
        onClick={activate}
        style={{ paddingLeft: 12 + depth * 14 }}
        className={`flex h-6 w-full items-center gap-1.5 pr-2 text-left hover:bg-raised ${
          selected ? "bg-raised text-ink" : "text-ink-muted"
        }`}
      >
        <span aria-hidden className={`w-3 text-[9px] text-ink-faint ${open ? "rotate-90" : ""}`}>
          {entry.isDir ? "▶" : ""}
        </span>
        <span className="truncate">{entry.name}</span>
      </button>
      {entry.isDir && open && (
        <ul role="group">
          <Folder
            workspaceId={props.workspaceId}
            dir={entry.path}
            depth={depth + 1}
            revision={props.revision}
          />
        </ul>
      )}
    </li>
  );
}
