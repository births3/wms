#!/usr/bin/env python3
"""check_admin_datagrid_system_fields.py — 管理端 DataGrid 系统字段检查

类别：6. 前端治理
Tier：T1（< 10s，纯静态扫描）
输入：apps/web-admin/src/pages/**/*.tsx
输出：人类可读 + --json
退出码：
  0  DataGrid 列定义包含创建时间
  1  DataGrid 列定义缺少创建时间
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
PAGES_DIR = REPO_ROOT / "apps" / "web-admin" / "src" / "pages"


@dataclass(frozen=True)
class Issue:
    file: str
    message: str


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def page_files() -> list[Path]:
    if not PAGES_DIR.exists():
        return []
    return sorted(PAGES_DIR.rglob("*.tsx"))


def has_datagrid(text: str) -> bool:
    return "<DataGrid" in text and "DataGridColumn" in text


def has_created_time_column(text: str) -> bool:
    return bool(re.search(r'key:\s*["\'](?:createdAt|created_at)["\']', text)) and "创建时间" in text


def scan() -> list[Issue]:
    issues: list[Issue] = []
    for path in page_files():
        text = path.read_text(encoding="utf-8")
        if not has_datagrid(text):
            continue
        if not has_created_time_column(text):
            issues.append(Issue(rel(path), "管理端 DataGrid 缺少创建时间列（key: createdAt/created_at，header: 创建时间）"))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    issues = scan()
    payload = {
        "check": "check_admin_datagrid_system_fields",
        "tier": "T1",
        "category": "前端治理",
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_admin_datagrid_system_fields (T1, 前端治理)")
        if issues:
            print(f"  ✘ {len(issues)} 个 DataGrid 缺少系统字段:")
            for issue in issues:
                print(f"    - {issue.file}: {issue.message}")
        else:
            print("  ✓ 管理端 DataGrid 均包含创建时间列")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
