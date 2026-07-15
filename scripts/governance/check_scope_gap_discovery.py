#!/usr/bin/env python3
"""check_scope_gap_discovery.py — 范围缺口自发现检查

类别：4. 流程治理
Tier：T1（< 10s，纯静态扫描）
输入：docs/domain/user-stories-*.md、governance/quality-matrix.toml、
      governance/menu-e2e-screenshot-policy.toml、apps/web-admin/src/App.tsx
输出：人类可读 + --json；--strict 按模块把发现型缺口升级为失败
退出码：
  0  当前矩阵接线无硬错误；发现型缺口已输出
  1  存在硬错误，或 --strict 下存在发现型缺口
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    import tomli as tomllib

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
STORY_DIR = REPO_ROOT / "docs" / "domain"
MATRIX = REPO_ROOT / "governance" / "quality-matrix.toml"
SCREENSHOT_POLICY = REPO_ROOT / "governance" / "menu-e2e-screenshot-policy.toml"
APP_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "App.tsx"
ADMIN_VIEW_RENDERER_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "app-shell" / "AdminViewRenderer.tsx"
ADMIN_MENU_DEV_MOCK_TS = REPO_ROOT / "apps" / "web-admin" / "dev-mocks" / "admin-menu-dev-mock.ts"

STORY_HEADING_RE = re.compile(r"^##\s+(US-[A-Z0-9]+-\d{3})[：:]\s*(.+?)\s*$", re.MULTILINE)
MENU_ITEM_RE = re.compile(r'\{\s*id:\s*"([^"]+)"\s*,\s*title:\s*"([^"]+)"')
MENU_TREE_ITEM_RE = re.compile(r'menuItem\("([^"]+)"\)')
VIEW_LITERAL_RE = re.compile(r'"(dashboard|[a-z][a-z0-9]+-[a-z0-9-]+)"')
DEV_MOCK_MENU_PAGE_RE = re.compile(r'\["([^"]+)",\s*"[^"]+",\s*"[^"]+"\]')


@dataclass(frozen=True)
class StoryHeading:
    story_id: str
    title: str
    module: str
    file: str


@dataclass(frozen=True)
class ScopeGap:
    severity: str  # "block" | "discover"
    kind: str
    module: str
    story_id: str
    file: str
    message: str


@dataclass(frozen=True)
class AdminNavigation:
    menu_sections: dict[str, str]
    default_menu_tree: set[str]
    routed_views: set[str]
    dev_mock_published_views: set[str] = field(default_factory=set)


@dataclass(frozen=True)
class ScopeScanResult:
    active_modules: list[str]
    deferred_story_ids: list[str]
    matrix_story_ids: list[str]
    story_count: int
    gaps: list[ScopeGap]
    ok: bool
    strict_ok: bool


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def story_module(story_id: str) -> str:
    body = story_id.removeprefix("US-")
    return body.rsplit("-", 1)[0]


def parse_story_headings(story_docs: dict[str, str]) -> list[StoryHeading]:
    headings: list[StoryHeading] = []
    for file, text in story_docs.items():
        for story_id, title in STORY_HEADING_RE.findall(text):
            headings.append(StoryHeading(story_id, title.strip(), story_module(story_id), file))
    return headings


def read_story_docs() -> dict[str, str]:
    return {
        rel(path): path.read_text(encoding="utf-8")
        for path in sorted(STORY_DIR.glob("user-stories-*.md"))
    }


def read_matrix() -> dict[str, Any]:
    return tomllib.loads(MATRIX.read_text(encoding="utf-8"))


def matrix_stories(data: dict[str, Any]) -> list[dict[str, Any]]:
    stories = data.get("stories", [])
    return [story for story in stories if isinstance(story, dict)]


def deferred_stories(data: dict[str, Any]) -> list[dict[str, Any]]:
    stories = data.get("deferred_stories", [])
    return [story for story in stories if isinstance(story, dict)]


def read_admin_navigation() -> AdminNavigation:
    text = APP_TSX.read_text(encoding="utf-8")
    start = text.index("const menuSections")
    end = text.index("const MENU_EXPANDED_STORAGE_KEY", start)
    menu_sections = {page_id: title for page_id, title in MENU_ITEM_RE.findall(text[start:end])}

    tree_start = text.index("const defaultMenuTree")
    tree_end = text.index("const adminMenuIconByKey", tree_start)
    default_menu_tree = set(MENU_TREE_ITEM_RE.findall(text[tree_start:tree_end]))

    route_text = ADMIN_VIEW_RENDERER_TSX.read_text(encoding="utf-8")
    routed_views = set(VIEW_LITERAL_RE.findall(route_text))

    return AdminNavigation(
        menu_sections=menu_sections,
        default_menu_tree=default_menu_tree,
        routed_views=routed_views,
        dev_mock_published_views=read_dev_mock_published_views(),
    )


def read_admin_pages() -> dict[str, str]:
    return read_admin_navigation().menu_sections


def read_dev_mock_published_views() -> set[str]:
    if not ADMIN_MENU_DEV_MOCK_TS.exists():
        return set()
    text = ADMIN_MENU_DEV_MOCK_TS.read_text(encoding="utf-8")
    return {page_id for page_id in DEV_MOCK_MENU_PAGE_RE.findall(text) if page_id == "dashboard" or "-" in page_id}


def page_module(page_id: str) -> str:
    prefix = page_id.split("-", 1)[0].upper()
    return "AL" if prefix == "HAL" else prefix


def has_frontend_e2e_checks(story: dict[str, Any]) -> bool:
    checks = story.get("e2e_checks")
    return isinstance(checks, list) and any(isinstance(item, str) and item.strip() for item in checks)


def has_real_frontend_e2e_check(story: dict[str, Any]) -> bool:
    checks = story.get("e2e_checks")
    return isinstance(checks, list) and any(
        isinstance(item, str) and ("test:e2e:" in item or "playwright test" in item)
        for item in checks
    )


def has_menu_screenshot_evidence(story: dict[str, Any], page_id: str, *, validate_files: bool) -> bool:
    if not has_real_frontend_e2e_check(story):
        return False
    evidence_refs = story.get("evidence_refs")
    if not isinstance(evidence_refs, list):
        return False
    records = story.get("e2e_screenshots")
    if not isinstance(records, list):
        return False
    for record in records:
        if not isinstance(record, dict) or record.get("page") != page_id:
            continue
        spec = record.get("spec")
        screenshot = record.get("screenshot")
        if not isinstance(spec, str) or not spec.startswith("prototypes/e2e/") or not spec.endswith(".spec.ts"):
            continue
        if (
            not isinstance(screenshot, str)
            or not screenshot.startswith("artifacts/screenshot-portal/real-web/")
            or not screenshot.endswith(".png")
        ):
            continue
        if spec not in evidence_refs or screenshot not in evidence_refs:
            continue
        if validate_files:
            spec_path = REPO_ROOT / spec
            if not spec_path.is_file() or Path(screenshot).name not in spec_path.read_text(encoding="utf-8"):
                continue
        return True
    return False


def read_screenshot_legacy_pages() -> set[str]:
    data = tomllib.loads(SCREENSHOT_POLICY.read_text(encoding="utf-8"))
    pages = data.get("policy", {}).get("legacy_pages", [])
    if not isinstance(pages, list) or not all(isinstance(page, str) and page for page in pages):
        raise ValueError("menu-e2e-screenshot-policy.toml 的 policy.legacy_pages 必须是字符串数组")
    return set(pages)


def scan_scope_gaps(
    *,
    story_docs: dict[str, str],
    matrix_stories: list[dict[str, Any]],
    deferred_stories: list[dict[str, Any]] | None = None,
    admin_pages: dict[str, str],
    admin_navigation: AdminNavigation | None = None,
    modules: set[str] | None = None,
    screenshot_legacy_pages: set[str] | None = None,
    validate_screenshot_files: bool = False,
) -> ScopeScanResult:
    story_headings = parse_story_headings(story_docs)
    stories_by_id = {story.story_id: story for story in story_headings}
    matrix_story_ids = {
        story.get("id")
        for story in matrix_stories
        if isinstance(story.get("id"), str)
    }
    all_active_modules = {story.module for story in story_headings}
    requested_modules = {module.upper() for module in modules} if modules is not None else None
    active_modules = requested_modules if requested_modules is not None else all_active_modules

    def should_scan_module(module: str) -> bool:
        if requested_modules is not None:
            return module in active_modules
        return not active_modules or module in active_modules

    gaps: list[ScopeGap] = []
    deferred_story_ids: set[str] = set()
    matrix_frontend_pages = {
        page_id
        for story in [*matrix_stories, *(deferred_stories or [])]
        for page_id in story.get("frontend_pages", [])
        if isinstance(page_id, str)
    }
    if admin_navigation is None:
        admin_navigation = AdminNavigation(
            menu_sections=admin_pages,
            default_menu_tree=set(admin_pages),
            routed_views=set(admin_pages),
        )

    for story in deferred_stories or []:
        story_id = str(story.get("id", ""))
        if not story_id:
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="deferred_story_missing_id",
                    module=str(story.get("module", "")).upper(),
                    story_id="-",
                    file=rel(MATRIX),
                    message="延期故事必须填写 id",
                )
            )
            continue
        module = story_module(story_id) if story_id.startswith("US-") else str(story.get("module", "")).upper()
        if not should_scan_module(module):
            continue
        if story_id in matrix_story_ids:
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="deferred_story_already_in_quality_matrix",
                    module=module,
                    story_id=story_id,
                    file=rel(MATRIX),
                    message="延期故事已进入质量矩阵，不能同时标记为延期",
                )
            )
        if story_id not in stories_by_id:
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="deferred_story_missing_from_story_docs",
                    module=module,
                    story_id=story_id or "-",
                    file=rel(MATRIX),
                    message="延期故事在用户故事文档中没有对应二级标题",
                )
            )
        if not str(story.get("reason", "")).strip():
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="deferred_story_missing_reason",
                    module=module,
                    story_id=story_id or "-",
                    file=rel(MATRIX),
                    message="延期故事必须填写 reason，说明为什么不进入本轮质量矩阵",
                )
            )
        if not str(story.get("owner", "")).strip():
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="deferred_story_missing_owner",
                    module=module,
                    story_id=story_id or "-",
                    file=rel(MATRIX),
                    message="延期故事必须填写 owner，明确负责收口的模块或角色",
                )
            )
        if not str(story.get("resume_when", "")).strip():
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="deferred_story_missing_resume_when",
                    module=module,
                    story_id=story_id or "-",
                    file=rel(MATRIX),
                    message="延期故事必须填写 resume_when，明确恢复实施的可验证条件",
                )
            )
        frontend_pages = [page for page in story.get("frontend_pages", []) if isinstance(page, str)]
        if frontend_pages and not has_frontend_e2e_checks(story):
            gaps.append(
                ScopeGap(
                    severity="discover",
                    kind="deferred_frontend_story_missing_e2e_check",
                    module=module,
                    story_id=story_id,
                    file=rel(MATRIX),
                    message="延期故事已有管理端页面，仍必须登记当前已实现切片的 E2E 检查",
                )
            )
        deferred_story_ids.add(story_id)

    for story in matrix_stories:
        story_id = str(story.get("id", ""))
        module = story_module(story_id) if story_id.startswith("US-") else str(story.get("module", "")).upper()
        if not should_scan_module(module):
            continue
        if story_id and story_id not in stories_by_id:
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="matrix_story_missing_from_story_docs",
                    module=module,
                    story_id=story_id,
                    file=rel(MATRIX),
                    message="质量矩阵登记了故事，但用户故事文档没有对应二级标题",
                )
            )
        frontend_pages = [page_id for page_id in story.get("frontend_pages", []) if isinstance(page_id, str) and page_id]
        for page_id in frontend_pages:
            if not isinstance(page_id, str) or not page_id:
                continue
            if page_id not in admin_navigation.menu_sections:
                gaps.append(
                    ScopeGap(
                        severity="block",
                        kind="frontend_page_not_in_menu",
                        module=module,
                        story_id=story_id,
                        file=rel(MATRIX),
                        message=f"质量矩阵声明前端页 {page_id}，但 App 菜单没有该页面",
                    )
                )
            if page_id not in admin_navigation.default_menu_tree:
                gaps.append(
                    ScopeGap(
                        severity="block",
                        kind="frontend_page_not_in_default_menu_tree",
                        module=module,
                        story_id=story_id,
                        file=rel(APP_TSX),
                        message=f"质量矩阵声明前端页 {page_id}，但默认三层菜单树没有该页面",
                    )
                )
            if page_id not in admin_navigation.routed_views:
                gaps.append(
                    ScopeGap(
                        severity="block",
                        kind="frontend_page_not_routed",
                        module=module,
                        story_id=story_id,
                        file=rel(APP_TSX),
                        message=f"质量矩阵声明前端页 {page_id}，但 renderAdminView 没有可达路由",
                    )
                )
            if admin_navigation.dev_mock_published_views and page_id not in admin_navigation.dev_mock_published_views:
                gaps.append(
                    ScopeGap(
                        severity="block",
                        kind="frontend_page_not_in_dev_mock_published_menu",
                        module=module,
                        story_id=story_id,
                        file=rel(ADMIN_MENU_DEV_MOCK_TS),
                        message=f"质量矩阵声明前端页 {page_id}，但 dev mock 已发布菜单没有该页面",
                    )
                )
        story_types = story.get("types", [])
        if (
            frontend_pages
            and isinstance(story_types, list)
            and "frontend_interaction" in story_types
            and not has_frontend_e2e_checks(story)
        ):
            gaps.append(
                ScopeGap(
                    severity="discover",
                    kind="frontend_story_missing_e2e_check",
                    module=module,
                    story_id=story_id,
                    file=rel(MATRIX),
                    message="前端交互故事已声明页面，但质量矩阵缺少 e2e_checks；需登记 Playwright 或页面级自检命令",
                )
            )

    if screenshot_legacy_pages is not None:
        stories_by_page: dict[str, list[dict[str, Any]]] = {}
        for story in matrix_stories:
            story_id = str(story.get("id", ""))
            module = story_module(story_id) if story_id.startswith("US-") else str(story.get("module", "")).upper()
            if not should_scan_module(module):
                continue
            for page_id in story.get("frontend_pages", []):
                if isinstance(page_id, str) and page_id:
                    stories_by_page.setdefault(page_id, []).append(story)
        for page_id in sorted(admin_navigation.menu_sections):
            module = page_module(page_id)
            if not should_scan_module(module) or page_id in screenshot_legacy_pages:
                continue
            stories = stories_by_page.get(page_id, [])
            if any(
                has_menu_screenshot_evidence(story, page_id, validate_files=validate_screenshot_files)
                for story in stories
            ):
                continue
            story_id = str(stories[0].get("id", "-")) if stories else "-"
            gaps.append(
                ScopeGap(
                    severity="block",
                    kind="menu_page_missing_e2e_screenshot_evidence",
                    module=module,
                    story_id=story_id,
                    file=rel(MATRIX),
                    message=(
                        f"菜单页 {page_id} 缺少真实 Playwright E2E 截图证据；需登记 e2e_checks、"
                        "e2e_screenshots(page/spec/screenshot) 并在 evidence_refs 引用 spec 与 PNG 产物路径"
                    ),
                )
            )

    for story in story_headings:
        if story.module not in active_modules:
            continue
        if story.story_id in matrix_story_ids:
            continue
        if story.story_id in deferred_story_ids:
            continue
        gaps.append(
            ScopeGap(
                severity="discover",
                kind="unregistered_story_in_active_module",
                module=story.module,
                story_id=story.story_id,
                file=story.file,
                message=f"活跃模块 {story.module} 还有未进入质量矩阵的故事：{story.title}",
            )
        )

    for page_id, title in sorted(admin_navigation.menu_sections.items()):
        module = page_module(page_id)
        if module not in active_modules:
            continue
        if page_id in matrix_frontend_pages:
            continue
        gaps.append(
            ScopeGap(
                severity="discover",
                kind="menu_page_not_in_quality_matrix",
                module=module,
                story_id="-",
                file=rel(APP_TSX),
                message=f"活跃模块 {module} 的菜单页 {page_id}（{title}）尚未被任何矩阵条目覆盖",
            )
        )

    block_gaps = [gap for gap in gaps if gap.severity == "block"]
    return ScopeScanResult(
        active_modules=sorted(active_modules),
        deferred_story_ids=sorted(deferred_story_ids),
        matrix_story_ids=sorted(matrix_story_ids),
        story_count=len(story_headings),
        gaps=gaps,
        ok=not block_gaps,
        strict_ok=not gaps,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--strict", action="store_true", help="发现型缺口也返回失败")
    parser.add_argument("--module", action="append", help="只检查指定模块，可重复，例如 --module H9 --module M2")
    args = parser.parse_args(argv)

    matrix = read_matrix()
    admin_navigation = read_admin_navigation()
    result = scan_scope_gaps(
        story_docs=read_story_docs(),
        matrix_stories=matrix_stories(matrix),
        deferred_stories=deferred_stories(matrix),
        admin_pages=admin_navigation.menu_sections,
        admin_navigation=admin_navigation,
        modules={module.upper() for module in args.module} if args.module else None,
        screenshot_legacy_pages=read_screenshot_legacy_pages(),
        validate_screenshot_files=True,
    )
    effective_ok = result.strict_ok if args.strict else result.ok
    payload = {
        "check": "check_scope_gap_discovery",
        "tier": "T1",
        "category": "流程治理",
        "active_modules": result.active_modules,
        "deferred_story_ids": result.deferred_story_ids,
        "matrix_story_count": len(result.matrix_story_ids),
        "story_count": result.story_count,
        "strict": args.strict,
        "gaps": [asdict(gap) for gap in result.gaps],
        "ok": effective_ok,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_scope_gap_discovery (T1, 流程治理)")
        print(f"  · 活跃模块：{', '.join(result.active_modules) or '-'}")
        print(f"  · 矩阵故事 {len(result.matrix_story_ids)} 个，故事文档标题 {result.story_count} 个")
        if not result.gaps:
            print("  ✓ 未发现范围缺口")
        else:
            block_count = sum(1 for gap in result.gaps if gap.severity == "block")
            discover_count = len(result.gaps) - block_count
            print(f"  {'✘' if block_count or args.strict else '⚠'} 硬错误 {block_count} 个，发现型缺口 {discover_count} 个")
            for gap in result.gaps[:80]:
                print(f"    - [{gap.severity}/{gap.kind}] {gap.story_id} {gap.file}: {gap.message}")
            if len(result.gaps) > 80:
                print(f"    ... 还有 {len(result.gaps) - 80} 个缺口，请用 --json 查看完整结果")
        if result.gaps and not args.strict:
            print("  · 默认模式只阻塞硬错误；模块验收请加 --strict --module <模块>")
    return 0 if effective_ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
