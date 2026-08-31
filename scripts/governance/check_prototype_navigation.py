#!/usr/bin/env python3
"""check_prototype_navigation.py — ADR-0043 生产导航兼容入口。

项目已从原型导航迁移到 `apps/web-admin` 的真实菜单、AdminView、renderer 和开发菜单种子。
本入口保留 Tier 历史名称，但将约束落到 `check_admin_navigation.py`，避免恢复已废弃的
`prototypes/src/App.tsx` / `Tabs.tsx`，同时继续阻断缺菜单、缺 renderer、重复 view 等问题。
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
REPLACEMENT = SCRIPTS_DIR / "check_admin_navigation.py"


def run() -> list[str]:
    errors = replacement_contract_errors()
    if errors:
        return errors
    if not REPLACEMENT.is_file():
        return ["生产导航检查器 check_admin_navigation.py 不存在"]

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
    return [f"生产导航契约未通过: {detail[:2000]}"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
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
        print(f"✗ check_prototype_navigation: {len(errors)} 项生产导航违规")
        for error in errors:
            print(f"  - {error}")
    else:
        print("✓ check_prototype_navigation: ADR-0043 生产导航契约通过")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
