#!/usr/bin/env python3
"""Repaint a 120x40 screen of 256-colour cells 600 times at up to 120 Hz.

This is what a streaming full-screen TUI does to a terminal — home the cursor and redraw
everything — and it is far harder on the renderer than scrolling text.
"""
import random
import sys
import time

ROWS, COLS, FRAMES, HZ = 40, 120, 600, 120


def main() -> None:
    random.seed(1)
    for frame in range(FRAMES):
        screen = ["\x1b[H"]
        for row in range(ROWS):
            cells = "".join(
                "\x1b[38;5;%dm%08x" % ((frame + row + col) % 230 + 1, random.getrandbits(32))
                for col in range(0, COLS, 8)
            )
            screen.append(cells + "\x1b[0m" + ("\x1b[K\r\n" if row < ROWS - 1 else ""))
        sys.stdout.write("".join(screen))
        sys.stdout.flush()
        time.sleep(1 / HZ)


if __name__ == "__main__":
    main()
