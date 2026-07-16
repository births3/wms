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
import re
import sys
from collections import defaultdict
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
APP_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "App.tsx"
ADMIN_VIEW_RENDERER_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "app-shell" / "AdminViewRenderer.tsx"
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
MIGRATIONS_DIR = REPO_ROOT / "backend" / "migrations"
MENU_ID_COLLISION_REPAIR = MIGRATIONS_DIR / "202607150015_admin_menu_id_collision_repair.sql"

LITERAL_MENU_NODE_PATTERN = re.compile(
    r"\(\s*'([0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12})'(?:\s*::uuid)?"
    r"\s*,\s*'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}'(?:\s*::uuid)?"
    r"\s*,\s*[123]\s*,\s*'([^']+)'",
    re.IGNORECASE | re.DOTALL,
)

# 历史冲突不能改写已执行迁移；这里只豁免已由向前迁移改为确定性 ID 的精确集合。
REPAIRED_MENU_ID_COLLISIONS = {
    "00000000-0000-0000-0000-000000130024": (
        {"platform.h4.wechat_settings", "platform.h1.sessions", "platform.mcg.numbering"},
        {"platform.h1.sessions", "platform.mcg.numbering"},
    ),
    "00000000-0000-0000-0000-000000130025": (
        {"platform.h1.api_keys", "master_data.docks"},
        {"master_data.docks"},
    ),
}

REQUIRED_SECTION_LABEL = 'label: "基础档案"'
REQUIRED_ROUTE_MARKERS = (
    "M1MasterDataPage",
    "M3BatchManagementPage",
    "masterDataViewToId(view)",
)
REQUIRED_DASHBOARD_BACK_MARKERS = (
    'onBack={() => setView("dashboard")}',
    'onBack={() => navigateTo("dashboard")}',
)

REQUIRED_NAV_ITEMS = (
    ("m1-products", "M1 商品档案"),
    ("m1-business-partners", "M1 客商档案"),
    ("m1-warehouses", "M1 仓库管理"),
    ("m1-zones", "M1 库区管理"),
    ("m1-locations", "M1 库位管理"),
    ("m1-system-dictionary", "M1 系统字典"),
    ("m3-batches", "M3 批号管理"),
    ("mcg-numbering", "M-CG 单据号规则"),
)


@dataclass
class Issue:
    file: str
    message: str


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def read_source(path: Path) -> str:
    """读取治理约定中的源码文件，缺失时返回空串让门禁给出业务缺口。"""
    return path.read_text(encoding="utf-8") if path.exists() else ""


def scan_menu_id_collisions(migrations_dir: Path, repair_file: Path) -> list[Issue]:
    assignments: dict[str, dict[str, set[Path]]] = defaultdict(lambda: defaultdict(set))
    for migration in sorted(migrations_dir.glob("*.sql")):
        for node_id, code in LITERAL_MENU_NODE_PATTERN.findall(read_source(migration)):
            assignments[node_id.lower()][code].add(migration)

    repair_text = read_source(repair_file)
    issues: list[Issue] = []
    for node_id, code_files in assignments.items():
        codes = set(code_files)
        if len(codes) < 2:
            continue
        expected = REPAIRED_MENU_ID_COLLISIONS.get(node_id)
        if expected and codes == expected[0] and all(
            f"admin_menu_node:{code}" in repair_text for code in expected[1]
        ):
            continue
        locations = sorted({
            rel(path) if path.is_relative_to(REPO_ROOT) else path.as_posix()
            for paths in code_files.values()
            for path in paths
        })
        issues.append(Issue(
            ", ".join(locations),
            f"菜单固定 UUID {node_id} 被多个 code 复用: {', '.join(sorted(codes))}",
        ))
    return issues


def scan() -> list[Issue]:
    if not APP_TSX.exists():
        return [Issue(rel(APP_TSX), "Web Admin 入口文件不存在")]

    text = read_source(APP_TSX)
    route_text = read_source(ADMIN_VIEW_RENDERER_TSX)
    issues: list[Issue] = []
    if REQUIRED_SECTION_LABEL not in text:
        issues.append(Issue(rel(APP_TSX), "缺少基础档案左侧导航分组"))
    for view_id, title in REQUIRED_NAV_ITEMS:
        if f'id: "{view_id}"' not in text or f'title: "{title}"' not in text:
            issues.append(Issue(rel(APP_TSX), f"缺少基础档案菜单项: {view_id} / {title}"))
    for marker in REQUIRED_ROUTE_MARKERS:
        if marker not in route_text:
            issues.append(Issue(rel(ADMIN_VIEW_RENDERER_TSX), f"缺少基础档案页面渲染入口标记: {marker}"))
    if not any(marker in route_text for marker in REQUIRED_DASHBOARD_BACK_MARKERS):
        issues.append(Issue(rel(ADMIN_VIEW_RENDERER_TSX), "缺少基础档案页面回工作台入口标记"))
    for path in (PAGE_TSX, QUERY_TS):
        if not path.exists():
            issues.append(Issue(rel(path), "缺少基础档案管理端页面或查询层文件"))
    issues.extend(scan_menu_id_collisions(MIGRATIONS_DIR, MENU_ID_COLLISION_REPAIR))
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
        "route_file": rel(ADMIN_VIEW_RENDERER_TSX),
        "page": rel(PAGE_TSX),
        "query": rel(QUERY_TS),
        "required_section_label": REQUIRED_SECTION_LABEL,
        "required_nav_items": REQUIRED_NAV_ITEMS,
        "required_route_markers": REQUIRED_ROUTE_MARKERS,
        "required_dashboard_back_markers": REQUIRED_DASHBOARD_BACK_MARKERS,
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
