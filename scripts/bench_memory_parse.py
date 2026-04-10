import platform
import re
from pathlib import Path

SCENARIO_MAP = {
    "RTreeOnly": "RTree only",
    "QuadOnly": "Quad only",
    "NoIndex": "No index",
}

MODE_MAP = {
    "RTreeOnly": "rtree",
    "QuadOnly": "quad",
    "NoIndex": "noindex",
}

CONFIG_MAP = {
    "RTreeOnly": "`enable_rtree=true`, `enable_compressed_quad=false`",
    "QuadOnly": "`enable_rtree=false`, `enable_compressed_quad=true`",
    "NoIndex": "`enable_rtree=false`, `enable_compressed_quad=false`",
}

TARGETS = [
    ("Finder", "FinderIndexModes", "finder"),
    ("DefaultFinder", "DefaultFinderIndexModes", "default"),
]

SCENARIOS = ["RTreeOnly", "QuadOnly", "NoIndex"]


def parse_peak_bytes(path: Path, is_darwin: bool) -> int:
    txt = path.read_text()
    if is_darwin:
        match = re.search(r"\n\s*(\d+)\s+maximum resident set size", txt)
        if not match:
            raise RuntimeError(f"cannot parse macOS RSS from {path}")
        return int(match.group(1))

    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", txt)
    if not match:
        raise RuntimeError(f"cannot parse Linux RSS from {path}")
    return int(match.group(1)) * 1024


def parse_range_us(bench_text: str, target_group: str, scenario: str):
    pattern = rf"{re.escape(target_group)}/{re.escape(scenario)}/0[\s\S]*?time:\s*\[([^\]]+)\]"
    match = re.search(pattern, bench_text)
    if not match:
        return None

    values = []
    for number, unit in re.findall(r"([0-9]+\.[0-9]+)\s*(ns|µs|ms)", match.group(1)):
        value = float(number)
        if unit == "ns":
            value /= 1000.0
        elif unit == "ms":
            value *= 1000.0
        values.append(value)

    if len(values) != 3:
        return None
    return values


def fmt_float(value: float, ndigits: int = 4) -> str:
    return f"{value:.{ndigits}f}"


def fmt_int(value: float) -> str:
    return f"{int(round(value)):,}"


def build_rows(bench_text: str, runs: int = 5):
    rows = []
    is_darwin = platform.system() == "Darwin"

    for target_name, target_group, target_key in TARGETS:
        for scenario in SCENARIOS:
            range_us = parse_range_us(bench_text, target_group, scenario)
            if range_us is None:
                continue

            median = range_us[1]
            throughput = 1_000_000.0 / median
            mode = MODE_MAP[scenario]

            rss_values = []
            for i in range(1, runs + 1):
                path = Path(f"/tmp/tzf_mem_{target_key}_{mode}_{i}.time")
                rss_values.append(parse_peak_bytes(path, is_darwin))

            avg_mib = sum(rss_values) / len(rss_values) / 1024 / 1024

            rows.append(
                (
                    target_name,
                    SCENARIO_MAP[scenario],
                    CONFIG_MAP[scenario],
                    f"[{fmt_float(range_us[0])}, {fmt_float(range_us[1])}, {fmt_float(range_us[2])}]",
                    fmt_float(median),
                    fmt_int(throughput),
                    f"{avg_mib:.2f}",
                    ", ".join(str(v) for v in rss_values),
                )
            )

    return rows
