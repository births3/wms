#!/usr/bin/env python3
"""check_baseline_completeness.py — ADR-0043 真实页面证据完整性兼容入口。

ADR-0043 明确禁止为新故事继续新增原型 baseline；生产前端验收必须由真实 HTTP + PostgreSQL
的 Playwright 业务断言、刷新回读和截图承接。因此原有 Tabs ↔ manifest ↔ PNG 三方原型
baseline 门禁迁移为 `check_scope_gap_discovery.py` 的真实页面 E2E 截图证据门禁。

这不是跳过视觉/页面证据：如果生产页面缺真实 E2E 或截图，本检查仍会失败。
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

from _direct_production_frontend import replacement_contract_errors

_THIS = Path(__file__).resolve()
SCRIPTS_DIR = _THIS.parent
REPO_ROOT = _THIS.parent.parent.parent
REPLACEMENT = SCRIPTS_DIR / "check_scope_gap_discovery.py"


def run() -> list[str]:
    errors = replacement_contract_errors()
    if errors:
        return errors
    if not REPLACEMENT.is_file():
        return ["真实页面证据检查器 check_scope_gap_discovery.py 不存在"]

    result = subprocess.run(
        [sys.executable, str(REPLACEMENT), "--json"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0:
        return []
    detail = result.stdout.strip() or result.stderr.strip() or f"exit={result.returncode}"
    return [f"生产页面真实 E2E/截图证据未闭环: {detail[:3000]}"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--base", default=None, help="保留历史 CLI 兼容；ADR-0043 模式由质量矩阵决定证据范围")
    args = parser.parse_args()
    try:
        errors = run()
    except Exception as exc:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(exc)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {exc}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors, "ok": not errors}, ensure_ascii=False))
    elif errors:
        print(f"✗ check_baseline_completeness: {len(errors)} 项生产证据缺口")
        for error in errors:
            print(f"  - {error}")
    else:
        print("✓ check_baseline_completeness: ADR-0043 真实页面 E2E/截图证据完整")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
