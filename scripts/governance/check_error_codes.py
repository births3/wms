#!/usr/bin/env python3
"""check_error_codes.py — 错误码字典一致性校验

类别：1. 文档治理
Tier：T1（< 5s）
输入：
  docs/error-codes.md §6 字典 YAML
  docs/compliance/gsp-field-traceability.md §6 字段词典（用于 related_fields 校验）
  docs/domain/user-stories-*.md（用于 related_stories 校验）
  docs/architecture-dependencies.md（用于 module 前缀校验）
输出：人类可读 + --json
退出码：
  0 通过
  1 发现违规
  2 脚本自身错误

校验项：
  1. 每条 11 项必填字段完整
  2. code 全局唯一
  3. code 三段式 <MODULE>_<CATEGORY>_<DETAIL> 全大写下划线
  4. module 前缀在架构清单内（H1-10/H_DOCK/H_AL/M1-11/M_TC/M_VR 等）
  5. severity 在白名单内（info/warning/error/critical）
  6. http_status 在 100-599
  7. related_fields 中的字段在字段词典存在
  8. related_stories 中的故事 ID 在故事文件中存在（如 US-M3-001）
  9. critical 级别错误码必须有 related_fields 或 related_stories（避免无关联）
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DICT_PATH = REPO_ROOT / "docs" / "error-codes.md"
FIELD_DICT_PATH = REPO_ROOT / "docs" / "compliance" / "gsp-field-traceability.md"
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"
ARCH_PATH = REPO_ROOT / "docs" / "architecture-dependencies.md"

VALID_SEVERITY = {"info", "warning", "error", "critical"}
REQUIRED_FIELDS = ["code", "module", "category", "detail", "http_status",
                   "severity", "message_zh", "message_en", "related_fields",
                   "related_stories", "introduced_in"]
CODE_RE = re.compile(r"^[A-Z][A-Z0-9_]+_[A-Z0-9_]+_[A-Z0-9_]+$")


@dataclass
class ErrorEntry:
    code: str = ""
    module: str = ""
    category: str = ""
    detail: str = ""
    http_status: int = 0
    severity: str = ""
    message_zh: str = ""
    message_en: str = ""
    related_fields: list[str] = field(default_factory=list)
    related_stories: list[str] = field(default_factory=list)
    introduced_in: str = ""


@dataclass
class Issue:
    code: str
    severity: str  # error / warning / info
    rule: str
    message: str


def parse_error_codes() -> tuple[list[ErrorEntry], list[str]]:
    """解析 docs/error-codes.md §6 字典 YAML。"""
    errors: list[str] = []
    if not DICT_PATH.exists():
        return [], [f"file not found: {DICT_PATH}"]

    text = DICT_PATH.read_text(encoding="utf-8")
    m = re.search(r"##\s*6\..*?```yaml\s*\n(.*?)\n```", text, re.DOTALL)
    if not m:
        return [], ["未找到 §6 字典 YAML 块"]

    yaml_body = m.group(1)
    entries: list[ErrorEntry] = []
    current: ErrorEntry | None = None

    code_re = re.compile(r"^\s*-\s*code:\s*(.+?)\s*$")
    field_re = re.compile(r"^\s*(\w+):\s*(.+?)\s*$")
    list_re = re.compile(r"^\s*(\w+):\s*\[(.*?)\]\s*$")

    for ln, line in enumerate(yaml_body.splitlines(), start=1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        m_code = code_re.match(line)
        if m_code:
            if current is not None:
                entries.append(current)
            current = ErrorEntry(code=m_code.group(1).strip())
            continue
        if current is None:
            continue
        m_list = list_re.match(line)
        if m_list:
            key = m_list.group(1).strip()
            val_str = m_list.group(2).strip()
            items = [v.strip() for v in val_str.split(",")] if val_str else []
            if hasattr(current, key):
                setattr(current, key, items)
            continue
        m_field = field_re.match(line)
        if m_field:
            key = m_field.group(1).strip()
            val = m_field.group(2).strip().strip("'\"")
            if hasattr(current, key):
                if key == "http_status":
                    try:
                        val = int(val)
                    except ValueError:
                        errors.append(f"code={current.code} http_status 不是整数: {val}")
                        continue
                setattr(current, key, val)

    if current is not None:
        entries.append(current)

    return entries, errors


def check_overview_counts(entries: list[ErrorEntry], text: str) -> list[Issue]:
    issues: list[Issue] = []
    severity_section = re.search(r"## 3\..*?(?=\n---)", text, re.DOTALL)
    module_section = re.search(r"## 4\..*?(?=\n---)", text, re.DOTALL)
    if not severity_section or not module_section:
        return [Issue("<overview>", "error", "overview_missing", "错误码概览章节缺失")]

    declared_severity = {
        match.group(1): int(match.group(2))
        for match in re.finditer(
            r"^\|\s*(info|warning|error|critical)\s*\|\s*(\d+)\s*\|",
            severity_section.group(0),
            re.MULTILINE,
        )
    }
    total_match = re.search(
        r"^\|\s*\*\*合计\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|",
        severity_section.group(0),
        re.MULTILINE,
    )
    expected_severity = Counter(entry.severity for entry in entries)
    for severity in sorted(VALID_SEVERITY):
        if declared_severity.get(severity) != expected_severity.get(severity, 0):
            issues.append(Issue(
                "<overview>",
                "error",
                "severity_count_drift",
                f"严重度 {severity} 概览={declared_severity.get(severity)}，字典={expected_severity.get(severity, 0)}",
            ))
    if not total_match or int(total_match.group(1)) != len(entries):
        declared_total = total_match.group(1) if total_match else None
        issues.append(Issue(
            "<overview>",
            "error",
            "total_count_drift",
            f"错误码合计概览={declared_total}，字典={len(entries)}",
        ))

    declared_modules = {
        match.group(1): int(match.group(2))
        for match in re.finditer(
            r"^\|\s*([A-Z][A-Z0-9_]*)\s*\|\s*(\d+)\s*\|",
            module_section.group(0),
            re.MULTILINE,
        )
    }
    expected_modules = Counter(entry.module for entry in entries)
    for module in sorted(set(declared_modules) | set(expected_modules)):
        if declared_modules.get(module) != expected_modules.get(module, 0):
            issues.append(Issue(
                "<overview>",
                "error",
                "module_count_drift",
                f"模块 {module} 概览={declared_modules.get(module)}，字典={expected_modules.get(module, 0)}",
            ))
    return issues


def load_field_canonicals() -> set[str]:
    """从 gsp-field-traceability.md §6 加载所有 canonical + alias。"""
    if not FIELD_DICT_PATH.exists():
        return set()
    text = FIELD_DICT_PATH.read_text(encoding="utf-8")
    m = re.search(r"##\s*6\..*?```yaml\s*\n(.*?)\n```", text, re.DOTALL)
    if not m:
        return set()
    yaml_body = m.group(1)
    terms: set[str] = set()
    for line in yaml_body.splitlines():
        m1 = re.match(r"^\s*-\s*canonical:\s*(.+?)\s*$", line)
        if m1:
            terms.add(m1.group(1).strip())
            continue
        m2 = re.match(r"^\s*aliases:\s*\[(.+?)\]\s*$", line)
        if m2:
            for a in m2.group(1).split(","):
                terms.add(a.strip())
    return terms


def load_story_ids() -> set[str]:
    """从 user-stories-*.md 加载所有故事 ID。"""
    ids: set[str] = set()
    for f in sorted(DOMAIN_DIR.glob("user-stories-*.md")):
        text = f.read_text(encoding="utf-8")
        for m in re.finditer(r"^##\s+(?:~~)?(US-[A-Z][A-Z0-9-]+\d+\b)(?:~~)?", text, re.M):
            ids.add(m.group(1))
    return ids


def load_module_prefixes() -> set[str]:
    """从 architecture-dependencies.md §1 加载模块前缀。"""
    prefixes = {
        # H 横向能力
        "H1", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10",
        "H_DOCK", "H_AL",
        # H 主动 actor 故事（v15 W4.E）
        "H_DRIVER", "H_STORE",
        # M 业务模块
        "M1", "M2", "M3", "M4", "M5", "M6", "M8", "M9", "M10", "M11",
        # M- 横向业务能力
        "M_TE", "M_RP", "M_PK", "M_VR", "M_QL", "M_CG", "M_SA",
        "M_RC", "M_DI", "M_BA", "M_PM", "M_TC",
    }
    return prefixes


def check_codes(
    entries: list[ErrorEntry],
    field_canonicals: set[str],
    story_ids: set[str],
    module_prefixes: set[str],
) -> list[Issue]:
    issues: list[Issue] = []
    seen_codes: set[str] = set()

    for e in entries:
        # 1. 必填字段
        for f_name in REQUIRED_FIELDS:
            v = getattr(e, f_name)
            if f_name in ("related_fields", "related_stories"):
                continue  # 列表型，可空
            if v in ("", 0, None):
                issues.append(Issue(e.code, "error", "missing_field",
                                    f"缺少必填字段 {f_name}"))

        # 2. code 唯一性
        if e.code in seen_codes:
            issues.append(Issue(e.code, "error", "duplicate_code",
                                f"code 重复: {e.code}"))
        seen_codes.add(e.code)

        # 3. code 三段式
        if not CODE_RE.match(e.code):
            issues.append(Issue(e.code, "error", "code_format",
                                f"code 不符合 <MODULE>_<CATEGORY>_<DETAIL> 三段式: {e.code}"))

        # 4. module 前缀
        if e.module not in module_prefixes:
            issues.append(Issue(e.code, "error", "invalid_module",
                                f"module='{e.module}' 不在架构模块清单内"))

        # 5. severity 白名单
        if e.severity not in VALID_SEVERITY:
            issues.append(Issue(e.code, "error", "invalid_severity",
                                f"severity='{e.severity}' 不在白名单 {sorted(VALID_SEVERITY)}"))

        # 6. http_status 范围
        if not (100 <= e.http_status <= 599):
            issues.append(Issue(e.code, "error", "invalid_http_status",
                                f"http_status={e.http_status} 不在 100-599"))

        # 7. related_fields 在词典中
        for fld in e.related_fields:
            if fld and fld not in field_canonicals:
                issues.append(Issue(e.code, "warning", "field_not_in_dict",
                                    f"related_field '{fld}' 不在字段词典 §6 中"))

        # 8. related_stories 存在
        for sid in e.related_stories:
            if sid and sid not in story_ids:
                issues.append(Issue(e.code, "warning", "story_not_found",
                                    f"related_story '{sid}' 不存在于故事文件中"))

        # 9. critical 必须有关联
        if e.severity == "critical":
            if not e.related_fields and not e.related_stories:
                issues.append(Issue(e.code, "warning", "critical_no_relation",
                                    "critical 错误码无 related_fields/stories 关联"))

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    entries, parse_errors = parse_error_codes()
    if parse_errors:
        if args.json:
            print(json.dumps({"check": "check_error_codes", "ok": False,
                              "parse_errors": parse_errors}, ensure_ascii=False, indent=2))
        else:
            print("check_error_codes (T1, 文档治理) — 字典解析失败")
            for err in parse_errors:
                print(f"  ✘ {err}")
        return 1

    field_canonicals = load_field_canonicals()
    story_ids = load_story_ids()
    module_prefixes = load_module_prefixes()

    issues = check_codes(entries, field_canonicals, story_ids, module_prefixes)
    issues.extend(check_overview_counts(entries, DICT_PATH.read_text(encoding="utf-8")))
    errors = [i for i in issues if i.severity == "error"]
    warnings = [i for i in issues if i.severity == "warning"]

    if args.json:
        print(json.dumps({
            "check": "check_error_codes",
            "tier": "T1",
            "category": "文档治理",
            "total_codes": len(entries),
            "errors": [asdict(i) for i in errors],
            "warnings": [asdict(i) for i in warnings],
            "ok": not errors,
        }, ensure_ascii=False, indent=2))
    else:
        print(f"check_error_codes (T1, 文档治理) — {len(entries)} 错误码")
        if errors:
            print(f"\n  错误（{len(errors)} 项）：")
            for i in errors:
                print(f"    ✘ [{i.code}] {i.rule}: {i.message}")
        if warnings:
            print(f"\n  警告（{len(warnings)} 项）：")
            for i in warnings:
                print(f"    ⚠ [{i.code}] {i.rule}: {i.message}")
        if not (errors or warnings):
            print("  ✓ 所有错误码合规")

    return 0 if not errors else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
