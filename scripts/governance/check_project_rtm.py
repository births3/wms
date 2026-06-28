#!/usr/bin/env python3
"""check_project_rtm.py — 项目级 RTM 完整性检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/requirements-traceability-matrix.md + docs/domain/user-stories-*.md
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DOC = REPO_ROOT / "docs" / "requirements-traceability-matrix.md"
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"
STORY_ID_PATTERN = re.compile(r"\bUS-[A-Z0-9]+-\d+[A-Za-z]?\b")
VALID_CONCLUSIONS = {"已覆盖", "部分覆盖", "待补证据", "不适用"}
INCOMPLETE_CONCLUSIONS = {"部分覆盖", "待补证据"}
EMPTY_GAP_VALUES = {"", "-", "无", "不适用", "N/A", "n/a"}


@dataclass(frozen=True)
class RtmSpec:
    name: str
    required_columns: tuple[str, ...]
    min_rows: int


@dataclass(frozen=True)
class Issue:
    rtm: str
    detail: str


RTM_SPECS = (
    RtmSpec("故事总 RTM", ("模块/能力", "用户故事源", "故事数量", "当前 RTM"), 1),
    RtmSpec("前端体验 RTM", ("范围", "需求来源", "前端入口", "设计/截图证据", "当前结论", "缺口说明", "补齐路径"), 1),
    RtmSpec(
        "后端实现 RTM",
        ("范围", "需求来源", "API / 契约", "Handler / Service", "Domain / Repository / Migration", "测试 / 证据", "当前结论", "缺口说明", "补齐路径"),
        1,
    ),
    RtmSpec("测试证据 RTM", ("范围", "需求来源", "验证命令", "证据对象", "当前结论", "缺口说明", "补齐路径"), 1),
    RtmSpec("合规风险 RTM", ("范围", "需求来源", "合规/风险来源", "控制措施", "证据对象", "当前结论", "缺口说明", "补齐路径"), 1),
)


def known_story_ids(domain_dir: Path = DOMAIN_DIR) -> set[str]:
    ids: set[str] = set()
    for path in sorted(domain_dir.glob("user-stories-*.md")):
        ids.update(STORY_ID_PATTERN.findall(path.read_text(encoding="utf-8")))
    return ids


def story_files(domain_dir: Path = DOMAIN_DIR) -> set[str]:
    return {path.name for path in domain_dir.glob("user-stories-*.md")}


def section_for(text: str, title: str) -> str:
    match = re.search(rf"^##\s+\d+\.\s+{re.escape(title)}\s*$", text, flags=re.MULTILINE)
    if not match:
        return ""
    next_heading = re.search(r"^##\s+\d+\.", text[match.end():], flags=re.MULTILINE)
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
    spec: RtmSpec,
    headers: list[str],
    rows: list[list[str]],
    existing_story_ids: set[str],
) -> list[Issue]:
    if "需求来源" not in headers:
        return []
    issues: list[Issue] = []
    source_index = headers.index("需求来源")
    for row_number, row in enumerate(rows, start=1):
        source = row[source_index] if source_index < len(row) else ""
        row_story_ids = STORY_ID_PATTERN.findall(source)
        if not row_story_ids:
            issues.append(Issue(spec.name, f"第 {row_number} 行需求来源缺少用户故事编号"))
            continue
        unknown = sorted(story_id for story_id in row_story_ids if story_id not in existing_story_ids)
        if unknown:
            issues.append(Issue(spec.name, f"第 {row_number} 行未知用户故事编号: {', '.join(unknown)}"))
    return issues


def validate_conclusions(spec: RtmSpec, headers: list[str], rows: list[list[str]]) -> list[Issue]:
    if "当前结论" not in headers:
        return []
    issues: list[Issue] = []
    conclusion_index = headers.index("当前结论")
    gap_index = headers.index("缺口说明") if "缺口说明" in headers else -1
    path_index = headers.index("补齐路径") if "补齐路径" in headers else -1
    for row_number, row in enumerate(rows, start=1):
        conclusion = row[conclusion_index] if conclusion_index < len(row) else ""
        if conclusion not in VALID_CONCLUSIONS:
            issues.append(Issue(spec.name, f"第 {row_number} 行当前结论非法: {conclusion}"))
            continue
        if conclusion in INCOMPLETE_CONCLUSIONS:
            gap = row[gap_index].strip() if 0 <= gap_index < len(row) else ""
            path = row[path_index].strip() if 0 <= path_index < len(row) else ""
            if gap in EMPTY_GAP_VALUES:
                issues.append(Issue(spec.name, f"第 {row_number} 行为 {conclusion} 但缺口说明为空"))
            if path in EMPTY_GAP_VALUES:
                issues.append(Issue(spec.name, f"第 {row_number} 行为 {conclusion} 但补齐路径为空"))
    return issues


def validate_doc(
    doc: Path = DOC,
    domain_dir: Path = DOMAIN_DIR,
) -> list[Issue]:
    issues: list[Issue] = []
    if not doc.exists():
        return [Issue("项目级 RTM", f"缺少 {doc.relative_to(REPO_ROOT)}")]

    text = doc.read_text(encoding="utf-8")
    existing_story_ids = known_story_ids(domain_dir)
    for spec in RTM_SPECS:
        section = section_for(text, spec.name)
        if not section:
            issues.append(Issue(spec.name, "缺少 RTM 小节"))
            continue
        headers, rows = parse_markdown_table(section)
        if not headers:
            issues.append(Issue(spec.name, "缺少 Markdown 表格"))
            continue
        missing = [column for column in spec.required_columns if column not in headers]
        if missing:
            issues.append(Issue(spec.name, f"缺少列: {', '.join(missing)}"))
        if len(rows) < spec.min_rows:
            issues.append(Issue(spec.name, f"数据行不足，至少需要 {spec.min_rows} 行"))
            continue
        issues.extend(validate_story_sources(spec, headers, rows, existing_story_ids))
        issues.extend(validate_conclusions(spec, headers, rows))

    all_story_files = story_files(domain_dir)
    missing_files = sorted(all_story_files - {name for name in all_story_files if name in text})
    for name in missing_files:
        issues.append(Issue("故事总 RTM", f"缺少用户故事文件引用: {name}"))

    backend_section = section_for(text, "后端实现 RTM")
    if backend_section and "backend/" not in backend_section:
        issues.append(Issue("后端实现 RTM", "后端矩阵未引用 backend/ 路径"))

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues = validate_doc()
    payload = {
        "check": "check_project_rtm",
        "tier": "T1",
        "category": "文档治理",
        "file": str(DOC.relative_to(REPO_ROOT)),
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_project_rtm (T1, 文档治理)")
        if not issues:
            print("  ✓ 项目级 RTM 覆盖故事、前端、后端、测试、合规风险维度")
        else:
            print(f"  ✘ {len(issues)} 处 RTM 缺口:")
            for issue in issues:
                print(f"    [{issue.rtm}] {issue.detail}")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"script error: {exc}", file=sys.stderr)
        sys.exit(2)
