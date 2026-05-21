#!/usr/bin/env python3
"""check_prototype_usability_baseline.py — 原型易用性基线校验

类别：6. 原型治理
Tier：T1（< 10s）
输入：docs/prototypes/index.toml
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项（对照 docs/infra/usability-baseline.md）：
- end=pda 的条目：必须声明 touch_target_min_pt ≥ 44 且 font_size_min_pt ≥ 16（如有 viewport 段）
- 所有条目：end 字段必须存在
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
INDEX_TOML = REPO_ROOT / "docs" / "prototypes" / "index.toml"

# PDA 易用性硬指标（来自 docs/infra/usability-baseline.md §2.1）
PDA_TOUCH_MIN = 44
PDA_FONT_MIN = 16


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
    required = data.get("required", [])
    errors: list[str] = []

    for entry in required:
        sid = entry.get("story_id", "")
        slug = entry.get("prototype_slug", "")
        end = entry.get("end", "")
        prefix = f"{sid}/{slug}"

        if not end:
            errors.append(f"{prefix}: 缺少 end 字段")
            continue

        # PDA 端必须有 viewport 段且满足触控/字体最小值
        viewport = entry.get("viewport", {})
        if end == "pda":
            if not viewport:
                errors.append(f"{prefix}: PDA 端必须声明 [viewport] 段（touch_target_min_pt + font_size_min_pt）")
            else:
                touch = viewport.get("touch_target_min_pt", 0)
                font = viewport.get("font_size_min_pt", 0)
                if touch < PDA_TOUCH_MIN:
                    errors.append(f"{prefix}: touch_target_min_pt={touch} < {PDA_TOUCH_MIN}")
                if font < PDA_FONT_MIN:
                    errors.append(f"{prefix}: font_size_min_pt={font} < {PDA_FONT_MIN}")

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
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors}))
    else:
        if errors:
            print(f"✗ check_prototype_usability_baseline: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        else:
            print("✓ check_prototype_usability_baseline: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
