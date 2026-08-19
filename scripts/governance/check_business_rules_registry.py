#!/usr/bin/env python3
"""check_business_rules_registry.py — 业务规则注册表结构校验

类别：1. 文档治理
Tier：T1（< 10s）
输入：
  docs/compliance/gsp-business-rules-registry.md
  docs/domain/user-stories-*.md（仅校验注册表引用的文档存在）
输出：人类可读 + --json
退出码：
  0  注册表结构完整
  1  规则索引、详情段或引用文档缺失
  2  脚本自身错误

本脚本不判断业务规则是否正确，只确保每条 BR 具备索引、详情、字段表和文档引用，
让人工评审聚焦在规则语义本身。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
REGISTRY_PATH = REPO_ROOT / "docs" / "compliance" / "gsp-business-rules-registry.md"

sys.path.insert(0, str(_THIS.parent))
from check_field_coding_standards import is_valid_data_type  # noqa: E402

RULE_ROW_RE = re.compile(
    r"^\|\s*(BR-\d+)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|"
)
SECTION_RE = re.compile(r"^##\s+\d+\.\s+(BR-\d+):\s*(.+?)\s*$", re.M)
FIELD_ROW_RE = re.compile(r"^\|\s*`?([^`|]+?)`?\s*\|\s*([A-Z]+(?:\([0-9,]+\))?(?:\[\])?)\s*\|", re.M)
DOC_REF_RE = re.compile(r"docs/(domain/user-stories-[a-z0-9-]+\.md|compliance/gsp-[a-z0-9-]+\.md)")


@dataclass
class BusinessRule:
    rule_id: str
    name: str
    index_fields: str
    trigger: str
    gsp_refs: str
    detail_title: str = ""
    detail_fields: list[str] = field(default_factory=list)


@dataclass
class Issue:
    rule_id: str
    rule: str
    detail: str


def parse_business_rules(text: str) -> tuple[list[BusinessRule], list[Issue]]:
    rules: dict[str, BusinessRule] = {}
    issues: list[Issue] = []

    for line in text.splitlines():
        match = RULE_ROW_RE.match(line)
        if not match:
            continue
        rule_id, name, fields_text, trigger, gsp_refs = [part.strip() for part in match.groups()]
        if rule_id in rules:
            issues.append(Issue(rule_id, "duplicate_index", "规则索引中 ID 重复"))
            continue
        rules[rule_id] = BusinessRule(
            rule_id=rule_id,
            name=name,
            index_fields=fields_text,
            trigger=trigger,
            gsp_refs=gsp_refs,
        )

    sections = list(SECTION_RE.finditer(text))
    for index, section in enumerate(sections):
        rule_id = section.group(1)
        title = section.group(2).strip()
        start = section.end()
        end = sections[index + 1].start() if index + 1 < len(sections) else len(text)
        body = text[start:end]

        rule = rules.get(rule_id)
        if rule is None:
            issues.append(Issue(rule_id, "detail_without_index", "存在详情段但索引表未登记"))
            continue

        rule.detail_title = title
        field_rows = [
            (field_name.strip(), data_type.strip())
            for field_name, data_type in FIELD_ROW_RE.findall(body)
            if field_name.strip() not in {"字段", "----"}
        ]
        rule.detail_fields = [field_name for field_name, _ in field_rows]
        for field_name, data_type in field_rows:
            if not is_valid_data_type(data_type):
                issues.append(Issue(
                    rule_id,
                    "invalid_field_type",
                    f"字段 {field_name} 的 data_type={data_type!r} 不符合字段编码规范",
                ))

        has_description = "规则描述" in body or (rule_id == "BR-8" and "状态枚举" in body)
        if not has_description:
            issues.append(Issue(rule_id, "missing_description", "详情段缺少“规则描述”小节"))
        if "涉及字段" not in body:
            issues.append(Issue(rule_id, "missing_fields_section", "详情段缺少“涉及字段”小节"))
        if not rule.detail_fields:
            issues.append(Issue(rule_id, "missing_detail_fields", "详情段未登记字段表"))

    for rule in rules.values():
        if not rule.index_fields or rule.index_fields == "—":
            issues.append(Issue(rule.rule_id, "missing_index_fields", "索引表缺少涉及字段"))
        if not rule.trigger or rule.trigger == "—":
            issues.append(Issue(rule.rule_id, "missing_trigger", "索引表缺少触发场景"))
        if not rule.gsp_refs or rule.gsp_refs == "—":
            issues.append(Issue(rule.rule_id, "missing_gsp_refs", "索引表缺少 GSP 关联"))
        if not rule.detail_title:
            issues.append(Issue(rule.rule_id, "missing_detail_section", "缺少规则详情段"))

    return sorted(rules.values(), key=lambda item: int(item.rule_id.split("-")[1])), issues


def referenced_docs(text: str) -> list[str]:
    return sorted({f"docs/{match.group(1)}" for match in DOC_REF_RE.finditer(text)})


def check_referenced_docs(paths: list[str]) -> list[Issue]:
    issues: list[Issue] = []
    for path in paths:
        if not (REPO_ROOT / path).exists():
            issues.append(Issue("<registry>", "missing_referenced_doc", f"引用文档不存在: {path}"))
    return issues


def run(path: Path = REGISTRY_PATH) -> tuple[list[BusinessRule], list[str], list[Issue]]:
    text = path.read_text(encoding="utf-8")
    rules, issues = parse_business_rules(text)
    refs = referenced_docs(text)
    issues.extend(check_referenced_docs(refs))
    return rules, refs, issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    rules, refs, issues = run()
    if args.json:
        print(json.dumps({
            "check": "check_business_rules_registry",
            "tier": "T1",
            "category": "文档治理",
            "registry": REGISTRY_PATH.relative_to(REPO_ROOT).as_posix(),
            "rule_count": len(rules),
            "referenced_docs": refs,
            "issues": [asdict(issue) for issue in issues],
            "ok": not issues,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_business_rules_registry (T1, 文档治理)")
        print(f"  · registry: {REGISTRY_PATH.relative_to(REPO_ROOT).as_posix()}")
        print(f"  · rules: {len(rules)}")
        print(f"  · referenced docs: {len(refs)}")
        if issues:
            print(f"  ✘ {len(issues)} 项业务规则注册表违规:")
            for issue in issues:
                print(f"    [{issue.rule}] {issue.rule_id}: {issue.detail}")
        else:
            print("  ✓ 业务规则索引、详情字段表与文档引用完整")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
