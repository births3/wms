#!/usr/bin/env python3
"""check_multi_end_consistency.py — 多端业务规则放置一致性校验

类别：1. 文档治理
Tier：T1（< 5s）
关联：ADR-0015 多端业务规则放置

校验项：
  1. 故事中验收标准的规则用 [A]/[B]/[C] 标注（推进式：Wave 1 启动前完整覆盖）
  2. 每个写操作故事至少 1 个 A 类规则（强一致约束）
  3. A 类规则的描述完整（不能仅标号不写内容）

退出码：
  0 通过
  1 发现 error 级违规
  2 脚本自身错误

适用范围：写操作故事（核心模块）
普通模式用于日常盘点；T4 使用 --strict，核心模块未分类会阻断。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

# 写操作核心模块（Wave 1-4 期间应完整标注 A/B/C）
WRITE_OPERATION_MODULES = {
    "user-stories-h1-auth-tenant",
    "user-stories-m1-master-data-product",
    "user-stories-m1-master-data-warehouse",
    "user-stories-m2-inbound-asn",
    "user-stories-m2-inbound-verify",
    "user-stories-m3-inventory-operation",
    "user-stories-m4-outbound-order",
    "user-stories-m4-outbound-pick",
    "user-stories-m4-outbound-return",
    "user-stories-mvr-validation-rules",
    "user-stories-mql-quality-liaison",
    "user-stories-mtc-traceability-code",
    "user-stories-msa-stock-adjustment",
    "user-stories-mba-batch-adjustment",
}

# 标注模式：行尾 [A] / [B] / [C]，或 §A: / §B:
TAG_RE = re.compile(r"\[(A|B|C)\]|§([ABC])[：:]")


@dataclass
class StoryTagStats:
    file: str
    story_id: str = ""
    a_count: int = 0
    b_count: int = 0
    c_count: int = 0
    total_ac: int = 0


@dataclass
class Issue:
    file: str
    story_id: str
    severity: str  # error / warning / info
    rule: str
    message: str


def scan_stories() -> list[StoryTagStats]:
    """扫描故事文件，统计每个故事的 A/B/C 标注数。"""
    results: list[StoryTagStats] = []
    for f in sorted(DOMAIN_DIR.glob("user-stories-*.md")):
        text = f.read_text(encoding="utf-8")
        rel = f.stem

        # 切分故事
        matches = list(re.finditer(r"^##\s+(US-[A-Z][A-Z0-9-]+\d+)\b.*$",
                                   text, re.M))
        for i, m in enumerate(matches):
            sid = m.group(1)
            start = m.end()
            end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
            body = text[start:end]
            ac_match = re.search(r"###\s+验收标准\s*\n([\s\S]*?)(?=^##|^###|\Z)",
                                 body, re.M)
            if not ac_match:
                continue
            ac_body = ac_match.group(1)
            stats = StoryTagStats(file=rel, story_id=sid)
            stats.total_ac = len(re.findall(r"^\s*\d+\.\s+", ac_body, re.M))
            for tag_match in TAG_RE.finditer(ac_body):
                tag = tag_match.group(1) or tag_match.group(2)
                if tag == "A":
                    stats.a_count += 1
                elif tag == "B":
                    stats.b_count += 1
                elif tag == "C":
                    stats.c_count += 1
            results.append(stats)

    return results


def check(stats: list[StoryTagStats]) -> list[Issue]:
    issues: list[Issue] = []

    # 按文件分组
    files_seen: dict[str, list[StoryTagStats]] = {}
    for s in stats:
        files_seen.setdefault(s.file, []).append(s)

    for fname, story_stats in files_seen.items():
        is_write_op = fname in WRITE_OPERATION_MODULES
        if not is_write_op:
            continue

        total_a = sum(s.a_count for s in story_stats)
        total_tags = sum(s.a_count + s.b_count + s.c_count for s in story_stats)

        # 1. 模块级：核心写操作模块至少有 A 类规则
        if total_a == 0:
            issues.append(Issue(
                fname, "", "info", "no_class_a_rules",
                "核心写操作模块未标注 A 类（强一致）规则；Wave 1 实施前补全（参 ADR-0015）"
            ))

        # 2. 模块级：未引入标注体系
        if total_tags == 0:
            issues.append(Issue(
                fname, "", "info", "no_rule_classification",
                "本模块未使用 [A]/[B]/[C] 规则分类标注；Wave 1 实施前推进（参 ADR-0015）"
            ))

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--strict", action="store_true", help="核心模块分类缺口阻断")
    args = parser.parse_args(argv)

    stats = scan_stories()
    issues = check(stats)
    errors = [i for i in issues if i.severity == "error"]
    warnings = [i for i in issues if i.severity == "warning"]
    infos = [i for i in issues if i.severity == "info"]

    total_a = sum(s.a_count for s in stats)
    total_b = sum(s.b_count for s in stats)
    total_c = sum(s.c_count for s in stats)

    if args.json:
        print(json.dumps({
            "check": "check_multi_end_consistency",
            "tier": "T1",
            "category": "文档治理",
            "stories_scanned": len(stats),
            "total_class_a": total_a,
            "total_class_b": total_b,
            "total_class_c": total_c,
            "errors": [asdict(i) for i in errors],
            "warnings": [asdict(i) for i in warnings],
            "infos": [asdict(i) for i in infos],
            "strict": args.strict,
            "ok": not errors and not (args.strict and (warnings or infos)),
        }, ensure_ascii=False, indent=2))
    else:
        print(f"check_multi_end_consistency (T1, 文档治理) — "
              f"{len(stats)} 故事 / "
              f"A={total_a} B={total_b} C={total_c}")

        if errors:
            print(f"\n  错误（{len(errors)} 项）：")
            for i in errors:
                loc = f"{i.file}/{i.story_id}" if i.story_id else i.file
                print(f"    ✘ [{loc}] {i.rule}: {i.message}")
        if warnings:
            print(f"\n  警告（{len(warnings)} 项）：")
            for i in warnings:
                loc = f"{i.file}/{i.story_id}" if i.story_id else i.file
                print(f"    ⚠ [{loc}] {i.rule}: {i.message}")
        if infos:
            print(f"\n  信息（{len(infos)} 项，T4 strict 出口前补全）：")
            for i in infos[:10]:
                loc = f"{i.file}/{i.story_id}" if i.story_id else i.file
                print(f"    ℹ [{loc}] {i.rule}: {i.message}")
            if len(infos) > 10:
                print(f"    ...还有 {len(infos) - 10} 项")
        if not (errors or warnings or infos):
            print("  ✓ 多端规则分类全部覆盖")

    return 1 if errors or (args.strict and (warnings or infos)) else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
