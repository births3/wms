#!/usr/bin/env python3
"""check_datagrid_popover_portal.py — DataGrid 浮层裁剪和关闭回归检查

类别：6. 原型治理
Tier：T1（< 10s）
输入：packages/ui/src/business/DataGrid/**/*.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- 带 data-datagrid-popover 的 DataGrid 浮层元素禁止使用 absolute 定位。
  DataGrid 表格容器可能有 overflow，浮层应使用 createPortal + fixed 脱离裁剪上下文。
- DataGrid 字段筛选、字段设置和命名视图浮层必须复用共享关闭 hook。
  按钮触发的门户浮层必须支持点击外部关闭和 Escape 关闭。
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
DISMISS_HOOK_FILE = DATA_GRID_DIR / "data-grid-popover-dismiss.ts"
DISMISS_HOOK_USERS = (
    DATA_GRID_DIR / "DataGrid.tsx",
    DATA_GRID_DIR / "DataGridNamedViewsToolbar.tsx",
)

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


def data_grid_dismiss_hook_is_complete(source: str) -> bool:
    required_tokens = (
        'document.addEventListener("pointerdown", dismissOnOutsidePointer);',
        'target?.closest("[data-datagrid-popover]")',
        'document.addEventListener("keydown", dismissOnEscape);',
        'event.key !== "Escape"',
    )
    return all(token in source for token in required_tokens)


def source_uses_datagrid_dismiss_hook(source: str) -> bool:
    return "useDataGridPopoverDismiss(" in source


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

    if not DISMISS_HOOK_FILE.exists():
        errors.append(
            f"{rel(DISMISS_HOOK_FILE)}: DataGrid 门户浮层必须提供共享关闭 hook，统一点击外部和 Escape 关闭"
        )
    elif not data_grid_dismiss_hook_is_complete(DISMISS_HOOK_FILE.read_text(encoding="utf-8")):
        errors.append(
            f"{rel(DISMISS_HOOK_FILE)}: 关闭 hook 必须同时监听外部 pointerdown 与 Escape，并忽略 data-datagrid-popover 内部点击"
        )

    for path in DISMISS_HOOK_USERS:
        if not path.exists():
            continue
        if not source_uses_datagrid_dismiss_hook(path.read_text(encoding="utf-8")):
            errors.append(f"{rel(path)}: DataGrid 门户浮层必须复用 useDataGridPopoverDismiss")

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
