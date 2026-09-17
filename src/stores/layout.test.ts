import { beforeEach, describe, expect, it } from "vitest";
import { useLayoutStore } from "./layout";

describe("layout store", () => {
  beforeEach(() => useLayoutStore.setState({ collapsed: { left: false, right: false } }));

  it("toggles one side without touching the other", () => {
    useLayoutStore.getState().toggle("left");
    expect(useLayoutStore.getState().collapsed).toEqual({ left: true, right: false });
    useLayoutStore.getState().toggle("left");
    expect(useLayoutStore.getState().collapsed).toEqual({ left: false, right: false });
  });

  it("keeps the same state object when nothing changes", () => {
    const before = useLayoutStore.getState().collapsed;
    useLayoutStore.getState().setCollapsed("right", false);
    expect(useLayoutStore.getState().collapsed).toBe(before);
  });
});
