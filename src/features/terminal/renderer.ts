import { WebglAddon } from "@xterm/addon-webgl";
import type { Terminal } from "@xterm/xterm";

export type RendererKind = "webgl" | "dom";
export type RendererPreference = RendererKind | "auto";

const STORAGE_KEY = "yardsort.terminal.renderer";

export function rendererPreference(override?: string | null): RendererPreference {
  const value = override ?? localStorage.getItem(STORAGE_KEY);
  return value === "webgl" || value === "dom" ? value : "auto";
}

/**
 * Give `term` the fastest renderer that works here. WebGL is several times faster than xterm's
 * built-in DOM renderer, but system webviews — WebKitGTK above all — can refuse to create a
 * context, or lose it later (GPU reset, suspend). Either way we fall back to the DOM renderer
 * instead of leaving a dead terminal. Must be called after `term.open()`.
 */
export function attachRenderer(
  term: Terminal,
  preference: RendererPreference,
  onChange: (kind: RendererKind) => void,
): void {
  if (preference === "dom") return onChange("dom");
  try {
    const webgl = new WebglAddon();
    webgl.onContextLoss(() => {
      // Disposing the addon returns the terminal to its built-in DOM renderer.
      webgl.dispose();
      onChange("dom");
    });
    term.loadAddon(webgl);
    onChange("webgl");
  } catch (error) {
    console.warn("WebGL terminal renderer unavailable; using the DOM renderer.", error);
    onChange("dom");
  }
}
