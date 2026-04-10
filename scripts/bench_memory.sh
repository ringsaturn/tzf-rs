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
MODES=(rtree quad noindex)
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

python3 - "$BENCH_FILE" <<'PY'
import platform
import re
import sys
from pathlib import Path

bench_file = Path(sys.argv[1])
bench_text = bench_file.read_text()

is_darwin = platform.system() == "Darwin"
runs = 5

scenario_map = {
    "RTreeOnly": "RTree only",
    "QuadOnly": "Quad only",
    "NoIndex": "No index",
}
mode_map = {
    "RTreeOnly": "rtree",
    "QuadOnly": "quad",
    "NoIndex": "noindex",
}
config_map = {
    "RTreeOnly": "`enable_rtree=true`, `enable_compressed_quad=false`",
    "QuadOnly": "`enable_rtree=false`, `enable_compressed_quad=true`",
    "NoIndex": "`enable_rtree=false`, `enable_compressed_quad=false`",
}

def parse_peak_bytes(path: Path) -> int:
    txt = path.read_text()
    if is_darwin:
        m = re.search(r"\n\s*(\d+)\s+maximum resident set size", txt)
        if not m:
            raise RuntimeError(f"cannot parse macOS RSS from {path}")
        return int(m.group(1))
    m = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", txt)
    if not m:
        raise RuntimeError(f"cannot parse Linux RSS from {path}")
    return int(m.group(1)) * 1024

def parse_range_us(target_group: str, scenario: str):
    pattern = rf"{re.escape(target_group)}/{re.escape(scenario)}/0[\s\S]*?time:\s*\[([^\]]+)\]"
    m = re.search(pattern, bench_text)
    if not m:
        return None
    vals = []
    for n, u in re.findall(r"([0-9]+\.[0-9]+)\s*(ns|µs|ms)", m.group(1)):
        x = float(n)
        if u == "ns":
            x /= 1000.0
        elif u == "ms":
            x *= 1000.0
        vals.append(x)
    if len(vals) != 3:
        return None
    return vals

def fmt_float(x, nd=4):
    return f"{x:.{nd}f}"

def fmt_int(x):
    return f"{int(round(x)):,}"

rows = []
for target_name, target_group, target_key in [
    ("Finder", "FinderIndexModes", "finder"),
    ("DefaultFinder", "DefaultFinderIndexModes", "default"),
]:
    for scenario in ["RTreeOnly", "QuadOnly", "NoIndex"]:
        range_us = parse_range_us(target_group, scenario)
        if range_us is None:
            continue
        median = range_us[1]
        throughput = 1_000_000.0 / median

        mode = mode_map[scenario]
        rss_vals = []
        for i in range(1, runs + 1):
            p = Path(f"/tmp/tzf_mem_{target_key}_{mode}_{i}.time")
            rss_vals.append(parse_peak_bytes(p))
        avg_mib = sum(rss_vals) / len(rss_vals) / 1024 / 1024

        rows.append(
            (
                target_name,
                scenario_map[scenario],
                config_map[scenario],
                f"[{fmt_float(range_us[0])}, {fmt_float(range_us[1])}, {fmt_float(range_us[2])}]",
                fmt_float(median),
                fmt_int(throughput),
                f"{avg_mib:.2f}",
                ", ".join(str(v) for v in rss_vals),
            )
        )

print("| Target | Scenario | Config | Finder time range (µs) | Median estimate (µs) | Approx throughput (ops/s) | Avg peak RSS (MiB) | Peak RSS raw values (bytes, 5 runs) |")
print("| --- |---|---|---:|---:|---:|---:|---|")
for r in rows:
    print(f"| {r[0]} | {r[1]} | {r[2]} | {r[3]} | {r[4]} | {r[5]} | {r[6]} | {r[7]} |")
PY
