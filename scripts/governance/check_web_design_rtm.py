#!/usr/bin/env python3
"""check_web_design_rtm.py — Web 设计方案 RTM 完整性检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/*-web-design-plan.md
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- 每个 Web 设计方案必须包含 4 类 RTM：字段、动作、状态、证据。
- 每类 RTM 必须有 Markdown 表格，并包含该类最小必需列。
- RTM 表格的“需求来源”必须引用已有用户故事编号。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DOCS_DIR = REPO_ROOT / "docs"
STORY_ID_PATTERN = re.compile(r"\bUS-[A-Z0-9]+-\d+[A-Za-z]?\b")


@dataclass(frozen=True)
class RtmSpec:
    name: str
    required_columns: tuple[str, ...]


@dataclass(frozen=True)
class Issue:
    file: str
    rtm: str
    detail: str


RTM_SPECS = (
    RtmSpec("字段 RTM", ("页面", "字段", "需求来源", "契约")),
    RtmSpec("动作 RTM", ("动作", "需求来源", "前端入口", "API / 契约", "当前结论")),
    RtmSpec("状态 RTM", ("状态流转", "需求来源", "触发动作", "当前结论")),
    RtmSpec("证据 RTM", ("证据对象", "需求来源", "真实截图", "动作验证", "当前结论")),
)


def web_design_plan_files(docs_dir: Path = DOCS_DIR) -> list[Path]:
    return sorted(docs_dir.glob("*-web-design-plan.md"))


def known_story_ids(docs_dir: Path = DOCS_DIR) -> set[str]:
    ids: set[str] = set()
    for path in sorted((docs_dir / "domain").glob("user-stories-*.md")):
        text = path.read_text(encoding="utf-8")
        for line in text.splitlines():
            if line.startswith("#"):
                ids.update(STORY_ID_PATTERN.findall(line))
    return ids


def section_for(text: str, title: str) -> str:
    match = re.search(rf"^##\s+.*{re.escape(title)}\s*$", text, flags=re.MULTILINE)
    if not match:
        return ""
    next_heading = re.search(r"^##\s+", text[match.end():], flags=re.MULTILINE)
    end = match.end() + next_heading.start() if next_heading else len(text)
    return text[match.start():end]


def is_separator_row(line: str) -> bool:
    return bool(re.fullmatch(r"\|[\s:\-|]+\|", line))


def split_table_row(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def parse_markdown_table(section: str) -> tuple[list[str], list[list[str]]]:
    lines = [line.strip() for line in section.splitlines() if line.strip().startswith("|")]
    for index, line in enumerate(lines[1:], start=1):
        if not is_separator_row(line):
            continue
        headers = split_table_row(lines[index - 1])
        rows = [split_table_row(row) for row in lines[index + 1:] if not is_separator_row(row)]
        return headers, rows
    return [], []


def validate_story_sources(
    rel: str,
    rtm: str,
    headers: list[str],
    rows: list[list[str]],
    existing_story_ids: set[str],
) -> list[Issue]:
    issues: list[Issue] = []
    if "需求来源" not in headers:
        return issues
    source_index = headers.index("需求来源")
    for row_number, row in enumerate(rows, start=1):
        source = row[source_index] if source_index < len(row) else ""
        row_story_ids = STORY_ID_PATTERN.findall(source)
        if not row_story_ids:
            issues.append(Issue(rel, rtm, f"第 {row_number} 行需求来源缺少用户故事编号"))
            continue
        unknown = sorted(story_id for story_id in row_story_ids if story_id not in existing_story_ids)
        if unknown:
            issues.append(Issue(rel, rtm, f"第 {row_number} 行未知用户故事编号: {', '.join(unknown)}"))
    return issues


def validate_file(
    path: Path,
    repo_root: Path = REPO_ROOT,
    existing_story_ids: set[str] | None = None,
) -> list[Issue]:
    text = path.read_text(encoding="utf-8")
    rel = str(path.relative_to(repo_root))
    issues: list[Issue] = []
    story_ids = existing_story_ids if existing_story_ids is not None else known_story_ids(repo_root / "docs")
    for spec in RTM_SPECS:
        section = section_for(text, spec.name)
        if not section:
            issues.append(Issue(rel, spec.name, "缺少 RTM 小节"))
            continue
        headers, rows = parse_markdown_table(section)
        if not headers:
            issues.append(Issue(rel, spec.name, "缺少 Markdown 表格"))
            continue
        missing = [column for column in spec.required_columns if column not in headers]
        if missing:
            issues.append(Issue(rel, spec.name, f"缺少列: {', '.join(missing)}"))
        if not rows:
            issues.append(Issue(rel, spec.name, "缺少数据行"))
            continue
        issues.extend(validate_story_sources(rel, spec.name, headers, rows, story_ids))
    return issues


def validate_all(docs_dir: Path = DOCS_DIR, repo_root: Path = REPO_ROOT) -> tuple[list[Path], list[Issue]]:
    files = web_design_plan_files(docs_dir)
    story_ids = known_story_ids(docs_dir)
    issues: list[Issue] = []
    for path in files:
        issues.extend(validate_file(path, repo_root, story_ids))
    return files, issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    files, issues = validate_all()
    payload = {
        "check": "check_web_design_rtm",
        "tier": "T1",
        "category": "文档治理",
        "files": [str(path.relative_to(REPO_ROOT)) for path in files],
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_web_design_rtm (T1, 文档治理) — scanned {len(files)} files")
        if not issues:
            print("  ✓ Web 设计方案均包含字段 / 动作 / 状态 / 证据 RTM")
        else:
            print(f"  ✘ {len(issues)} 处 RTM 缺口:")
            for issue in issues:
                print(f"    {issue.file} [{issue.rtm}] {issue.detail}")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"script error: {exc}", file=sys.stderr)
        sys.exit(2)
