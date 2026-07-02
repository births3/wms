#!/usr/bin/env python3
"""check_datagrid_popover_portal.py — DataGrid 浮层裁剪回归检查

类别：6. 原型治理
Tier：T1（< 10s）
输入：packages/ui/src/business/DataGrid/**/*.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- 带 data-datagrid-popover 的 DataGrid 浮层元素禁止使用 absolute 定位。
  DataGrid 表格容器可能有 overflow，浮层应使用 createPortal + fixed 脱离裁剪上下文。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
DATA_GRID_DIR = REPO_ROOT / "packages" / "ui" / "src" / "business" / "DataGrid"

TAG_RE = re.compile(r"<[A-Za-z][\w.]*\b[^>]*data-datagrid-popover[^>]*>", re.DOTALL)
CLASSNAME_RE = re.compile(r"className\s*=\s*(\"[^\"]*\"|\{[^}]*\})", re.DOTALL)
ABSOLUTE_RE = re.compile(r"\babsolute\b")


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def line_no(text: str, index: int) -> int:
    return text.count("\n", 0, index) + 1


def popover_tag_uses_absolute(tag: str) -> bool:
    class_match = CLASSNAME_RE.search(tag)
    return bool(class_match and ABSOLUTE_RE.search(class_match.group(1)))


def run() -> list[str]:
    errors: list[str] = []
    if not DATA_GRID_DIR.exists():
        return errors

    for path in sorted(DATA_GRID_DIR.rglob("*.tsx")):
        text = path.read_text(encoding="utf-8")
        for match in TAG_RE.finditer(text):
            if popover_tag_uses_absolute(match.group(0)):
                errors.append(
                    f"{rel(path)}:L{line_no(text, match.start())}: "
                    "DataGrid 浮层禁止使用 absolute；请用 createPortal + fixed，避免低行数表格裁剪弹窗"
                )

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    try:
        errors = run()
    except Exception as e:  # noqa: BLE001
        if args.json:
            print(json.dumps({"check": "check_datagrid_popover_portal", "ok": False, "error": str(e)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({
            "check": "check_datagrid_popover_portal",
            "tier": "T1",
            "category": "原型治理",
            "errors": errors,
            "ok": not errors,
        }, ensure_ascii=False, indent=2))
    else:
        if errors:
            print(f"✗ check_datagrid_popover_portal: {len(errors)} 项违规")
            for error in errors:
                print(f"  - {error}")
        else:
            print("✓ check_datagrid_popover_portal: 通过")

    return 0 if not errors else 1


if __name__ == "__main__":
    sys.exit(main())
