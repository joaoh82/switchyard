import type { ITheme } from "@xterm/xterm";

/** Matches the app's dark surface; ANSI colours tuned for contrast on it. */
export const darkTheme: ITheme = {
  background: "#0f1115",
  foreground: "#e8eaed",
  cursor: "#f5b83d",
  cursorAccent: "#0f1115",
  selectionBackground: "#f5b83d44",
  black: "#1b1f26",
  red: "#f7768e",
  green: "#9ece6a",
  yellow: "#e0af68",
  blue: "#7aa2f7",
  magenta: "#bb9af7",
  cyan: "#7dcfff",
  white: "#c0caf5",
  brightBlack: "#565f89",
  brightRed: "#ff899d",
  brightGreen: "#b3e07f",
  brightYellow: "#f0c583",
  brightBlue: "#93b4ff",
  brightMagenta: "#cbb0ff",
  brightCyan: "#a4e1ff",
  brightWhite: "#ffffff",
};

export const lightTheme: ITheme = {
  background: "#f4f5f7",
  foreground: "#14171c",
  cursor: "#b77a00",
  cursorAccent: "#f4f5f7",
  selectionBackground: "#b77a0033",
  black: "#14171c",
  red: "#c4314b",
  green: "#3d7a1f",
  yellow: "#8f5e00",
  blue: "#2a5bd7",
  magenta: "#7c3aed",
  cyan: "#0e7490",
  white: "#6b7380",
  brightBlack: "#4b5563",
  brightRed: "#e0435f",
  brightGreen: "#4e9a27",
  brightYellow: "#b77a00",
  brightBlue: "#3b6ff0",
  brightMagenta: "#9155ff",
  brightCyan: "#1290b0",
  brightWhite: "#9aa3af",
};

export const FONT_FAMILY =
  '"JetBrains Mono", "JetBrainsMono Nerd Font", ui-monospace, "Cascadia Code", "SF Mono", Menlo, Consolas, monospace';
