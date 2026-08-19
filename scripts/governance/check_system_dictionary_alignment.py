#!/usr/bin/env python3
"""check_system_dictionary_alignment.py — 系统字典故事与单据类型 RTM 对齐检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：US-M1-011、M2/M4 Web 设计 RTM、项目级 RTM
输出：人类可读 + --json
退出码：
  0  通过
  1  系统字典关键决策或单据类型 RTM 缺失
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
M1_STORY = REPO_ROOT / "docs/domain/user-stories-m1-master-data-warehouse.md"
M2_PLAN = REPO_ROOT / "docs/m2-inbound-web-design-plan.md"
M4_PLAN = REPO_ROOT / "docs/m4-outbound-web-design-plan.md"
PROJECT_RTM = REPO_ROOT / "docs/requirements-traceability-matrix.md"

REQUIRED_M1_TERMS = (
    "US-M1-011：系统字典中心",
    "dict_code",
    "item_code",
    "param_schema JSONB",
    "params JSONB",
    "scope_mode",
    "global_only",
    "owner_extensible",
    "owner_override",
    "override_policy",
    "effective_from",
    "effective_to",
    "M-QL",
    "H2-005",
    "M-PM",
    "document_type",
    "purchase_inbound",
    "sales_return",
    "purchase_return_outbound",
    "sales_outbound",
    "direction",
    "workflow_template",
    "batch_policy",
)

REQUIRED_RTM_TERMS = {
    M2_PLAN: ("US-M1-011", "document_type", "direction = inbound", "batch_policy"),
    M4_PLAN: ("US-M1-011", "document_type", "direction = outbound", "purchase_return_outbound"),
    PROJECT_RTM: ("US-M1-011", "系统字典", "check_system_dictionary_alignment.py"),
}


@dataclass
class Issue:
    file: str
    detail: str


def _rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def _read(path: Path, issues: list[Issue]) -> str:
    if not path.exists():
        issues.append(Issue(_rel(path), "文件不存在"))
        return ""
    return path.read_text(encoding="utf-8")


def validate() -> list[Issue]:
    issues: list[Issue] = []
    story = _read(M1_STORY, issues)
    for term in REQUIRED_M1_TERMS:
        if term not in story:
            issues.append(Issue(_rel(M1_STORY), f"US-M1-011 缺少关键决策: {term}"))

    for path, terms in REQUIRED_RTM_TERMS.items():
        text = _read(path, issues)
        for term in terms:
            if term not in text:
                issues.append(Issue(_rel(path), f"系统字典 RTM 缺少: {term}"))

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues = validate()
    if args.json:
        print(json.dumps({
            "check": "check_system_dictionary_alignment",
            "tier": "T1",
            "category": "文档治理",
            "issues": [asdict(issue) for issue in issues],
            "ok": not issues,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_system_dictionary_alignment (T1, 文档治理)")
        if issues:
            print(f"  ✘ {len(issues)} 处系统字典对齐缺口:")
            for issue in issues:
                print(f"    {issue.file}: {issue.detail}")
        else:
            print("  ✓ US-M1-011 与 M2/M4 单据类型 RTM 对齐")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
