#!/usr/bin/env python3
"""check_gsp_field_traceability.py — GSP 字段追溯矩阵一致性校验

类别：1. 文档治理
Tier：T1（< 10s）
输入：
  docs/compliance/gsp-field-traceability.md    （字段词典 §6）
  docs/domain/user-stories-*.md                   （故事字段表）
输出：人类可读 + --json
退出码：
  0  通过（每个 gsp/audit 强制字段至少 1 个 alias 在故事字段表或正文中出现）
  1  发现违规：GSP 字段无任何 alias 实现 / 字段词典格式错误
  2  脚本自身错误

原理：
- 从 gsp-field-traceability.md §6 解析字段词典（YAML 块，正则解析）
- 扫描 docs/domain/user-stories-*.md 的字段表行（"| 字段名 | (必填|条件必填|可选|系统带出) |"）
- 对每个 canonical 字段：
  - gsp/audit：查找 aliases 中至少 1 个出现在故事字段表或正文中 → ✅
  - gsp/audit 全部未匹配 → ❌ 报错
  - business/config/interface 未匹配 → ℹ 信息（辅助字段由模块/防腐层归口，不污染 T1 warning）
- 同一 canonical 的多个 alias 在故事中混用 → 信息（不报错，仅提示）

局限（需人工补充）：
- 字段词典与故事字段表的精确匹配；模糊匹配未实现（如"批号 / 数量"组合字段）
- 只检查存在性，不检查字段类型 / 必填性 / 业务语义
- 字段词典需手工维护（gsp-field-traceability.md §6）
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
MATRIX_PATH = REPO_ROOT / "docs" / "compliance" / "gsp-field-traceability.md"
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

# 故事字段表行模式："| 字段名 | (必填|条件必填|可选|系统带出) |"
FIELD_TABLE_ROW_RE = re.compile(
    r"^\|\s*([^|]+?)\s*\|\s*(必填|条件必填|可选|系统带出)\b"
)


@dataclass
class FieldEntry:
    canonical: str
    aliases: list[str] = field(default_factory=list)
    gsp_clauses: list[str] = field(default_factory=list)
    wms_status: str = "implemented"  # implemented | unimplemented | not_applicable | acceptable_alias
    # acceptable_alias: 多 alias 是合理的（细化语义 / 中英文双语），不报混用 info
    # ── v2 技术属性（2026-05-17 增）──
    data_type: str = ""           # PostgreSQL 类型，如 VARCHAR(20)
    validation: str = ""          # 校验规则
    nullable: bool | None = None  # None=未声明（视为缺失）
    encryption: str = ""          # none | masked | encrypted
    audit_required: bool | None = None
    example: str = ""
    # ── v3 性质类别 + 业务领域（2026-05-17 增）──
    field_class: str = ""         # gsp | audit | business | system | config | derived | interface
    category: str = ""            # 12 业务领域


@dataclass
class Issue:
    canonical: str
    severity: str  # "error" | "warning" | "info"
    message: str
    aliases: list[str] = field(default_factory=list)
    matched_files: list[str] = field(default_factory=list)


def parse_field_dictionary() -> tuple[list[FieldEntry], list[str]]:
    """解析 gsp-field-traceability.md §6 字段词典 YAML 块。

    返回：(entries, parse_errors)
    """
    if not MATRIX_PATH.exists():
        return [], [f"matrix file not found: {MATRIX_PATH}"]

    text = MATRIX_PATH.read_text(encoding="utf-8")

    # 找到 §6 字段词典的 ```yaml ... ``` 块
    yaml_block_re = re.compile(
        r"##\s*6\.\s*字段词典.*?```yaml\s*\n(.*?)\n```",
        re.DOTALL,
    )
    m = yaml_block_re.search(text)
    if not m:
        return [], [
            "未找到 §6 字段词典 YAML 块（应为 ```yaml ... ``` 包围）"
        ]

    yaml_body = m.group(1)
    entries: list[FieldEntry] = []
    errors: list[str] = []
    current: FieldEntry | None = None

    canonical_re = re.compile(r"^\s*-\s*canonical:\s*(.+?)\s*$")
    aliases_re = re.compile(r"^\s*aliases:\s*\[(.+?)\]\s*$")
    gsp_re = re.compile(r"^\s*gsp_clauses:\s*\[(.+?)\]\s*$")
    status_re = re.compile(r"^\s*wms_status:\s*(\w+)")  # 行尾允许注释
    # v2 技术属性
    data_type_re = re.compile(r"^\s*data_type:\s*(.+?)\s*$")
    validation_re = re.compile(r"^\s*validation:\s*'(.*?)'\s*$")
    nullable_re = re.compile(r"^\s*nullable:\s*(true|false)\s*$")
    encryption_re = re.compile(r"^\s*encryption:\s*(\w+)")
    audit_re = re.compile(r"^\s*audit_required:\s*(true|false)\s*$")
    example_re = re.compile(r"^\s*example:\s*'(.*?)'\s*$")
    # v3 性质类别
    field_class_re = re.compile(r"^\s*field_class:\s*(\w+)\s*$")
    category_re = re.compile(r"^\s*category:\s*(.+?)\s*$")

    for ln, line in enumerate(yaml_body.splitlines(), start=1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        m_can = canonical_re.match(line)
        if m_can:
            if current is not None:
                entries.append(current)
            current = FieldEntry(canonical=m_can.group(1).strip())
            continue
        m_ali = aliases_re.match(line)
        if m_ali and current is not None:
            current.aliases = [
                s.strip() for s in m_ali.group(1).split(",") if s.strip()
            ]
            continue
        m_gsp = gsp_re.match(line)
        if m_gsp and current is not None:
            current.gsp_clauses = [
                s.strip() for s in m_gsp.group(1).split(",") if s.strip()
            ]
            continue
        m_status = status_re.match(line)
        if m_status and current is not None:
            current.wms_status = m_status.group(1).strip()
            continue
        m_dt = data_type_re.match(line)
        if m_dt and current is not None:
            current.data_type = m_dt.group(1).strip()
            continue
        m_val = validation_re.match(line)
        if m_val and current is not None:
            current.validation = m_val.group(1)
            continue
        m_nul = nullable_re.match(line)
        if m_nul and current is not None:
            current.nullable = m_nul.group(1) == "true"
            continue
        m_enc = encryption_re.match(line)
        if m_enc and current is not None:
            current.encryption = m_enc.group(1).strip()
            continue
        m_audit = audit_re.match(line)
        if m_audit and current is not None:
            current.audit_required = m_audit.group(1) == "true"
            continue
        m_ex = example_re.match(line)
        if m_ex and current is not None:
            current.example = m_ex.group(1)
            continue
        m_fc = field_class_re.match(line)
        if m_fc and current is not None:
            current.field_class = m_fc.group(1).strip()
            continue
        m_cat = category_re.match(line)
        if m_cat and current is not None:
            current.category = m_cat.group(1).strip()
            continue

    if current is not None:
        entries.append(current)

    # 校验完整性：每个 entry 必须有 aliases + 合法 wms_status + 5 项技术属性
    # v3：gsp/audit 必须有 gsp_clauses；其他 field_class 不强制
    valid_statuses = {"implemented", "unimplemented", "not_applicable", "acceptable_alias"}
    valid_encryption = {"none", "masked", "encrypted"}
    valid_field_classes = {"gsp", "audit", "business", "system", "config", "derived", "interface"}
    for e in entries:
        if not e.aliases:
            errors.append(f"字段 '{e.canonical}' 缺少 aliases")
        # v3：仅 gsp/audit 类必须有 gsp_clauses
        if e.field_class in ("gsp", "audit") and not e.gsp_clauses:
            errors.append(f"字段 '{e.canonical}' (field_class={e.field_class}) 缺少 gsp_clauses")
        if e.wms_status not in valid_statuses:
            errors.append(
                f"字段 '{e.canonical}' wms_status='{e.wms_status}' 不在白名单 {sorted(valid_statuses)}"
            )
        # v2 技术属性校验
        if not e.data_type:
            errors.append(f"字段 '{e.canonical}' 缺少 data_type（v2 技术属性）")
        if not e.validation:
            errors.append(f"字段 '{e.canonical}' 缺少 validation（v2 技术属性）")
        if e.nullable is None:
            errors.append(f"字段 '{e.canonical}' 缺少 nullable（v2 技术属性）")
        if e.encryption not in valid_encryption:
            errors.append(
                f"字段 '{e.canonical}' encryption='{e.encryption}' 不在白名单 {sorted(valid_encryption)}"
            )
        if e.audit_required is None:
            errors.append(f"字段 '{e.canonical}' 缺少 audit_required（v2 技术属性）")
        if not e.example:
            errors.append(f"字段 '{e.canonical}' 缺少 example（v2 技术属性）")
        # v3 性质类别校验
        if e.field_class not in valid_field_classes:
            errors.append(
                f"字段 '{e.canonical}' field_class='{e.field_class}' 不在白名单 {sorted(valid_field_classes)}"
            )
        if not e.category:
            errors.append(f"字段 '{e.canonical}' 缺少 category（v3 业务领域）")

    return entries, errors


def scan_story_field_tables() -> dict[str, list[str]]:
    """扫描 docs/domain/user-stories-*.md 中的字段表，返回 {字段名: [文件列表]}。

    字段名取字段表行第 1 列（"|  字段名  |  必填  |..."）。
    """
    field_to_files: dict[str, set[str]] = {}
    for f in sorted(DOMAIN_DIR.glob("user-stories-*.md")):
        text = f.read_text(encoding="utf-8")
        for line in text.splitlines():
            m = FIELD_TABLE_ROW_RE.match(line)
            if not m:
                continue
            field_name = m.group(1).strip()
            # 跳过表头分隔行
            if set(field_name) <= {"-", " ", ":"}:
                continue
            # 去掉 markdown 强调符号
            field_name = field_name.replace("**", "").strip()
            field_to_files.setdefault(field_name, set()).add(
                f.relative_to(REPO_ROOT).as_posix()
            )
    return {k: sorted(v) for k, v in field_to_files.items()}


def scan_story_full_text() -> dict[str, str]:
    """读取每个故事文件的全文，返回 {file_path: text}。

    用于"弱匹配"：字段在故事文档中作为正文 / 验收标准 / 枚举 / 配置项出现。
    """
    return {
        f.relative_to(REPO_ROOT).as_posix(): f.read_text(encoding="utf-8")
        for f in sorted(DOMAIN_DIR.glob("user-stories-*.md"))
    }


def check_field_coverage(
    entries: list[FieldEntry],
    field_to_files: dict[str, list[str]],
    full_text: dict[str, str],
) -> list[Issue]:
    """对每个字段词典条目，检查 aliases 中至少 1 个在故事中实现。

    分两级匹配：
    - 强匹配：alias 出现在 PDA 字段表行（结构化字段）
    - 弱匹配：alias 出现在故事正文（验收标准 / 枚举 / 配置项）

    only when 弱匹配也失败时才报 error。
    """
    issues: list[Issue] = []

    for e in entries:
        strong_matched: list[str] = []   # 出现在字段表行的 alias
        weak_matched: list[str] = []     # 仅出现在正文的 alias
        matched_files: list[str] = []

        for alias in e.aliases:
            # 强匹配：PDA 字段表行
            for fname, files in field_to_files.items():
                if alias == fname or (
                    len(alias) >= 2 and alias in fname
                ):
                    if alias not in strong_matched:
                        strong_matched.append(alias)
                    for fp in files:
                        if fp not in matched_files:
                            matched_files.append(fp)
                    break

            # 弱匹配：全文搜索（仅在强匹配失败时记录）
            if alias in strong_matched:
                continue
            for fp, text in full_text.items():
                if alias in text:
                    if alias not in weak_matched:
                        weak_matched.append(alias)
                    if fp not in matched_files:
                        matched_files.append(fp)
                    break

        all_matched = strong_matched + weak_matched

        # unimplemented 字段：所有 alias 不在故事中是预期的，仅记 info
        if e.wms_status == "unimplemented":
            issues.append(
                Issue(
                    canonical=e.canonical,
                    severity="info",
                    message=(
                        f"GSP 字段 '{e.canonical}' 标记为 unimplemented（v25 backlog）；"
                        f"aliases {e.aliases} 在故事中"
                        f"{'均未找到' if not all_matched else f'部分出现于 {all_matched}'}"
                    ),
                    aliases=e.aliases,
                    matched_files=matched_files,
                )
            )
            continue

        # not_applicable 字段（外部系统/ERP 主管）：仅记 info
        if e.wms_status == "not_applicable":
            issues.append(
                Issue(
                    canonical=e.canonical,
                    severity="info",
                    message=(
                        f"GSP 字段 '{e.canonical}' 标记为 not_applicable（外部系统/ERP 主管）"
                    ),
                    aliases=e.aliases,
                )
            )
            continue

        # v3：system / derived 字段不强制故事提及（系统层自动管理）
        if e.field_class in ("system", "derived"):
            if all_matched:
                issues.append(
                    Issue(
                        canonical=e.canonical,
                        severity="info",
                        message=(
                            f"{e.field_class} 字段 '{e.canonical}' 在故事中出现于 {all_matched}"
                        ),
                        aliases=all_matched,
                        matched_files=matched_files,
                    )
                )
            continue

        # 错误：弱匹配也失败（且 status 为 implemented，且 field_class 强制要求出现）
        # v3 严重度分级：
        #   gsp / audit         → error（GSP 法规强制，必须实现）
        #   business / config / interface → info（辅助字段由模块/防腐层归口，不污染 T1 warning）
        #   system / derived    → 已在上面跳过
        if not all_matched:
            if e.field_class in ("gsp", "audit"):
                severity = "error"
            else:
                severity = "info"
            issues.append(
                Issue(
                    canonical=e.canonical,
                    severity=severity,
                    message=(
                        f"{e.field_class} 字段 '{e.canonical}' 的所有 aliases "
                        f"{e.aliases} 在所有故事文档（含字段表 + 正文）中均未找到"
                    ),
                    aliases=e.aliases,
                )
            )
            continue

        # 信息：多个 alias 在故事中混用
        if len(all_matched) >= 2:
            # acceptable_alias 状态：多 alias 是合理的（细化语义 / 中英双语），不报 info
            if e.wms_status == "acceptable_alias":
                continue
            tag = "字段表" if strong_matched else "正文"
            issues.append(
                Issue(
                    canonical=e.canonical,
                    severity="info",
                    message=(
                        f"{e.field_class} 字段 '{e.canonical}' 在故事中混用了 "
                        f"{len(all_matched)} 个 alias: {all_matched}（"
                        f"主要在 {tag}；建议 glossary 规范化）"
                    ),
                    aliases=all_matched,
                    matched_files=matched_files,
                )
            )

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    entries, parse_errors = parse_field_dictionary()
    if parse_errors:
        if args.json:
            print(
                json.dumps(
                    {
                        "check": "check_gsp_field_traceability",
                        "tier": "T1",
                        "category": "文档治理",
                        "ok": False,
                        "parse_errors": parse_errors,
                    },
                    ensure_ascii=False,
                    indent=2,
                )
            )
        else:
            print(
                "check_gsp_field_traceability (T1, 文档治理) — 字段词典解析失败"
            )
            for err in parse_errors:
                print(f"  ✘ {err}")
        return 1

    field_to_files = scan_story_field_tables()
    full_text = scan_story_full_text()
    issues = check_field_coverage(entries, field_to_files, full_text)

    errors = [i for i in issues if i.severity == "error"]
    warnings = [i for i in issues if i.severity == "warning"]
    infos = [i for i in issues if i.severity == "info"]

    if args.json:
        print(
            json.dumps(
                {
                    "check": "check_gsp_field_traceability",
                    "tier": "T1",
                    "category": "文档治理",
                    "fields_total": len(entries),
                    "fields_covered": len(entries) - len(errors),
                    "story_fields_indexed": len(field_to_files),
                    "errors": [asdict(i) for i in errors],
                    "warnings": [asdict(i) for i in warnings],
                    "infos": [asdict(i) for i in infos],
                    "ok": not errors,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    else:
        print(
            f"check_gsp_field_traceability (T1, 文档治理) — "
            f"{len(entries)} GSP 字段 / "
            f"{len(field_to_files)} 故事字段表行"
        )
        if errors:
            print(f"\n  错误（{len(errors)} 项）：")
            for i in errors:
                print(f"    ✘ [{i.canonical}] {i.message}")
        if warnings:
            print(f"\n  警告（{len(warnings)} 项）：")
            for i in warnings:
                print(f"    ⚠ [{i.canonical}] {i.message}")
        if infos:
            print(f"\n  信息（{len(infos)} 项，不阻塞 T1）：")
            for i in infos:
                print(f"    ℹ [{i.canonical}] {i.message}")
        if not (errors or warnings):
            covered = len(entries) - len(errors)
            print(
                f"\n  ✓ {covered}/{len(entries)} GSP 字段在故事字段表中均有实现"
            )

    return 0 if not errors else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
