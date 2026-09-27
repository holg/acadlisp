#!/usr/bin/env bash
# Full matrix: kisp / acadlisp / xlisp, native and wasm. See docs/kisp-comparison.md.
set -euo pipefail
cd "$(dirname "$0")"
RUNS="${1:-20}"
echo "=== native: kisp + acadlisp (Rust harness)"; cargo run --release --quiet -- "$RUNS"
echo; echo "=== native: xlisp (C driver, needs xlisp/build.sh once)"
for p in fib loop_sum list_ops call_overhead cad_grid; do printf '%-14s ' "$p"; xlisp/xlisp-bench "programs/$p.xl" "$RUNS"; done
echo; echo "=== wasm: kisp + acadlisp + xlisp (Node, needs wasm/build.sh once)"; (cd wasm && node bench.mjs "$RUNS")
