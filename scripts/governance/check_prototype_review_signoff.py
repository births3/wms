#!/usr/bin/env python3
"""check_prototype_review_signoff.py — 原型走查签字校验

类别：6. 原型治理
Tier：T2（PR 阶段）
输入：docs/prototypes/index.toml
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- 当前 Wave 对应 priority 的条目，status=approved 必须有 ≥1 条 walkthrough
- walkthrough 中至少一条 result=approved
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
INDEX_TOML = REPO_ROOT / "docs" / "prototypes" / "index.toml"

# Wave → 必须 approved 的 priority 上限
WAVE_PRIORITY_MAP = {
    "0.5": {"P0"},
    "1": {"P0"},
    "1.5": {"P0", "P1"},
    "2": {"P0", "P1"},
    "2.5": {"P0", "P1", "P2"},
    "3": {"P0", "P1", "P2"},
    "3.5": {"P0", "P1", "P2", "P3"},
    "4": {"P0", "P1", "P2", "P3"},
    "4.5": {"P0", "P1", "P2", "P3", "P4"},
    "5": {"P0", "P1", "P2", "P3", "P4"},
}


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def run() -> list[str]:
    if not INDEX_TOML.exists():
        return []

    data = _load_toml(INDEX_TOML)
    current_wave = data.get("meta", {}).get("current_wave", "0.5")
    required_priorities = WAVE_PRIORITY_MAP.get(current_wave, {"P0"})
    required = data.get("required", [])
    errors: list[str] = []

    for entry in required:
        sid = entry.get("story_id", "")
        slug = entry.get("prototype_slug", "")
        priority = entry.get("priority", "")
        status = entry.get("status", "")

        # Only enforce for current wave's priorities
        if priority not in required_priorities:
            continue

        # Only enforce if status claims approved
        if status != "approved":
            continue

        prefix = f"{sid}/{slug}"
        walkthroughs = entry.get("walkthroughs", {}).get("entries", [])
        if not walkthroughs:
            errors.append(f"{prefix}: status=approved 但无走查记录")
            continue

        has_approved = any(w.get("result") == "approved" for w in walkthroughs)
        if not has_approved:
            errors.append(f"{prefix}: 走查记录中无 result=approved 的条目")

    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors = run()
    except Exception as e:
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors, "ok": not errors}))
    else:
        if errors:
            print(f"✗ check_prototype_review_signoff: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        else:
            print("✓ check_prototype_review_signoff: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
