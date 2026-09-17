import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { useLayoutStore } from "./stores/layout";

describe("App shell", () => {
  beforeEach(() => useLayoutStore.setState({ collapsed: { left: false, right: false } }));

  it("renders the three panels", () => {
    render(<App />);
    expect(screen.getByRole("complementary", { name: "Projects" })).toBeInTheDocument();
    expect(screen.getByRole("main", { name: "Workspace" })).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: "Changes" })).toBeInTheDocument();
  });

  it("toggles side panels with Mod+B and Mod+Alt+B", () => {
    render(<App />);
    fireEvent.keyDown(window, { code: "KeyB", ctrlKey: true });
    expect(useLayoutStore.getState().collapsed).toEqual({ left: true, right: false });
    fireEvent.keyDown(window, { code: "KeyB", ctrlKey: true, altKey: true });
    expect(useLayoutStore.getState().collapsed).toEqual({ left: true, right: true });
  });

  it("leaves plain B alone so the terminal keeps its keys", () => {
    render(<App />);
    fireEvent.keyDown(window, { code: "KeyB" });
    expect(useLayoutStore.getState().collapsed).toEqual({ left: false, right: false });
  });
});
