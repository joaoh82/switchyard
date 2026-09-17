import { useEffect, useState } from "react";
import { hasCore, ipc, type AppInfo } from "@/lib/ipc";
import { formatShortcut } from "@/lib/platform";
import { useLayoutStore } from "@/stores/layout";

export function StatusBar() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const collapsed = useLayoutStore((s) => s.collapsed);
  const toggle = useLayoutStore((s) => s.toggle);

  useEffect(() => {
    if (!hasCore()) return;
    let cancelled = false;
    ipc.appInfo().then((value) => {
      if (!cancelled) setInfo(value);
    }, console.error);
    return () => {
      cancelled = true;
    };
  }, []);

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
      <span className="font-mono">
        {info
          ? `${info.name} ${info.version}${info.debug ? "-dev" : ""} · ${info.os}/${info.arch}`
          : "Switchyard · no core"}
      </span>
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
