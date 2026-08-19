#!/usr/bin/env python3
"""check_field_coding_standards.py — 字段编码规范静态校验

类别：1. 文档治理
Tier：T1（< 10s）
输入：
  docs/compliance/gsp-field-traceability.md §6 字段词典
  docs/compliance/gsp-field-coding-standards.md
输出：人类可读 + --json
退出码：
  0  字段技术属性符合现有编码规范
  1  字段词典缺技术属性或类型/审计标记违规
  2  脚本自身错误

本脚本只校验已声明规则，不新增字段语义：
- data_type 必须是编码规范允许的 PostgreSQL 类型形态
- encryption / nullable / audit_required / field_class 等技术属性必须完整
- gsp / audit 字段必须 audit_required=true
- ID 类字段不得使用 INT
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
STANDARD_PATH = REPO_ROOT / "docs" / "compliance" / "gsp-field-coding-standards.md"
FIELD_DICT_PATH = REPO_ROOT / "docs" / "compliance" / "gsp-field-traceability.md"

sys.path.insert(0, str(_THIS.parent))
from check_gsp_field_traceability import FieldEntry, parse_field_dictionary  # noqa: E402

VARCHAR_RE = re.compile(r"^VARCHAR\((\d+)\)$")
NUMERIC_RE = re.compile(r"^NUMERIC\((\d+),(\d+)\)$")
ALLOWED_EXACT_TYPES = {
    "BIGINT",
    "BIGSERIAL",
    "BOOLEAN",
    "DATE",
    "INET",
    "INT",
    "JSONB",
    "SMALLINT",
    "TEXT",
    "TEXT[]",
    "TIMESTAMPTZ",
}
VALID_ENCRYPTION = {"none", "masked", "encrypted"}
ID_TOKEN_RE = re.compile(r"(^|_)id$|_id_|ID|主键|外键")


@dataclass
class Issue:
    field: str
    rule: str
    detail: str


def is_valid_data_type(data_type: str) -> bool:
    """校验字段词典中的 PostgreSQL 类型形态。"""
    if data_type in ALLOWED_EXACT_TYPES:
        return True

    varchar = VARCHAR_RE.match(data_type)
    if varchar:
        length = int(varchar.group(1))
        return 1 <= length <= 128

    numeric = NUMERIC_RE.match(data_type)
    if numeric:
        precision = int(numeric.group(1))
        scale = int(numeric.group(2))
        return 1 <= scale < precision <= 18

    return False


def is_id_like_field(entry: FieldEntry) -> bool:
    terms = [entry.canonical, *entry.aliases]
    return any(ID_TOKEN_RE.search(term) for term in terms)


def validate_entries(entries: Iterable[FieldEntry], parse_errors: Iterable[str]) -> list[Issue]:
    issues = [
        Issue(field="<field_dictionary>", rule="parse_error", detail=message)
        for message in parse_errors
    ]

    for entry in entries:
        if not is_valid_data_type(entry.data_type):
            issues.append(Issue(
                field=entry.canonical,
                rule="invalid_data_type",
                detail=f"data_type={entry.data_type!r} 不符合字段编码规范 §3",
            ))
        if entry.data_type == "INT" and is_id_like_field(entry):
            issues.append(Issue(
                field=entry.canonical,
                rule="int_id_type",
                detail="ID 类字段不得使用 INT；应使用 BIGINT / BIGSERIAL",
            ))
        if entry.encryption not in VALID_ENCRYPTION:
            issues.append(Issue(
                field=entry.canonical,
                rule="invalid_encryption",
                detail=f"encryption={entry.encryption!r} 不在 {sorted(VALID_ENCRYPTION)}",
            ))
        if entry.field_class in {"gsp", "audit"} and entry.audit_required is not True:
            issues.append(Issue(
                field=entry.canonical,
                rule="missing_required_audit",
                detail=f"field_class={entry.field_class} 必须 audit_required=true",
            ))
        if entry.nullable is None:
            issues.append(Issue(
                field=entry.canonical,
                rule="missing_nullable",
                detail="缺少 nullable 技术属性",
            ))

    return issues


def run() -> tuple[list[Issue], int]:
    entries, parse_errors = parse_field_dictionary()
    return validate_entries(entries, parse_errors), len(entries)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues, field_count = run()
    if args.json:
        print(json.dumps({
            "check": "check_field_coding_standards",
            "tier": "T1",
            "category": "文档治理",
            "standard": STANDARD_PATH.relative_to(REPO_ROOT).as_posix(),
            "field_dictionary": FIELD_DICT_PATH.relative_to(REPO_ROOT).as_posix(),
            "field_count": field_count,
            "issues": [asdict(issue) for issue in issues],
            "ok": not issues,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_field_coding_standards (T1, 文档治理)")
        print(f"  · standard: {STANDARD_PATH.relative_to(REPO_ROOT).as_posix()}")
        print(f"  · fields: {field_count}")
        if issues:
            print(f"  ✘ {len(issues)} 项字段编码规范违规:")
            for issue in issues:
                print(f"    [{issue.rule}] {issue.field}: {issue.detail}")
        else:
            print("  ✓ 字段技术属性符合现有编码规范")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
