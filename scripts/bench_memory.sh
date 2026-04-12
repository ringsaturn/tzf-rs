#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BENCH_FILE="${1:-benchmark_result.txt}"
if [[ ! -f "$BENCH_FILE" ]]; then
  echo "benchmark file not found: $BENCH_FILE" >&2
  exit 1
fi

TARGETS=(finder default)
MODES=(ystripes noindex)
RUNS=5

find /tmp -maxdepth 1 -type f -name "tzf_mem_*" -delete || true

for target in "${TARGETS[@]}"; do
  for mode in "${MODES[@]}"; do
    for ((i=1; i<=RUNS; i++)); do
      out_file="/tmp/tzf_mem_${target}_${mode}_${i}.out"
      time_file="/tmp/tzf_mem_${target}_${mode}_${i}.time"

      if [[ "$(uname)" == "Darwin" ]]; then
        /usr/bin/time -l cargo run --release --example index_memory_probe -- "$target" "$mode" >"$out_file" 2>"$time_file"
      else
        /usr/bin/time -v cargo run --release --example index_memory_probe -- "$target" "$mode" >"$out_file" 2>"$time_file"
      fi
    done
  done
done

python3 "$ROOT_DIR/scripts/bench_memory_report.py" "$BENCH_FILE"
