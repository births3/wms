#!/usr/bin/env python3
"""检查 Python 未使用导入、变量和失效名称。"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
RULES = "F401,F811,F821,F822,F823,F841"


def run_check() -> dict:
    result = subprocess.run(
        [
            "ruff",
            "check",
            "scripts/agents",
            "scripts/governance",
            "--select",
            RULES,
            "--output-format",
            "concise",
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    output = result.stdout if result.stdout.strip() else result.stderr
    return {
        "check": "check_dead_code",
        "tier": "T1",
        "category": "代码治理",
        "ok": result.returncode == 0,
        "violations": []
        if result.returncode == 0
        else [line for line in output.splitlines() if line.strip()],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    payload = run_check()
    if args.json:
        print(json.dumps(payload, ensure_ascii=False))
    elif payload["ok"]:
        print("✓ check_dead_code: Python 未使用代码检查通过")
    else:
        print("✗ check_dead_code: 发现未使用或失效代码")
        for violation in payload["violations"]:
            print(f"  - {violation}")
    return 0 if payload["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
