#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BENCH_FILE="${1:-benchmark_full_result.txt}"
if [[ ! -f "$BENCH_FILE" ]]; then
  echo "benchmark file not found: $BENCH_FILE" >&2
  exit 1
fi

RUNS=5

find /tmp -maxdepth 1 -type f -name "tzf_mem_full_*" -delete || true

for ((i=1; i<=RUNS; i++)); do
  out_file="/tmp/tzf_mem_full_${i}.out"
  time_file="/tmp/tzf_mem_full_${i}.time"

  if [[ "$(uname)" == "Darwin" ]]; then
    /usr/bin/time -l cargo run --release --no-default-features --features full \
      --example index_memory_probe_full >"$out_file" 2>"$time_file"
  else
    /usr/bin/time -v cargo run --release --no-default-features --features full \
      --example index_memory_probe_full >"$out_file" 2>"$time_file"
  fi
done

python3 "$ROOT_DIR/scripts/bench_memory_full_report.py" "$BENCH_FILE"
