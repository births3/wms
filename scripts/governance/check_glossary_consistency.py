#!/usr/bin/env python3
"""check_glossary_consistency.py — 术语一致性校验（半自动）

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/glossary.md（术语表）+ docs/domain/*.md + docs/*.md
输出：人类可读 + --json
退出码：
  0  通过（无禁用词出现）
  1  发现禁用同义词
  2  脚本自身错误

原理：
- 从 docs/glossary.md 解析"禁用同义词"列
- 扫描所有 .md 文件，检查是否出现禁用词
- 排除 glossary.md 本身（定义处允许出现）
- 排除引号/括号内的引用（如"签收（单独使用时）"）

局限（需人工补充）：
- 只能检查已登记的禁用词，新的混用需人工发现后加入术语表
- 不做语义分析（如"签收"在"收货签收"语境下可能合理）
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
GLOSSARY_PATH = REPO_ROOT / "docs" / "glossary.md"


@dataclass
class Violation:
    file: str
    line: int
    term: str
    context: str


def parse_banned_terms() -> dict[str, str]:
    """从 glossary.md 解析禁用同义词 → 正确术语的映射。"""
    if not GLOSSARY_PATH.exists():
        return {}

    text = GLOSSARY_PATH.read_text(encoding="utf-8")
    banned: dict[str, str] = {}

    # 匹配表格行：| # | 术语 | 英文 | 定义 | 禁用同义词 |
    # 禁用同义词在最后一列，可能含多个词用顿号/逗号分隔
    row_re = re.compile(
        r"^\|\s*\d+\s*\|\s*(.+?)\s*\|\s*\S+\s*\|\s*.+?\s*\|\s*(.+?)\s*\|$",
        re.MULTILINE,
    )
    for m in row_re.finditer(text):
        correct_term = m.group(1).strip()
        banned_cell = m.group(2).strip()
        if banned_cell == "—" or not banned_cell:
            continue
        # 拆分：顿号、逗号、分号
        for raw in re.split(r"[、，,；;]", banned_cell):
            word = raw.strip()
            # 去掉括号注释：如"禁用（太具体）" → "禁用"
            word = re.sub(r"[（(].*?[）)]", "", word).strip()
            if word and len(word) >= 2:
                # 禁用词不能等于正确术语本身（防止自引用）
                if word != correct_term:
                    banned[word] = correct_term

    return banned


def scan_file(path: Path, banned: dict[str, str]) -> list[Violation]:
    """扫描单个文件，返回违规列表。"""
    violations: list[Violation] = []
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return []

    rel = path.relative_to(REPO_ROOT).as_posix()

    for lineno, line in enumerate(text.splitlines(), start=1):
        # 跳过表格定义行（glossary 本身）
        if path == GLOSSARY_PATH:
            continue
        # 跳过代码块
        if line.strip().startswith("```"):
            continue
        for term in banned:
            if term in line:
                # 排除：禁用词是更长词的子串（如"储位"在"存储位"中）
                idx = line.find(term)
                while idx != -1:
                    # 检查前后字符是否为中文（如果是，说明是更长词的一部分）
                    before = line[idx - 1] if idx > 0 else ""
                    after = line[idx + len(term)] if idx + len(term) < len(line) else ""
                    is_part_of_longer = (
                        ("\u4e00" <= before <= "\u9fff") or
                        ("\u4e00" <= after <= "\u9fff")
                    )
                    if not is_part_of_longer:
                        context = line.strip()[:100]
                        violations.append(Violation(
                            file=rel, line=lineno, term=term, context=context
                        ))
                        break
                    idx = line.find(term, idx + 1)

    return violations


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    banned = parse_banned_terms()
    if not banned:
        print("check_glossary_consistency: no banned terms found (glossary empty or missing)")
        return 0

    # 扫描范围：所有 .md（排除 glossary 本身和 node_modules 等）
    md_files: list[Path] = []
    for p in REPO_ROOT.rglob("*.md"):
        rel = p.relative_to(REPO_ROOT).as_posix()
        if any(skip in rel for skip in ("node_modules/", "target/", ".git/")):
            continue
        if p == GLOSSARY_PATH:
            continue
        md_files.append(p)

    all_violations: list[Violation] = []
    for f in sorted(md_files):
        all_violations.extend(scan_file(f, banned))

    if args.json:
        payload = {
            "check": "check_glossary_consistency",
            "tier": "T1",
            "category": "文档治理",
            "banned_terms_count": len(banned),
            "files_scanned": len(md_files),
            "violations": [asdict(v) for v in all_violations],
            "ok": not all_violations,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_glossary_consistency (T1, 文档治理) — {len(banned)} banned terms, {len(md_files)} files")
        if not all_violations:
            print("  ✓ no banned synonyms found")
        else:
            print(f"  ✘ {len(all_violations)} violation(s):")
            # 按文件分组
            by_file: dict[str, list[Violation]] = {}
            for v in all_violations:
                by_file.setdefault(v.file, []).append(v)
            for f, vs in sorted(by_file.items()):
                print(f"    {f}:")
                for v in vs[:10]:  # 每文件最多显示 10 条
                    print(f"      L{v.line}: '{v.term}' → 应使用 '{banned[v.term]}'")
                if len(vs) > 10:
                    print(f"      ... and {len(vs) - 10} more")

    return 0 if not all_violations else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
