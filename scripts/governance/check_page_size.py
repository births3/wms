#!/usr/bin/env python3
"""check_page_size.py — 页面/原型支撑组件文件大小约束（600 警告 / 800 门禁）

类别：6. 原型治理
Tier：T1（< 10s）
输入：prototypes/src/pages/**/*.tsx + prototypes/src/prototype-kit/**/*.tsx + apps/*/src/pages/**/*.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规（≥ 800 行）/ 2 脚本错误

校验项（对照 docs/frontend-coding-standards.md §页面级大小约束）：
- 单页面文件 ≥ 600 行 → warning（提示提取组件）
- 单页面文件 ≥ 800 行 → error（强制提取组件）
- 原型运行时支撑组件同样受约束，防止 UniversalPrototypePage 类模板绕过治理

豁免方式：文件顶部加 `@governance: skip-page-size` 注释 + 理由

不覆盖：
- 单组件复杂度（应交由 cyclomatic complexity 检查）
- 跨文件累计行数
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
PAGE_DIRS = (
    REPO_ROOT / "prototypes" / "src" / "pages",
    REPO_ROOT / "prototypes" / "src" / "prototype-kit",
    REPO_ROOT / "apps" / "web-admin" / "src" / "pages",
    REPO_ROOT / "apps" / "pda-mobile" / "src" / "pages",
)

WARN_THRESHOLD = 600
ERROR_THRESHOLD = 800
SKIP_TAG = "@governance: skip-page-size"


def _count_effective_lines(path: Path) -> int:
    """计算有效代码行（去掉空行 + 纯注释行，但保留 JSDoc 头部计入"""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    return len([l for l in lines if l.strip()])


def _check_file(path: Path) -> tuple[int, str | None, bool]:
    """Returns (line_count, severity, is_error)"""
    text = path.read_text(encoding="utf-8")
    if SKIP_TAG in text[:500]:
        return (0, None, False)  # 豁免
    lines = _count_effective_lines(path)
    if lines >= ERROR_THRESHOLD:
        return (lines, "error", True)
    if lines >= WARN_THRESHOLD:
        return (lines, "warning", False)
    return (lines, None, False)


def run() -> tuple[list[str], list[str]]:
    """Returns (errors, warnings)"""
    errors: list[str] = []
    warnings: list[str] = []
    files: list[Path] = []
    for page_dir in PAGE_DIRS:
        if page_dir.exists():
            files.extend(page_dir.rglob("*.tsx"))
    for f in sorted(files):
        if ".stories." in f.name or ".spec." in f.name or ".test." in f.name:
            continue
        lines, severity, is_error = _check_file(f)
        rel = f.relative_to(REPO_ROOT).as_posix()
        if is_error:
            errors.append(f"{rel}: {lines} 行 ≥ {ERROR_THRESHOLD}（门禁，必须提取组件或加 {SKIP_TAG} 豁免）")
        elif severity == "warning":
            warnings.append(f"{rel}: {lines} 行 ≥ {WARN_THRESHOLD}（警告，建议提取 PageHeader/DataTable/FilterBar 等）")
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
            "thresholds": {"warning": WARN_THRESHOLD, "error": ERROR_THRESHOLD},
        }))
    else:
        if errors:
            print(f"✗ check_page_size: {len(errors)} 项门禁违规")
            for e in errors:
                print(f"  - {e}")
        if warnings:
            tag = "⚠" if not errors else " "
            print(f"{tag} check_page_size: {len(warnings)} 项警告")
            for w in warnings:
                print(f"  - {w}")
        if not errors and not warnings:
            print(f"✓ check_page_size: 通过（阈值 {WARN_THRESHOLD}/{ERROR_THRESHOLD}）")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
