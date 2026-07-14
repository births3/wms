#!/usr/bin/env python3
"""检查延期故事是否登记了最小可验证证据。"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    import tomli as tomllib


REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MATRIX = REPO_ROOT / "governance" / "quality-matrix.toml"
REQUIRED_TEXT_FIELDS = ("id", "title", "reason", "resume_when")
CHECK_FIELDS = ("test_checks", "e2e_checks")
IMPLEMENTATION_EVIDENCE_FIELDS = (
    "frontend_pages",
    "api_paths",
    "database_objects",
    "evidence_refs",
)


@dataclass(frozen=True)
class Issue:
    """一个可直接修复的延期故事证据缺口。"""

    story_id: str
    field: str
    message: str


@dataclass(frozen=True)
class ScanResult:
    story_count: int
    issues: list[Issue]

    @property
    def ok(self) -> bool:
        return not self.issues


def load_matrix() -> dict[str, Any]:
    """读取事实源；解析错误交给入口转换为脚本错误。"""

    value = tomllib.loads(MATRIX.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("quality-matrix.toml 根节点必须是对象")
    return value


def _non_empty_text(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _non_empty_string_list(value: Any) -> bool:
    return isinstance(value, list) and bool(value) and all(_non_empty_text(item) for item in value)


def _story_id(story: dict[str, Any], index: int) -> str:
    value = story.get("id")
    return value.strip() if isinstance(value, str) and value.strip() else f"<第 {index + 1} 条>"


def _path_is_in_repository(reference: str, repo_root: Path) -> bool:
    """只接受仓库内的相对路径，避免绝对路径或 .. 绕出仓库。"""

    path = Path(reference)
    if path.is_absolute():
        return False
    root = repo_root.resolve()
    try:
        (root / path).resolve(strict=False).relative_to(root)
    except ValueError:
        return False
    return (root / path).exists()


def scan_deferred_stories(
    matrix: dict[str, Any], *, repo_root: Path = REPO_ROOT
) -> ScanResult:
    """检查延期故事最小证据登记，不修改传入的矩阵数据。"""

    raw_stories = matrix.get("deferred_stories", [])
    if not isinstance(raw_stories, list):
        raise ValueError("deferred_stories 必须是数组")

    issues: list[Issue] = []
    for index, raw_story in enumerate(raw_stories):
        if not isinstance(raw_story, dict):
            issues.append(Issue(f"<第 {index + 1} 条>", "story", "延期故事必须是对象"))
            continue
        story = raw_story
        story_id = _story_id(story, index)
        for field in REQUIRED_TEXT_FIELDS:
            if not _non_empty_text(story.get(field)):
                issues.append(Issue(story_id, field, f"缺少 {field} 或内容为空"))

        types = story.get("types", [])
        needs_frontend_page = (
            isinstance(types, list) and "frontend_interaction" in types
        )
        if needs_frontend_page and not _non_empty_string_list(story.get("frontend_pages")):
            issues.append(
                Issue(
                    story_id,
                    "frontend_pages",
                    "声明 frontend_interaction 时必须登记非空 frontend_pages",
                )
            )
        elif "frontend_pages" in story and not _non_empty_string_list(story["frontend_pages"]):
            issues.append(
                Issue(story_id, "frontend_pages", "frontend_pages 必须是非空字符串数组")
            )

        valid_check_fields = []
        for field in CHECK_FIELDS:
            if field not in story:
                continue
            if not isinstance(story[field], list) or not all(_non_empty_text(item) for item in story[field]):
                issues.append(Issue(story_id, field, f"{field} 必须是字符串数组，且不能包含空项"))
                continue
            if story[field]:
                valid_check_fields.append(field)
        has_implementation_evidence = any(
            field in story for field in (*IMPLEMENTATION_EVIDENCE_FIELDS, *CHECK_FIELDS)
        )
        if has_implementation_evidence and not valid_check_fields:
            issues.append(Issue(story_id, "test_checks/e2e_checks", "至少填写 test_checks 或 e2e_checks 之一"))

        if "evidence_refs" not in story:
            continue
        references = story["evidence_refs"]
        if not isinstance(references, list) or not all(_non_empty_text(item) for item in references):
            issues.append(Issue(story_id, "evidence_refs", "evidence_refs 必须是字符串数组，且不能包含空项"))
            continue
        for reference in references:
            if not _path_is_in_repository(reference, repo_root):
                issues.append(Issue(story_id, "evidence_refs", f"证据路径不存在或不在仓库内: {reference}"))

    return ScanResult(story_count=len(raw_stories), issues=issues)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="输出机器可读 JSON")
    args = parser.parse_args(argv)

    result = scan_deferred_stories(load_matrix())
    payload = {
        "check": "check_deferred_story_evidence",
        "tier": "T2",
        "category": "流程治理",
        "story_count": result.story_count,
        "issues": [asdict(issue) for issue in result.issues],
        "ok": result.ok,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_deferred_story_evidence (T2, 流程治理)")
        print(f"  · 延期故事：{result.story_count} 条")
        if result.ok:
            print("  ✓ 未发现明确证据缺口")
        else:
            print(f"  ✘ 发现 {len(result.issues)} 个明确证据缺口")
            for issue in result.issues:
                print(f"    - [{issue.story_id}/{issue.field}] {issue.message}")
    return 0 if result.ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
