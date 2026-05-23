#!/usr/bin/env python3
"""check_component_registry_consistency.py — 业务复合组件注册表一致性

类别：6. 原型治理
Tier：T1（< 10s）
输入：docs/prototypes/component-registry.md + packages/ui/src/business/
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项（对照 ADR-0022 §7）：
- component-registry.md §3.1 表格中每个组件 → business/ 下有同名目录
- business/ 下每个组件目录 → registry §3.1 已注册
- 例外：governance/check-data.toml [[component_exemptions]] 显式豁免
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
REGISTRY_MD = REPO_ROOT / "docs" / "prototypes" / "component-registry.md"
BUSINESS_DIR = REPO_ROOT / "packages" / "ui" / "src" / "business"
CHECK_DATA_TOML = REPO_ROOT / "governance" / "check-data.toml"

# 匹配 markdown 表格行：| # | **<Name>** | 端 | 职责 | 故事 | 状态 |
REGISTRY_ROW_RE = re.compile(r"\|\s*\d+\s*\|\s*\*\*(\w+)\*\*\s*\|[^|]*\|[^|]*\|[^|]*\|\s*([^|]+?)\s*\|", re.MULTILINE)


def _load_exemptions() -> set[str]:
    if not CHECK_DATA_TOML.exists():
        return set()
    text = CHECK_DATA_TOML.read_text(encoding="utf-8")
    try:
        import tomllib
        data = tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        data = tomli.loads(text)
    return {r["component_name"] for r in data.get("component_exemptions", [])}


def _registered() -> dict[str, str]:
    """返回 {component_name: status}"""
    if not REGISTRY_MD.exists():
        return {}
    text = REGISTRY_MD.read_text(encoding="utf-8")
    return {m.group(1): m.group(2).strip() for m in REGISTRY_ROW_RE.finditer(text)}


def _existing_dirs() -> set[str]:
    if not BUSINESS_DIR.exists():
        return set()
    return {p.name for p in BUSINESS_DIR.iterdir() if p.is_dir()}


def run() -> list[str]:
    errors: list[str] = []
    exemptions = _load_exemptions()
    registered = _registered()
    existing = _existing_dirs()

    # 1) 注册了 status=已开发 但目录不存在 → 报错
    #    status=待开发 → OK（计划中）
    for name, status in sorted(registered.items()):
        if name in exemptions:
            continue
        if status in ("已开发", "已交付"):
            if name not in existing:
                errors.append(f"已注册（{status}）但目录不存在: {name}")
        # 待开发 / 设计中 / 计划中 → 允许目录尚未存在

    # 2) 目录存在但未注册 → 报错
    for name in sorted(existing - set(registered.keys()) - exemptions):
        errors.append(f"目录存在但未注册: {name}（须在 component-registry.md §3.1 添加）")

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
            print(f"✗ check_component_registry_consistency: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        else:
            print("✓ check_component_registry_consistency: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
