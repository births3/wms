#!/usr/bin/env python3
"""check_admin_page_query_panel.py — 管理端页面级查询条件检查

类别：6. 前端治理
Tier：T1（< 10s，纯静态扫描）
输入：apps/web-admin/src/App.tsx、apps/web-admin/src/pages/page-query-core-fields.json、QueryPanel 调用页
输出：人类可读 + --json；--suggest 输出缺失菜单页的建议配置
退出码：
  0  页面级查询配置和 QueryPanel 调用一致
  1  配置缺失或核心/更多查询条件未按规则接入
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
APP_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "App.tsx"
CONFIG = REPO_ROOT / "apps" / "web-admin" / "src" / "pages" / "page-query-core-fields.json"
QUERY_PANEL = REPO_ROOT / "packages" / "ui" / "src" / "business" / "QueryPanel" / "QueryPanel.tsx"


@dataclass(frozen=True)
class Issue:
    file: str
    page: str
    message: str


@dataclass(frozen=True)
class MenuPage:
    id: str
    title: str


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def load_config() -> list[dict[str, Any]]:
    payload = json.loads(CONFIG.read_text(encoding="utf-8"))
    pages = payload.get("pages")
    if not isinstance(pages, list):
        raise ValueError("page-query-core-fields.json 缺少 pages 数组")
    return pages


def menu_pages() -> list[MenuPage]:
    text = APP_TSX.read_text(encoding="utf-8")
    start = text.index("const menuSections")
    end = text.index("const WORKSPACE_TABS_STORAGE_KEY", start)
    return [
        MenuPage(page_id, title)
        for page_id, title in re.findall(r'\{\s*id:\s*"([^"]+)"\s*,\s*title:\s*"([^"]+)"', text[start:end])
    ]


def menu_ids() -> set[str]:
    return {page.id for page in menu_pages()}


def suggest_page_config(page_id: str, title: str) -> dict[str, Any]:
    """按当前管理端页面族给新增菜单页生成查询治理建议。"""
    if page_id == "dashboard":
        return {
            "id": page_id,
            "title": title,
            "required": False,
            "reason": "工作台总览页不承担列表查询",
        }
    if page_id == "m1-system-dictionary":
        return {
            "id": page_id,
            "title": title,
            "required": False,
            "reason": "系统字典使用双栏目录内部搜索，不使用页面上方 QueryPanel",
        }
    if page_id == "m1-feature-flags":
        return {
            "id": page_id,
            "title": title,
            "required": False,
            "reason": "配置中心当前不是大列表查询页",
        }
    if page_id.startswith("m1-"):
        return {
            "id": page_id,
            "title": title,
            "required": True,
            "source": "apps/web-admin/src/pages/master-data/M1MasterDataPage.tsx",
            "fieldConstant": "m1QueryFields",
            "coreConstant": "m1CoreQueryFieldKeys",
            "core": ["keyword"],
            "more": [],
        }
    if page_id.startswith("m2-"):
        return {
            "id": page_id,
            "title": title,
            "required": True,
            "source": "apps/web-admin/src/pages/inbound/M2InboundPage.tsx",
            "fieldConstant": "m2InboundQueryFields",
            "coreConstant": "m2InboundCoreQueryFieldKeys",
            "core": ["keyword", "ownerKeyword", "statusFilter"],
            "more": ["documentTypeFilter", "arrivalDate", "createdAt"],
        }
    if page_id == "m3-batches":
        return {
            "id": page_id,
            "title": title,
            "required": True,
            "source": "apps/web-admin/src/pages/inventory/M3BatchManagementPage.tsx",
            "fieldConstant": "m3BatchQueryFields",
            "coreConstant": "m3BatchCoreQueryFieldKeys",
            "core": ["keyword", "qualityStatus"],
            "more": ["recallFlag", "productionDate", "expiryDate", "createdAt"],
        }
    if page_id.startswith("m4-"):
        return {
            "id": page_id,
            "title": title,
            "required": True,
            "source": "apps/web-admin/src/pages/outbound/M4OutboundPage.tsx",
            "fieldConstant": "m4OutboundQueryFields",
            "coreConstant": "m4OutboundCoreQueryFieldKeys",
            "core": ["keyword", "statusFilter"],
            "more": ["businessDate"],
        }
    if page_id.startswith("h9-"):
        return {
            "id": page_id,
            "title": title,
            "required": True,
            "source": "apps/web-admin/src/pages/print-template/H9PrintTemplatePage.tsx",
            "fieldConstant": "h9PrintTemplateQueryFields",
            "coreConstant": "h9PrintTemplateCoreQueryFieldKeys",
            "core": ["keyword", "templateType"],
            "more": [],
        }
    return {
        "id": page_id,
        "title": title,
        "required": False,
        "reason": "新增页面待确认页面级查询分类",
    }


def missing_page_suggestions() -> list[dict[str, Any]]:
    configured_ids = {page.get("id") for page in load_config() if isinstance(page.get("id"), str)}
    return [
        suggest_page_config(page.id, page.title)
        for page in menu_pages()
        if page.id not in configured_ids
    ]


def extract_array_literal(text: str, const_name: str) -> str | None:
    match = re.search(rf"\bconst\s+{re.escape(const_name)}\b", text)
    if not match:
        return None
    equals = text.find("=", match.end())
    if equals < 0:
        return None
    start = text.find("[", equals)
    if start < 0:
        return None
    depth = 0
    for index in range(start, len(text)):
        char = text[index]
        if char == "[":
            depth += 1
        elif char == "]":
            depth -= 1
            if depth == 0:
                return text[start:index + 1]
    return None


def string_values(array_literal: str | None) -> set[str]:
    if not array_literal:
        return set()
    return set(re.findall(r'["\']([^"\']+)["\']', array_literal))


def field_keys(text: str, const_name: str) -> set[str]:
    block = extract_array_literal(text, const_name)
    if not block:
        return set()
    return set(re.findall(r'key:\s*["\']([^"\']+)["\']', block))


def scan() -> list[Issue]:
    issues: list[Issue] = []
    pages = load_config()
    configured_ids = {page.get("id") for page in pages if isinstance(page.get("id"), str)}
    actual_menu_ids = menu_ids()

    for missing in sorted(actual_menu_ids - configured_ids):
        issues.append(Issue(rel(CONFIG), missing, "菜单页缺少页面级查询分类配置"))
    for extra in sorted(configured_ids - actual_menu_ids):
        issues.append(Issue(rel(CONFIG), str(extra), "查询分类配置中的页面 ID 不在 App 菜单中"))

    query_panel_source = QUERY_PANEL.read_text(encoding="utf-8")
    if "defaultVisibleFieldKeys?: string[]" not in query_panel_source:
        issues.append(Issue(rel(QUERY_PANEL), "QueryPanel", "公共 QueryPanel 缺少 defaultVisibleFieldKeys 折叠配置"))
    if "展开" not in query_panel_source or "收起" not in query_panel_source:
        issues.append(Issue(rel(QUERY_PANEL), "QueryPanel", "公共 QueryPanel 缺少展开/收起入口"))

    for page in pages:
        page_id = str(page.get("id", ""))
        required = page.get("required")
        if required is False:
            if not str(page.get("reason", "")).strip():
                issues.append(Issue(rel(CONFIG), page_id, "不需要页面上方查询时必须写 reason"))
            continue
        if required is not True:
            issues.append(Issue(rel(CONFIG), page_id, "required 必须是 true 或 false"))
            continue

        source = page.get("source")
        field_constant = page.get("fieldConstant")
        core_constant = page.get("coreConstant")
        core = page.get("core")
        more = page.get("more", [])
        if not all(isinstance(value, str) and value for value in [source, field_constant, core_constant]):
            issues.append(Issue(rel(CONFIG), page_id, "required=true 时必须配置 source/fieldConstant/coreConstant"))
            continue
        if not isinstance(core, list) or not core or not all(isinstance(item, str) for item in core):
            issues.append(Issue(rel(CONFIG), page_id, "required=true 时 core 必须是非空字符串数组"))
            continue
        if not isinstance(more, list) or not all(isinstance(item, str) for item in more):
            issues.append(Issue(rel(CONFIG), page_id, "more 必须是字符串数组"))
            continue
        overlap = sorted(set(core) & set(more))
        if overlap:
            issues.append(Issue(rel(CONFIG), page_id, f"core 和 more 不能重复: {', '.join(overlap)}"))

        page_path = REPO_ROOT / source
        if not page_path.exists():
            issues.append(Issue(rel(CONFIG), page_id, f"source 不存在: {source}"))
            continue
        text = page_path.read_text(encoding="utf-8")
        fields = field_keys(text, field_constant)
        core_fields = string_values(extract_array_literal(text, core_constant))
        expected = set(core) | set(more)

        missing_fields = sorted(expected - fields)
        if missing_fields:
            issues.append(Issue(rel(page_path), page_id, f"{field_constant} 缺少字段: {', '.join(missing_fields)}"))
        missing_core = sorted(set(core) - core_fields)
        if missing_core:
            issues.append(Issue(rel(page_path), page_id, f"{core_constant} 缺少核心字段: {', '.join(missing_core)}"))
        leaked_more = sorted(set(more) & core_fields)
        if leaked_more:
            issues.append(Issue(rel(page_path), page_id, f"{core_constant} 不能包含更多条件字段: {', '.join(leaked_more)}"))
        if f"fields={{{field_constant}}}" not in text:
            issues.append(Issue(rel(page_path), page_id, f"QueryPanel 未使用 fields={{{field_constant}}}"))
        if f"defaultVisibleFieldKeys={{{core_constant}}}" not in text:
            issues.append(Issue(rel(page_path), page_id, f"QueryPanel 未使用 defaultVisibleFieldKeys={{{core_constant}}}"))

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--suggest", action="store_true", help="输出 App 菜单中缺失页面的查询分类建议")
    args = parser.parse_args(argv)
    if args.suggest:
        suggestions = missing_page_suggestions()
        payload = {
            "check": "check_admin_page_query_panel",
            "tier": "T1",
            "category": "前端治理",
            "suggestions": suggestions,
            "ok": True,
        }
        if args.json:
            print(json.dumps(payload, ensure_ascii=False, indent=2))
        else:
            print("check_admin_page_query_panel suggestions (T1, 前端治理)")
            if suggestions:
                print(f"  发现 {len(suggestions)} 个缺失配置的菜单页，建议补入 page-query-core-fields.json:")
                for suggestion in suggestions:
                    print(json.dumps(suggestion, ensure_ascii=False, indent=2))
            else:
                print("  ✓ 当前 App 菜单页均已纳入页面级查询分类配置")
        return 0

    issues = scan()
    payload = {
        "check": "check_admin_page_query_panel",
        "tier": "T1",
        "category": "前端治理",
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_admin_page_query_panel (T1, 前端治理)")
        if issues:
            print(f"  ✘ {len(issues)} 个页面级查询配置问题:")
            for issue in issues:
                print(f"    - {issue.file} [{issue.page}]: {issue.message}")
        else:
            print("  ✓ 页面级查询核心/更多条件配置已对齐")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
