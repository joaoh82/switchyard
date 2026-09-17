#!/bin/sh
# Run the terminal benchmarks against a dev build and print one JSON result per run.
# See docs/07-terminal-benchmarks.md.
#
# usage: scripts/bench/run.sh            # both workloads x both renderers
set -eu
cd "$(dirname "$0")/../.."
here="$(pwd)/scripts/bench"
work="${TMPDIR:-/tmp}/switchyard-bench"
mkdir -p "$work"
[ -f "$work/big.txt" ] || python3 "$here/make-big-file.py" "$work/big.txt"

for workload in "cat $work/big.txt" "python3 $here/repaint.py"; do
  for renderer in webgl dom; do
    SWITCHYARD_RENDERER="$renderer" SWITCHYARD_BENCH="$workload" bun tauri dev 2>&1 |
      grep -a --line-buffered 'SWITCHYARD_BENCH_RESULT' | head -1
  done
done
