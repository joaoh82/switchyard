#!/usr/bin/env python3
"""Write ~24 MB of coloured, log-like text: the payload for the throughput benchmark.

usage: make-big-file.py <output-path>
"""
import random
import sys

WORDS = (
    "the quick brown fox jumps over lazy dog fn let mut impl struct async await "
    "return match error warning Compiling src/main.rs"
).split()


def main(path: str, size: int = 24 * 1024 * 1024) -> None:
    random.seed(7)  # same bytes every time, so runs are comparable
    written = 0
    with open(path, "w") as out:
        while written < size:
            text = " ".join(random.choice(WORDS) for _ in range(random.randint(6, 18)))
            line = f"\x1b[3{random.randint(1, 7)}m[{written:>9}]\x1b[0m {text}\n"
            out.write(line)
            written += len(line)


if __name__ == "__main__":
    main(sys.argv[1])
