#!/usr/bin/env python3
"""check_component_registry_consistency.py — 业务复合组件生产注册一致性。

类别：6. 前端治理
Tier：T1（< 10s）
输入：packages/ui/src/business/ + packages/ui/src/business/index.ts
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

ADR-0043 已废止原型先行流程，ADR-0028 的组件分层仍然有效。因此组件的权威
注册表应是生产包 `@wms/ui/business` 的 barrel export，而不是已退出生产流程的
`docs/prototypes/component-registry.md`。

校验项：
- business/ 下每个组件目录都必须由 business/index.ts 导出；
- business/index.ts 中每个相对组件导出都必须对应真实目录；
- governance/check-data.toml [[component_exemptions]] 仍可用于明确例外。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
BUSINESS_DIR = REPO_ROOT / "packages" / "ui" / "src" / "business"
BARREL_FILE = BUSINESS_DIR / "index.ts"
CHECK_DATA_TOML = REPO_ROOT / "governance" / "check-data.toml"

EXPORT_RE = re.compile(r'\bfrom\s+["\']\./([A-Za-z0-9_]+)["\']')


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
    return {str(row["component_name"]) for row in data.get("component_exemptions", [])}


def _existing_dirs() -> set[str]:
    if not BUSINESS_DIR.exists():
        return set()
    return {path.name for path in BUSINESS_DIR.iterdir() if path.is_dir()}


def _registered_exports() -> set[str]:
    if not BARREL_FILE.exists():
        return set()
    return set(EXPORT_RE.findall(BARREL_FILE.read_text(encoding="utf-8")))


def run() -> list[str]:
    errors: list[str] = []
    if not BUSINESS_DIR.is_dir():
        return ["packages/ui/src/business 不存在"]
    if not BARREL_FILE.is_file():
        return ["packages/ui/src/business/index.ts 不存在"]

    exemptions = _load_exemptions()
    existing = _existing_dirs() - exemptions
    registered = _registered_exports() - exemptions

    for name in sorted(existing - registered):
        errors.append(f"组件目录存在但未从 @wms/ui/business 导出: {name}")
    for name in sorted(registered - existing):
        errors.append(f"business/index.ts 导出了不存在的组件目录: {name}")

    return errors


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors = run()
    except Exception as exc:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(exc)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {exc}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors, "ok": not errors}, ensure_ascii=False))
    elif errors:
        print(f"✗ check_component_registry_consistency: {len(errors)} 项违规")
        for error in errors:
            print(f"  - {error}")
    else:
        print("✓ check_component_registry_consistency: 生产 barrel 与组件目录一致")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
