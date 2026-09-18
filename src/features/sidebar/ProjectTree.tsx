import { useState } from "react";
import type { Project, Workspace } from "@/lib/ipc";
import { native } from "@/lib/native";
import { useProjectsStore } from "@/stores/projects";
import { useTerminalStore } from "@/stores/terminals";
import { enterWorkspace, removeProject } from "./actions";
import { ContextMenu, type MenuItem } from "./ContextMenu";

export function ProjectTree() {
  const projects = useProjectsStore((s) => s.projects);
  return (
    <ul role="tree" aria-label="Projects" className="min-h-0 flex-1 overflow-y-auto py-1">
      {projects.map((project, index) => (
        <ProjectNode
          key={project.id}
          project={project}
          isFirst={index === 0}
          isLast={index === projects.length - 1}
        />
      ))}
    </ul>
  );
}

function ProjectNode(props: { project: Project; isFirst: boolean; isLast: boolean }) {
  const { project } = props;
  const expanded = useProjectsStore((s) => !s.collapsed.includes(project.id));
  const toggleCollapsed = useProjectsStore((s) => s.toggleCollapsed);
  const move = useProjectsStore((s) => s.move);
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);

  const items: MenuItem[] = [
    {
      label: "Reveal in file manager",
      disabled: project.missing,
      onSelect: () => void native.revealInFileManager(project.rootPath).catch(console.error),
    },
    { label: "Move up", disabled: props.isFirst, onSelect: () => void move(project.id, -1) },
    { label: "Move down", disabled: props.isLast, onSelect: () => void move(project.id, 1) },
    { label: "Remove from Switchyard…", danger: true, onSelect: () => void removeProject(project) },
  ];

  return (
    <li role="treeitem" aria-expanded={expanded} aria-label={project.name}>
      <div
        className="group flex h-7 items-center pr-1 hover:bg-raised"
        onContextMenu={(event) => {
          event.preventDefault();
          setMenuAt({ x: event.clientX, y: event.clientY });
        }}
      >
        <button
          type="button"
          onClick={() => toggleCollapsed(project.id)}
          title={project.rootPath}
          className="flex h-full min-w-0 flex-1 items-center gap-1 pl-2 text-left"
        >
          <span
            aria-hidden
            className={`w-3 text-[10px] text-ink-faint ${expanded ? "rotate-90" : ""}`}
          >
            ▶
          </span>
          <span
            className={`truncate font-medium ${project.missing ? "text-ink-faint line-through" : ""}`}
          >
            {project.name}
          </span>
          {project.missing && <span className="text-[11px] text-red-400">missing</span>}
        </button>
        <RowButton
          label={`More actions for ${project.name}`}
          onClick={(event) => {
            const box = event.currentTarget.getBoundingClientRect();
            setMenuAt({ x: box.left, y: box.bottom + 2 });
          }}
        >
          ⋯
        </RowButton>
        <RowButton label="New workspace (coming in the next milestone)" disabled>
          +
        </RowButton>
      </div>

      {expanded && (
        <ul role="group">
          {project.workspaces.map((workspace) => (
            <WorkspaceNode key={workspace.id} workspace={workspace} disabled={project.missing} />
          ))}
        </ul>
      )}
      {menuAt && <ContextMenu at={menuAt} items={items} onClose={() => setMenuAt(null)} />}
    </li>
  );
}

function WorkspaceNode({ workspace, disabled }: { workspace: Workspace; disabled: boolean }) {
  const selected = useProjectsStore((s) => s.selectedWorkspaceId === workspace.id);
  const running = useTerminalStore((s) =>
    s.tabs.some((tab) => tab.workspaceId === workspace.id && !tab.exit),
  );
  const head = workspace.head;

  return (
    <li role="treeitem" aria-selected={selected} aria-label={workspace.name}>
      <button
        type="button"
        disabled={disabled}
        onClick={() => enterWorkspace(workspace.id)}
        title={workspace.path}
        className={`flex h-7 w-full items-center gap-2 pr-2 pl-7 text-left disabled:opacity-40 ${
          selected ? "bg-raised text-ink" : "text-ink-muted hover:bg-raised"
        }`}
      >
        <span
          aria-hidden
          className={`size-1.5 shrink-0 rounded-full ${running ? "bg-accent" : "bg-line"}`}
        />
        <span className="truncate">{workspace.name}</span>
        {head && (
          <span
            className="ml-auto max-w-[55%] truncate font-mono text-[11px] text-ink-faint"
            title={head.detached ? "Detached HEAD" : head.unborn ? "No commits yet" : "Branch"}
          >
            {head.detached ? `@${head.label}` : head.label}
          </span>
        )}
      </button>
    </li>
  );
}

function RowButton(props: {
  label: string;
  disabled?: boolean;
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={props.label}
      title={props.label}
      disabled={props.disabled}
      onClick={props.onClick}
      className="size-6 shrink-0 rounded text-ink-faint opacity-0 group-hover:opacity-100 hover:bg-line hover:text-ink focus-visible:opacity-100 disabled:hover:bg-transparent disabled:hover:text-ink-faint"
    >
      {props.children}
    </button>
  );
}
