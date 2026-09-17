import { useEffect } from "react";
import { Group, Panel, Separator, useDefaultLayout, usePanelRef } from "react-resizable-panels";
import { ChangesPanel } from "@/features/changes/ChangesPanel";
import { Sidebar } from "@/features/sidebar/Sidebar";
import { WorkspacePanel } from "@/features/workspace/WorkspacePanel";
import { isModKey } from "@/lib/platform";
import { useLayoutStore, type SidePanel } from "@/stores/layout";
import { StatusBar } from "./StatusBar";

const separatorClass =
  "w-px bg-line outline-none transition-colors hover:bg-accent focus-visible:bg-accent data-[separator=active]:bg-accent";

/** The three-panel frame: projects · work · files & changes. */
export function AppShell() {
  const leftRef = usePanelRef();
  const rightRef = usePanelRef();
  const collapsed = useLayoutStore((s) => s.collapsed);
  const toggle = useLayoutStore((s) => s.toggle);
  const setCollapsed = useLayoutStore((s) => s.setCollapsed);

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: "switchyard.shell",
    storage: localStorage,
  });

  // Drive the panels from the store…
  useEffect(() => {
    for (const [side, ref] of [
      ["left", leftRef],
      ["right", rightRef],
    ] as const) {
      const panel = ref.current;
      if (!panel || panel.isCollapsed() === collapsed[side]) continue;
      if (collapsed[side]) panel.collapse();
      else panel.expand();
    }
  }, [collapsed, leftRef, rightRef]);

  // …and keep the store honest when the user collapses one by dragging.
  const syncFromPanel = (side: SidePanel) => () => {
    const panel = (side === "left" ? leftRef : rightRef).current;
    if (panel) setCollapsed(side, panel.isCollapsed());
  };

  // Mod+B toggles the left panel, Mod+Alt+B the right. Mod-only, so a TUI never loses a key.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!isModKey(event) || event.shiftKey || event.code !== "KeyB") return;
      event.preventDefault();
      toggle(event.altKey ? "right" : "left");
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [toggle]);

  return (
    <div className="flex h-full flex-col">
      <Group
        orientation="horizontal"
        className="min-h-0 flex-1"
        defaultLayout={defaultLayout}
        onLayoutChanged={onLayoutChanged}
      >
        <Panel
          id="left"
          panelRef={leftRef}
          defaultSize={260}
          minSize={200}
          maxSize={480}
          collapsible
          collapsedSize={0}
          onResize={syncFromPanel("left")}
        >
          <Sidebar />
        </Panel>
        <Separator className={separatorClass} />
        <Panel id="center" minSize={360}>
          <WorkspacePanel />
        </Panel>
        <Separator className={separatorClass} />
        <Panel
          id="right"
          panelRef={rightRef}
          defaultSize={340}
          minSize={240}
          maxSize={720}
          collapsible
          collapsedSize={0}
          onResize={syncFromPanel("right")}
        >
          <ChangesPanel />
        </Panel>
      </Group>
      <StatusBar />
    </div>
  );
}
