#!/usr/bin/env python3
"""_tier_timing.py — 测量 T1-T4 实际耗时并写入 baselines

由 `just tier-timing` 调用。原本作为 justfile 单行 Python 命令，因 just 1.51+
不再接受 `;` 多语句分隔，提取为独立脚本。

输出：
  governance/baselines/tier-runtime.json
"""
from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import time

BUDGET_MS = {
    "T1": 10_000,
    "T2": 120_000,
    "T3": 300_000,
    "T4": 1_800_000,
}


def main() -> int:
    print("▶ measuring tier runtimes (单位: ms)")
    results: dict[str, dict] = {}
    for tier in BUDGET_MS:
        start = time.perf_counter()
        p = subprocess.run(
            [sys.executable, "scripts/governance/governance_checks.py", "--tier", tier],
            capture_output=True,
        )
        results[tier] = {
            "duration_ms": int((time.perf_counter() - start) * 1000),
            "exit_code": p.returncode,
        }
        print(f'  {tier}: {results[tier]["duration_ms"]}ms (exit={results[tier]["exit_code"]})')

    print()
    print("budget check:")
    exceeded: list[str] = []
    for t, r in results.items():
        over = r["duration_ms"] > BUDGET_MS[t]
        if over:
            exceeded.append(t)
        print(f'  {t}: {r["duration_ms"]}ms vs budget {BUDGET_MS[t]}ms — {"OVER" if over else "OK"}')

    out = pathlib.Path("governance/baselines/tier-runtime.json")
    out.write_text(
        json.dumps(
            {
                "measured_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "results": results,
                "budget_ms": BUDGET_MS,
                "exceeded": exceeded,
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )

    return 1 if exceeded else 0


if __name__ == "__main__":
    sys.exit(main())
