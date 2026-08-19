#!/usr/bin/env python3
"""check_prototype_freshness.py — 原型新鲜度校验

类别：6. 原型治理
Tier：T1（< 10s）
输入：docs/prototypes/index.toml
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- 有 last_reviewed_at 的条目：距今 ≤ 90 天（超过报 warning，不阻断）
- priority=P0 且 status=approved 的条目必须有 last_reviewed_at
"""
from __future__ import annotations

import argparse
import datetime
import json
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
INDEX_TOML = REPO_ROOT / "docs" / "prototypes" / "index.toml"

MAX_DAYS = 90


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def run() -> tuple[list[str], list[str]]:
    if not INDEX_TOML.exists():
        return ([], [])

    data = _load_toml(INDEX_TOML)
    required = data.get("required", [])
    errors: list[str] = []
    warnings: list[str] = []
    today = datetime.date.today()

    for entry in required:
        sid = entry.get("story_id", "")
        slug = entry.get("prototype_slug", "")
        priority = entry.get("priority", "")
        status = entry.get("status", "")
        last_reviewed = entry.get("last_reviewed_at", "")

        prefix = f"{sid}/{slug}"

        if priority == "P0" and status == "approved" and not last_reviewed:
            errors.append(f"{prefix}: P0+approved 必须有 last_reviewed_at")
            continue

        if last_reviewed:
            try:
                reviewed_date = datetime.date.fromisoformat(last_reviewed)
                age = (today - reviewed_date).days
                if age > MAX_DAYS:
                    warnings.append(f"{prefix}: last_reviewed_at 距今 {age} 天（>{MAX_DAYS}），建议重新走查")
            except ValueError:
                errors.append(f"{prefix}: last_reviewed_at 格式错误 '{last_reviewed}'")

    return (errors, warnings)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors, warnings = run()
    except Exception as e:
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({
            "status": "fail" if errors else "pass",
            "errors": errors,
            "warnings": warnings,
            "ok": not errors,
        }))
    else:
        if errors:
            print(f"✗ check_prototype_freshness: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        elif warnings:
            print(f"⚠ check_prototype_freshness: 通过（{len(warnings)} 项 warning）")
            for w in warnings:
                print(f"  - {w}")
        else:
            print("✓ check_prototype_freshness: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
