#!/usr/bin/env python3
"""check_mkdocs_nav_consistency.py — MkDocs 导航与关键文档索引一致性检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：mkdocs.yml + AGENTS.md + docs/**/*.md
输出：人类可读 + --json
退出码：
  0  docs/**/*.md 均已纳入 MkDocs nav
  1  文档未纳入 MkDocs nav
  2  脚本自身错误

说明：
  AGENTS 必读文档和 CORE_DOCS 使用更明确的问题类型。
  其他 docs/**/*.md 未入 nav 同样阻断 T1，避免文档导航漂移。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MKDOCS_YML = REPO_ROOT / "mkdocs.yml"
AGENTS_MD = REPO_ROOT / "AGENTS.md"
DOCS_DIR = REPO_ROOT / "docs"

CORE_DOCS = {
    "docs/coding-standards.md",
    "docs/frontend-coding-standards.md",
    "docs/layered-design.md",
    "docs/governance.md",
    "docs/architecture-dependencies.md",
    "docs/adr/README.md",
}

MKDOCS_MD_RE = re.compile(r"(?P<path>[A-Za-z0-9_./-]+\.md)(?:\s*(?:#.*)?)?$")
AGENTS_REQUIRED_SECTION_RE = re.compile(
    r"^\s*##\s+必读文档（按优先级）(?P<body>.*?)(?:^\s*##\s+|\Z)",
    re.DOTALL | re.MULTILINE,
)
AGENTS_DOC_LINK_RE = re.compile(r"\]\((docs/[^)#]+\.md)(?:#[^)]+)?\)")
TOP_LEVEL_KEY_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*:")


@dataclass
class Issue:
    kind: str
    target: str
    detail: str


@dataclass
class WarningItem:
    kind: str
    target: str
    detail: str


def _normalize_doc_path(raw: str) -> str | None:
    path = raw.strip().strip("'\"")
    if not path.endswith(".md"):
        return None
    if path.startswith(("http://", "https://")):
        return None
    path = path.lstrip("./")
    if path.startswith("docs/"):
        return path
    return f"docs/{path}"


def _nav_section_lines(text: str) -> list[str]:
    lines = text.splitlines()
    nav_start: int | None = None
    nav_indent = 0

    for index, line in enumerate(lines):
        if line.strip() == "nav:":
            nav_start = index + 1
            nav_indent = len(line) - len(line.lstrip())
            break

    if nav_start is None:
        return []

    section: list[str] = []
    for line in lines[nav_start:]:
        stripped = line.strip()
        indent = len(line) - len(line.lstrip())
        if stripped and indent <= nav_indent and TOP_LEVEL_KEY_RE.match(stripped):
            break
        section.append(line)
    return section


def parse_mkdocs_nav_paths(text: str) -> set[str]:
    paths: set[str] = set()
    for raw_line in _nav_section_lines(text):
        line = raw_line.split("#", 1)[0].strip()
        if not line or ".md" not in line:
            continue
        candidate = line.rsplit(":", 1)[-1].strip() if ":" in line else line
        match = MKDOCS_MD_RE.search(candidate)
        if not match:
            continue
        normalized = _normalize_doc_path(match.group("path"))
        if normalized:
            paths.add(normalized)
    return paths


def parse_agents_required_docs(text: str) -> set[str]:
    match = AGENTS_REQUIRED_SECTION_RE.search(text)
    if not match:
        return set()
    return set(AGENTS_DOC_LINK_RE.findall(match.group("body")))


def discover_docs(root: Path = REPO_ROOT) -> set[str]:
    docs: set[str] = set()
    for path in (root / "docs").rglob("*.md"):
        rel = path.relative_to(root).as_posix()
        parts = rel.split("/")
        if any(part in {"node_modules", "target", ".git", "dist", "build", "site"} for part in parts):
            continue
        docs.add(rel)
    return docs


def check_consistency(
    *,
    mkdocs_paths: set[str],
    agents_required_docs: set[str],
    all_docs: set[str],
) -> tuple[list[Issue], list[WarningItem]]:
    issues: list[Issue] = []
    warnings: list[WarningItem] = []

    required = CORE_DOCS | agents_required_docs
    for doc in sorted(required):
        if doc not in mkdocs_paths:
            issues.append(
                Issue(
                    "missing_required_nav",
                    doc,
                    "AGENTS 必读或核心治理文档未纳入 mkdocs.yml nav",
                )
            )

    required_missing = {issue.target for issue in issues}
    for doc in sorted(all_docs - mkdocs_paths - required_missing):
        issues.append(
            Issue(
                "doc_not_in_nav",
                doc,
                "docs 文档未纳入 mkdocs.yml nav",
            )
        )

    return issues, warnings


def run() -> tuple[list[Issue], list[WarningItem], dict[str, int]]:
    mkdocs_paths = parse_mkdocs_nav_paths(MKDOCS_YML.read_text(encoding="utf-8"))
    agents_required_docs = parse_agents_required_docs(AGENTS_MD.read_text(encoding="utf-8"))
    all_docs = discover_docs()
    issues, warnings = check_consistency(
        mkdocs_paths=mkdocs_paths,
        agents_required_docs=agents_required_docs,
        all_docs=all_docs,
    )
    stats = {
        "mkdocs_nav_docs": len(mkdocs_paths),
        "agents_required_docs": len(agents_required_docs),
        "all_docs": len(all_docs),
        "warnings": len(warnings),
    }
    return issues, warnings, stats


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues, warnings, stats = run()

    if args.json:
        print(
            json.dumps(
                {
                    "check": "check_mkdocs_nav_consistency",
                    "tier": "T1",
                    "category": "文档治理",
                    "stats": stats,
                    "issues": [asdict(issue) for issue in issues],
                    "warnings": [asdict(warning) for warning in warnings],
                    "ok": not issues,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    else:
        print("check_mkdocs_nav_consistency (T1, 文档治理)")
        print(f"  · mkdocs nav docs: {stats['mkdocs_nav_docs']}")
        print(f"  · AGENTS 必读 docs: {stats['agents_required_docs']}")
        print(f"  · docs/**/*.md: {stats['all_docs']}")
        if issues:
            print(f"  ✘ {len(issues)} 项关键导航漂移:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.target}: {issue.detail}")
        else:
            print("  ✓ AGENTS 必读与核心治理文档均已纳入 mkdocs nav")
        if warnings:
            print(f"  ⚠ {len(warnings)} 个非阻断提示")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
