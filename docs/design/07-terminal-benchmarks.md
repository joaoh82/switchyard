# 07 — Terminal benchmarks

M1's exit criterion asks for a recorded go/no-go on terminal rendering in the system webview, with
numbers. This is that record. **Verdict: go** — see [conclusions](#conclusions).

## How to run

```sh
scripts/bench/run.sh
```

A debug build started with `YARDSORT_BENCH='<shell script>'` opens one terminal, runs the script in
it, records `requestAnimationFrame` intervals from the first byte of output to the last, prints a
`YARDSORT_BENCH_RESULT {json}` line and quits. `YARDSORT_RENDERER=webgl|dom` forces the
renderer. The window must be visible: webviews stop animation frames when hidden.

Two workloads:

| Workload    | What it is                                                                                         | What it stresses                                 |
| ----------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| **cat**     | `cat` a 24 MB file of coloured log lines                                                           | PTY → IPC → parser throughput; UI liveness       |
| **repaint** | `scripts/bench/repaint.py`: redraw a 120×40 screen of 256-colour cells, 600 frames at up to 120 Hz | The renderer — this is what a streaming TUI does |

What to look at: `frameMs.p95`/`p99` and `framesOver50ms` say whether the UI stayed smooth while
output was flowing; `megabytesPerSecond` says how fast a flood is absorbed.

## Results

### Linux — the hard case

Arch (Omarchy), Hyprland/Wayland, WebKitGTK 2.52.6, hybrid NVIDIA RTX 3070 Ti + AMD Radeon 680M,
devicePixelRatio 2, debug build, 2026-09-17.

| Workload | Renderer | MB/s | fps | frame p50 | p95   | p99   | max    | frames > 50 ms |
| -------- | -------- | ---- | --- | --------- | ----- | ----- | ------ | -------------- |
| cat      | webgl    | 6.3  | 171 | 6 ms      | 14 ms | 17 ms | 79 ms  | 1              |
| cat      | dom      | 6.3  | 160 | 6 ms      | 16 ms | 20 ms | 79 ms  | 1              |
| repaint  | webgl    | 1.3  | 71  | 16 ms     | 18 ms | 35 ms | 169 ms | 2              |
| repaint  | dom      | 1.3  | 73  | 17 ms     | 17 ms | 29 ms | 38 ms  | 0              |

(`repaint` is rate-limited by the script, so its MB/s is the offered load, not a ceiling.)

Also verified by hand on the same machine: bash with a starship prompt, truecolor, `ls` colours;
Claude Code's full-screen TUI; plain Ctrl+B / Alt+X reaching the program; Ctrl+Shift shortcuts not
reaching it; and detach → re-attach of a running Claude Code session with 60 lines of scrollback —
the repainted screen differed from the original by 39 pixels (the blinking cursor).

### macOS, Windows

Not yet measured on hardware. CI runs the PTY host's integration tests (real processes in real
PTYs, including ConPTY) on both. Run `scripts/bench/run.sh` on each before v0.1 and add the rows.

## Conclusions

1. **WebGL works on WebKitGTK here**, on the GPU/compositor combination known for trouble, without
   `WEBKIT_DISABLE_DMABUF_RENDERER` or any other workaround.
2. **The DOM fallback is good enough to rely on.** For agent-shaped workloads it is
   indistinguishable from WebGL, so losing the GL context degrades nothing the user would notice.
3. **The UI stays live under flood.** 24 MB in 3.7 s with p99 frame time ≤ 20 ms and a single long
   frame. Agents produce kilobytes per second; this is three orders of magnitude of headroom.
4. Throughput is identical across renderers, so it is bound by parsing and IPC, not painting. If it
   ever matters, the lever is there (bigger batches, a release build), not in the renderer.

No reason to reconsider Tauri.

## Notes

- In `tauri dev`, a page reload while IPC requests are in flight logs _"IPC custom protocol failed,
  Tauri will now use the postMessage interface"_. It is a dev-only artefact of the reload (it does
  not occur on a clean start, with or without the CSP) and the fallback is fully functional.
