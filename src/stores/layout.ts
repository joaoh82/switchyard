import { create } from "zustand";

export type SidePanel = "left" | "right";

interface LayoutState {
  collapsed: Record<SidePanel, boolean>;
  toggle: (panel: SidePanel) => void;
  setCollapsed: (panel: SidePanel, collapsed: boolean) => void;
}

/**
 * Which side panels are collapsed. Panel *sizes* are persisted by the panel group itself; this
 * store only carries intent so that shortcuts and buttons can drive the panels.
 */
export const useLayoutStore = create<LayoutState>((set) => ({
  collapsed: { left: false, right: false },
  toggle: (panel) =>
    set((state) => ({ collapsed: { ...state.collapsed, [panel]: !state.collapsed[panel] } })),
  setCollapsed: (panel, collapsed) =>
    set((state) =>
      state.collapsed[panel] === collapsed
        ? state
        : { collapsed: { ...state.collapsed, [panel]: collapsed } },
    ),
}));
