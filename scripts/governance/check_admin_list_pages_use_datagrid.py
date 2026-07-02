#!/usr/bin/env python3
"""check_admin_list_pages_use_datagrid.py — 管理端列表页 DataGrid 使用检查

类别：6. 前端治理
Tier：T1（< 10s，纯静态扫描）
输入：apps/web-admin/src/pages/**/*Page.tsx
输出：人类可读 + --json
退出码：
  0  管理端页面级列表未使用旧 DataTable
  1  管理端页面级列表仍使用旧 DataTable
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
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


def scan() -> list[Issue]:
    issues: list[Issue] = []
    for path in sorted(PAGES_DIR.rglob("*Page.tsx")):
        text = path.read_text(encoding="utf-8")
        if "DataTable" in text or "<DataTable" in text:
            issues.append(Issue(rel(path), "管理端页面级列表必须使用 DataGrid，不再直接使用 DataTable"))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    issues = scan()
    payload = {
        "check": "check_admin_list_pages_use_datagrid",
        "tier": "T1",
        "category": "前端治理",
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_admin_list_pages_use_datagrid (T1, 前端治理)")
        if issues:
            print(f"  ✘ {len(issues)} 个管理端列表页未使用 DataGrid:")
            for issue in issues:
                print(f"    - {issue.file}: {issue.message}")
        else:
            print("  ✓ 管理端页面级列表均使用 DataGrid")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
