#!/usr/bin/env python3
"""check_dialog_overlay_pointer_events.py — Dialog 遮罩点击穿透回归检查

类别：6. 原型治理
Tier：T1（< 10s）
输入：packages/ui/src/ui/dialog.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- DialogOverlay 只负责视觉遮罩，不得包 DialogPrimitive.Close。
- DialogOverlay 必须包含 pointer-events-none，避免弹窗内 Select 等 portal 浮层打开后，点击另一个字段命中遮罩并关闭 Dialog。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
DIALOG_FILE = REPO_ROOT / "packages" / "ui" / "src" / "ui" / "dialog.tsx"

OVERLAY_CLOSE_RE = re.compile(
    r"export const DialogOverlay[\s\S]*?<DialogPrimitive\.Close asChild>[\s\S]*?<DialogPrimitive\.Overlay",
    re.MULTILINE,
)


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def dialog_overlay_is_pointer_safe(source: str) -> bool:
    return "pointer-events-none" in source and not OVERLAY_CLOSE_RE.search(source)


def run() -> list[str]:
    errors: list[str] = []
    if not DIALOG_FILE.exists():
        return [f"{rel(DIALOG_FILE)}: 共享 Dialog 文件不存在"]

    source = DIALOG_FILE.read_text(encoding="utf-8")
    if OVERLAY_CLOSE_RE.search(source):
        errors.append(f"{rel(DIALOG_FILE)}: DialogOverlay 不得包 DialogPrimitive.Close，关闭交给 Dialog 外部点击逻辑")
    if "pointer-events-none" not in source:
        errors.append(f"{rel(DIALOG_FILE)}: DialogOverlay 必须包含 pointer-events-none，避免遮罩抢走弹窗内字段点击")
    if "<DialogOverlay />" not in source:
        errors.append(f"{rel(DIALOG_FILE)}: DialogContent 必须继续渲染 DialogOverlay")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    try:
        errors = run()
    except Exception as e:  # noqa: BLE001
        if args.json:
            print(json.dumps({"check": "check_dialog_overlay_pointer_events", "ok": False, "error": str(e)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({
            "check": "check_dialog_overlay_pointer_events",
            "tier": "T1",
            "category": "原型治理",
            "errors": errors,
            "ok": not errors,
        }, ensure_ascii=False, indent=2))
    else:
        if errors:
            print(f"✗ check_dialog_overlay_pointer_events: {len(errors)} 项违规")
            for error in errors:
                print(f"  - {error}")
        else:
            print("✓ check_dialog_overlay_pointer_events: 通过")

    return 0 if not errors else 1


if __name__ == "__main__":
    sys.exit(main())
