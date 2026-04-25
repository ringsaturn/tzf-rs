#!/usr/bin/env python3
import sys
from pathlib import Path

from bench_memory_parse import build_rows


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: bench_memory_report.py <benchmark_result.txt>", file=sys.stderr)
        return 1

    bench_file = Path(sys.argv[1])
    if not bench_file.exists():
        print(f"benchmark file not found: {bench_file}", file=sys.stderr)
        return 1

    bench_text = bench_file.read_text()
    rows = build_rows(bench_text, runs=5)

    print(
        "| Target | Dataset | Scenario | Median estimate (µs) | Approx throughput (ops/s) | Avg peak RSS (MiB) |"
    )
    print("| --- | --- | --- |---:|---:|---:|")
    for row in rows:
        # indices: 0=target 1=dataset 2=scenario 5=median 6=throughput 7=rss
        print(f"| {row[0]} | {row[1]} | {row[2]} | {row[5]} | {row[6]} | {row[7]} |")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
