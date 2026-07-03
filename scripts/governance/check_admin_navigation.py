#!/usr/bin/env python3
"""check_admin_navigation.py — 管理端核心模块导航可见性检查

类别：6. 前端治理
Tier：T1（< 10s，纯静态扫描）
输入：apps/web-admin/src/App.tsx
输出：人类可读 + --json
退出码：
  0  管理端核心模块在左侧导航中可见
  1  导航入口缺失
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
APP_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "App.tsx"
PAGE_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "pages" / "master-data" / "M1MasterDataPage.tsx"
QUERY_TS = (
    REPO_ROOT
    / "apps"
    / "web-admin"
    / "src"
    / "features"
    / "master-data"
    / "master-data-queries.ts"
)

REQUIRED_SECTION_LABEL = 'label: "基础档案"'
REQUIRED_ROUTE_MARKERS = (
    "M1MasterDataPage",
    "masterDataViewToId(view)",
    'onBack={() => setView("dashboard")}',
)

REQUIRED_NAV_ITEMS = (
    ("m1-products", "M1 商品档案"),
    ("m1-business-partners", "M1 客商档案"),
    ("m1-warehouses", "M1 仓库管理"),
    ("m1-locations", "M1 库位管理"),
    ("m1-system-dictionary", "M1 系统字典"),
)


@dataclass
class Issue:
    file: str
    message: str


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def scan() -> list[Issue]:
    if not APP_TSX.exists():
        return [Issue(rel(APP_TSX), "Web Admin 入口文件不存在")]

    text = APP_TSX.read_text(encoding="utf-8")
    issues: list[Issue] = []
    if REQUIRED_SECTION_LABEL not in text:
        issues.append(Issue(rel(APP_TSX), "缺少基础档案左侧导航分组"))
    for view_id, title in REQUIRED_NAV_ITEMS:
        if f'id: "{view_id}"' not in text or f'title: "{title}"' not in text:
            issues.append(Issue(rel(APP_TSX), f"缺少基础档案菜单项: {view_id} / {title}"))
    for marker in REQUIRED_ROUTE_MARKERS:
        if marker not in text:
            issues.append(Issue(rel(APP_TSX), f"缺少基础档案页面渲染入口标记: {marker}"))
    for path in (PAGE_TSX, QUERY_TS):
        if not path.exists():
            issues.append(Issue(rel(path), "缺少基础档案管理端页面或查询层文件"))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues = scan()
    payload = {
        "check": "check_admin_navigation",
        "tier": "T1",
        "category": "前端治理",
        "file": rel(APP_TSX),
        "page": rel(PAGE_TSX),
        "query": rel(QUERY_TS),
        "required_section_label": REQUIRED_SECTION_LABEL,
        "required_nav_items": REQUIRED_NAV_ITEMS,
        "required_route_markers": REQUIRED_ROUTE_MARKERS,
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }

    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_admin_navigation (T1, 前端治理)")
        print(f"  · 检查文件: {payload['file']}")
        if issues:
            print(f"  ✘ {len(issues)} 处管理端导航缺口:")
            for issue in issues:
                print(f"    - {issue.file}: {issue.message}")
        else:
            print("  ✓ 管理端核心模块导航入口已登记")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
