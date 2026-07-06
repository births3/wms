#!/usr/bin/env python3
"""check_scope_gap_discovery.py — 范围缺口自发现检查

类别：4. 流程治理
Tier：T1（< 10s，纯静态扫描）
输入：docs/domain/user-stories-*.md、governance/quality-matrix.toml、apps/web-admin/src/App.tsx
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
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    import tomli as tomllib

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
STORY_DIR = REPO_ROOT / "docs" / "domain"
MATRIX = REPO_ROOT / "governance" / "quality-matrix.toml"
APP_TSX = REPO_ROOT / "apps" / "web-admin" / "src" / "App.tsx"

STORY_HEADING_RE = re.compile(r"^##\s+(US-[A-Z0-9]+-\d{3})[：:]\s*(.+?)\s*$", re.MULTILINE)
MENU_ITEM_RE = re.compile(r'\{\s*id:\s*"([^"]+)"\s*,\s*title:\s*"([^"]+)"')


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


def read_admin_pages() -> dict[str, str]:
    text = APP_TSX.read_text(encoding="utf-8")
    start = text.index("const menuSections")
    end = text.index("const MENU_EXPANDED_STORAGE_KEY", start)
    return {page_id: title for page_id, title in MENU_ITEM_RE.findall(text[start:end])}


def page_module(page_id: str) -> str:
    return page_id.split("-", 1)[0].upper()


def scan_scope_gaps(
    *,
    story_docs: dict[str, str],
    matrix_stories: list[dict[str, Any]],
    deferred_stories: list[dict[str, Any]] | None = None,
    admin_pages: dict[str, str],
    modules: set[str] | None = None,
) -> ScopeScanResult:
    story_headings = parse_story_headings(story_docs)
    stories_by_id = {story.story_id: story for story in story_headings}
    matrix_story_ids = {
        story.get("id")
        for story in matrix_stories
        if isinstance(story.get("id"), str)
    }
    all_active_modules = {
        str(story.get("module", "")).upper()
        for story in matrix_stories
        if str(story.get("module", "")).strip()
    }
    all_active_modules |= {story_module(story_id) for story_id in matrix_story_ids}
    requested_modules = {module.upper() for module in modules} if modules is not None else None
    active_modules = (
        all_active_modules & requested_modules
        if requested_modules is not None
        else all_active_modules
    )

    def should_scan_module(module: str) -> bool:
        if requested_modules is not None:
            return module in active_modules
        return not active_modules or module in active_modules

    gaps: list[ScopeGap] = []
    deferred_story_ids: set[str] = set()
    matrix_frontend_pages = {
        page_id
        for story in matrix_stories
        for page_id in story.get("frontend_pages", [])
        if isinstance(page_id, str)
    }

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
        for page_id in story.get("frontend_pages", []):
            if not isinstance(page_id, str) or not page_id:
                continue
            if page_id not in admin_pages:
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

    for page_id, title in sorted(admin_pages.items()):
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
    result = scan_scope_gaps(
        story_docs=read_story_docs(),
        matrix_stories=matrix_stories(matrix),
        deferred_stories=deferred_stories(matrix),
        admin_pages=read_admin_pages(),
        modules={module.upper() for module in args.module} if args.module else None,
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
