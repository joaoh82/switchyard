import { formatShortcut } from "@/lib/platform";
import { useAppStore } from "@/stores/app";
import { useLayoutStore } from "@/stores/layout";
import { useTerminalStore } from "@/stores/terminals";

export function StatusBar() {
  const info = useAppStore((s) => s.info);
  const env = useAppStore((s) => s.env);
  const renderer = useTerminalStore((s) => s.renderer);
  const collapsed = useLayoutStore((s) => s.collapsed);
  const toggle = useLayoutStore((s) => s.toggle);

  return (
    <footer className="flex h-6 shrink-0 items-center justify-between border-t border-line bg-surface px-2 text-[11px] text-ink-faint">
      <div className="flex items-center gap-1">
        <PanelToggle
          label="projects"
          shortcut={formatShortcut("B")}
          pressed={!collapsed.left}
          onClick={() => toggle("left")}
        />
        <PanelToggle
          label="changes"
          shortcut={formatShortcut("B", { alt: true })}
          pressed={!collapsed.right}
          onClick={() => toggle("right")}
        />
      </div>
      <div className="flex items-center gap-3 font-mono">
        {env?.warning && (
          <span className="text-red-400" title={env.warning}>
            shell environment unavailable
          </span>
        )}
        {env && !env.warning && (
          <span title={`Programs launch with the environment of ${env.shell}`}>
            env: {env.source === "loginShell" ? "login shell" : "process"} · {env.pathEntries} PATH
          </span>
        )}
        {renderer && <span title="Terminal renderer">{renderer}</span>}
        <span>
          {info
            ? `${info.name} ${info.version}${info.debug ? "-dev" : ""} · ${info.os}/${info.arch}`
            : "Switchyard · no core"}
        </span>
      </div>
    </footer>
  );
}

function PanelToggle(props: {
  label: string;
  shortcut: string;
  pressed: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={props.pressed}
      title={`Toggle ${props.label} (${props.shortcut})`}
      onClick={props.onClick}
      className="rounded px-1.5 py-0.5 hover:bg-raised hover:text-ink aria-pressed:text-ink-muted"
    >
      {props.label}
    </button>
  );
}
