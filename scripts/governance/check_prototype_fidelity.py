#!/usr/bin/env python3
"""check_prototype_fidelity.py — ADR-0043 生产前端保真度兼容入口。

ADR-0043 已停止新增独立原型页；业务保真度现在必须由真实生产页面的设计契约承担。
本入口保留原 Tier 名称以维持治理历史和 59 项统计稳定，但不再要求已废弃的
`prototypes/src/prototype-kit/*`。它将旧的“原型保真度”约束迁移到
`check_admin_page_design_contract.py`，并先验证 ADR-0043 的替代契约完整。
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
REPLACEMENT = SCRIPTS_DIR / "check_admin_page_design_contract.py"


def run() -> list[str]:
    errors = replacement_contract_errors()
    if errors:
        return errors
    if not REPLACEMENT.is_file():
        return ["生产前端设计契约检查器 check_admin_page_design_contract.py 不存在"]

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
    return [f"生产前端设计契约未通过: {detail[:2000]}"]


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
        print(f"✗ check_prototype_fidelity: {len(errors)} 项生产前端保真度违规")
        for error in errors:
            print(f"  - {error}")
    else:
        print("✓ check_prototype_fidelity: ADR-0043 生产前端设计契约通过")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
