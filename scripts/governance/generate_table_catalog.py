#!/usr/bin/env python3
"""从 backend/migrations 生成数据库表目录。"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

CREATE_TABLE_HEAD_RE = re.compile(
    r"\bCREATE\s+TABLE(?:\s+IF\s+NOT\s+EXISTS)?\s+([a-z_][a-z0-9_]*)",
    re.IGNORECASE,
)
ALTER_TABLE_RE = re.compile(
    r"\bALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:ONLY\s+)?([a-z_][a-z0-9_]*)",
    re.IGNORECASE,
)
DROP_TABLE_RE = re.compile(
    r"\bDROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?([a-z_][a-z0-9_]*)",
    re.IGNORECASE,
)
RENAME_TABLE_RE = re.compile(
    r"\bALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:ONLY\s+)?"
    r"([a-z_][a-z0-9_]*)\s+RENAME\s+TO\s+([a-z_][a-z0-9_]*)",
    re.IGNORECASE,
)
REFERENCES_RE = re.compile(r"\bREFERENCES\s+([a-z_][a-z0-9_]*)", re.IGNORECASE)
PARTITION_RE = re.compile(
    r"PARTITION\s+OF\s+([a-z_][a-z0-9_]*)",
    re.IGNORECASE,
)
CREATE_INDEX_RE = re.compile(
    r"\bCREATE\s+(UNIQUE\s+)?INDEX\s+IF\s+NOT\s+EXISTS\s+"
    r"([a-z_][a-z0-9_]*)\s+ON\s+([a-z_][a-z0-9_]*)",
    re.IGNORECASE,
)
CONSTRAINT_KEYWORDS = {
    "check",
    "constraint",
    "exclude",
    "foreign",
    "primary",
    "unique",
}


@dataclass(frozen=True)
class ColumnInfo:
    name: str
    definition: str


@dataclass
class TableInfo:
    name: str
    migration: str
    module: str
    columns: list[ColumnInfo]
    indexes: list[str]
    partition_of: str | None = None
    references: list[str] = field(default_factory=list)
    alter_migrations: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class SchemaEvent:
    kind: str
    table: str
    migration: str
    target: str | None = None


def module_name(path: Path) -> str:
    name = re.sub(r"^\d+_", "", path.stem)
    mapping = {
        "audit_event": "Wave 1 审计",
        "wave3_core_tables": "Wave 3 入库 / 库存 / 冷链 / 计费",
        "wave4_outbound_tables": "Wave 4 出库 / 追溯",
        "wave5_value_added_tables": "Wave 5 增值 / TMS / 计费",
        "h1_auth_tables": "H1 鉴权 / 货主访问",
        "system_dictionary": "M1 系统字典",
        "database_design_standard_alignment": "M1 主数据 / 数据库规范对齐",
    }
    return mapping.get(name, name)


def read_parenthesized(text: str, start: int) -> tuple[str, int]:
    depth = 0
    in_single_quote = False
    index = start
    while index < len(text):
        char = text[index]
        if in_single_quote:
            if char == "'" and index + 1 < len(text) and text[index + 1] == "'":
                index += 2
                continue
            if char == "'":
                in_single_quote = False
        elif char == "'":
            in_single_quote = True
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return text[start + 1:index], index + 1
        index += 1
    raise ValueError("CREATE TABLE body is not closed")


def split_top_level_commas(text: str) -> list[str]:
    parts: list[str] = []
    start = 0
    depth = 0
    in_single_quote = False
    index = 0
    while index < len(text):
        char = text[index]
        if in_single_quote:
            if char == "'" and index + 1 < len(text) and text[index + 1] == "'":
                index += 2
                continue
            if char == "'":
                in_single_quote = False
        elif char == "'":
            in_single_quote = True
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(text[start:index])
            start = index + 1
        index += 1
    parts.append(text[start:])
    return parts


def parse_columns(body: str) -> list[ColumnInfo]:
    columns: list[ColumnInfo] = []
    for part in split_top_level_commas(body):
        definition = re.sub(r"\s+", " ", part.strip())
        if not definition:
            continue
        first = definition.split(maxsplit=1)[0].strip('"').lower()
        if first in CONSTRAINT_KEYWORDS:
            continue
        if not re.match(r"^[a-z_][a-z0-9_]*$", first):
            continue
        columns.append(ColumnInfo(first, definition))
    return columns


def parse_tables(sql: str, migration_path: Path) -> list[TableInfo]:
    tables: list[TableInfo] = []
    module = module_name(migration_path)
    migration = f"backend/migrations/{migration_path.name}"
    for match in CREATE_TABLE_HEAD_RE.finditer(sql):
        table_name = match.group(1).lower()
        cursor = match.end()
        while cursor < len(sql) and sql[cursor].isspace():
            cursor += 1

        if cursor < len(sql) and sql[cursor] == "(":
            body, _ = read_parenthesized(sql, cursor)
            tables.append(
                TableInfo(
                    table_name,
                    migration,
                    module,
                    parse_columns(body),
                    [],
                    references=sorted(set(REFERENCES_RE.findall(body.lower()))),
                )
            )
            continue

        partition = PARTITION_RE.match(sql, cursor)
        if partition:
            tables.append(TableInfo(
                table_name,
                migration,
                module,
                [],
                [],
                partition_of=partition.group(1).lower(),
            ))
    return tables


def parse_schema_events(sql: str, migration_path: Path) -> list[SchemaEvent]:
    migration = f"backend/migrations/{migration_path.name}"
    events = [
        SchemaEvent("alter", match.group(1).lower(), migration)
        for match in ALTER_TABLE_RE.finditer(sql)
    ]
    events.extend(
        SchemaEvent("drop", match.group(1).lower(), migration)
        for match in DROP_TABLE_RE.finditer(sql)
    )
    events.extend(
        SchemaEvent("rename", match.group(1).lower(), migration, match.group(2).lower())
        for match in RENAME_TABLE_RE.finditer(sql)
    )
    return events


def parse_alter_references(sql: str) -> dict[str, list[str]]:
    references: dict[str, list[str]] = {}
    statements = re.finditer(
        r"\bALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:ONLY\s+)?"
        r"(?P<table>[a-z_][a-z0-9_]*)(?P<body>.*?);",
        sql,
        re.IGNORECASE | re.DOTALL,
    )
    for statement in statements:
        table = statement.group("table").lower()
        targets = {target.lower() for target in REFERENCES_RE.findall(statement.group("body"))}
        if targets:
            references.setdefault(table, []).extend(sorted(targets))
    return references


def parse_indexes(sql: str) -> dict[str, list[str]]:
    indexes: dict[str, list[str]] = {}
    for match in CREATE_INDEX_RE.finditer(sql):
        unique = "UNIQUE " if match.group(1) else ""
        index_name = match.group(2).lower()
        table_name = match.group(3).lower()
        indexes.setdefault(table_name, []).append(f"{unique}{index_name}".strip())
    return indexes


def collect_catalog(repo_root: Path = REPO_ROOT) -> list[TableInfo]:
    migrations_dir = repo_root / "backend" / "migrations"
    tables: list[TableInfo] = []
    alter_migrations: dict[str, set[str]] = {}
    alter_references: dict[str, set[str]] = {}
    for path in sorted(migrations_dir.glob("*.sql")):
        sql = path.read_text(encoding="utf-8")
        parsed_tables = parse_tables(sql, path)
        parsed_indexes = parse_indexes(sql)
        migration = f"backend/migrations/{path.name}"
        for event in parse_schema_events(sql, path):
            if event.kind == "alter":
                alter_migrations.setdefault(event.table, set()).add(migration)
        for table, references in parse_alter_references(sql).items():
            alter_references.setdefault(table, set()).update(references)
        for table in parsed_tables:
            table.indexes = sorted(parsed_indexes.get(table.name, []))
        tables.extend(parsed_tables)
    for table in tables:
        table.references = sorted(set(table.references) | alter_references.get(table.name, set()))
        table.alter_migrations = sorted(
            migration
            for migration in alter_migrations.get(table.name, set())
            if migration != table.migration
        )
    return tables


def collect_schema_events(repo_root: Path = REPO_ROOT) -> list[SchemaEvent]:
    migrations_dir = repo_root / "backend" / "migrations"
    events: list[SchemaEvent] = []
    for path in sorted(migrations_dir.glob("*.sql")):
        events.extend(parse_schema_events(path.read_text(encoding="utf-8"), path))
    return sorted(set(events), key=lambda event: (event.migration, event.table, event.kind, event.target or ""))


def owner_scope(table: TableInfo) -> str:
    if table.partition_of:
        return f"继承 {table.partition_of}"
    if any(column.name == "owner_id" for column in table.columns):
        return "有"
    return "无"


def md(value: str) -> str:
    return value.replace("|", "\\|").replace("\n", " ")


def format_catalog(tables: list[TableInfo], schema_events: list[SchemaEvent] | None = None) -> str:
    index_count = sum(len(table.indexes) for table in tables)
    migrations = sorted({table.migration for table in tables})
    lines = [
        "# 数据库表目录",
        "",
        "> 本文件由 `python3 scripts/governance/generate_table_catalog.py` "
        "从 `backend/migrations/*.sql` 生成；不要手工修改表清单。"
        "业务解释以用户故事、ADR 和迁移脚本为准。"
        "本文件随表数量自然超过普通文档行数阈值，行数门禁按生成物处理。",
        "",
        "## 统计",
        "",
        f"- 迁移文件：{len(migrations)}",
        f"- 数据表：{len(tables)}",
        f"- 索引：{index_count}",
        "",
        "## 表清单",
        "",
        "| 表 | 模块 | 创建迁移 | 货主字段 | 字段数 | 索引数 | ALTER 迁移数 | 引用表数 |",
        "|---|---|---|---|---:|---:|---:|---:|",
    ]
    for table in tables:
        lines.append(
            f"| `{table.name}` | {md(table.module)} | `{table.migration}` | "
            f"{md(owner_scope(table))} | {len(table.columns)} | {len(table.indexes)} | "
            f"{len(table.alter_migrations)} | {len(table.references)} |"
        )

    lines.extend(["", "## 字段明细", ""])
    for table in tables:
        lines.extend([
            f"### `{table.name}`",
            "",
            f"- 模块：{table.module}",
            f"- 迁移：`{table.migration}`",
            f"- 货主字段：{owner_scope(table)}",
            f"- 索引：{', '.join(f'`{index}`' for index in table.indexes) if table.indexes else '无'}",
            f"- ALTER 迁移：{', '.join(f'`{migration}`' for migration in table.alter_migrations) if table.alter_migrations else '无'}",
            f"- 引用表：{', '.join(f'`{reference}`' for reference in table.references) if table.references else '无'}",
            "",
        ])
        if table.partition_of:
            lines.extend([f"分区表，字段继承 `{table.partition_of}`。", ""])
            continue

        lines.extend(["| 字段 | SQL 定义 |", "|---|---|"])
        for column in table.columns:
            lines.append(f"| `{column.name}` | `{md(column.definition)}` |")
        lines.append("")
    events = schema_events or []
    lines.extend([
        "## Schema 变更事件",
        "",
        "| 类型 | 表 | 目标 | 迁移 |",
        "|---|---|---|---|",
    ])
    for event in events:
        lines.append(
            f"| {event.kind} | `{event.table}` | "
            f"{f'`{event.target}`' if event.target else '—'} | `{event.migration}` |"
        )
    return "\n".join(lines).rstrip() + "\n"


def resolved_output(repo_root: Path, output: str) -> Path:
    path = Path(output)
    if path.is_absolute():
        return path
    return repo_root / path


def write_catalog(content: str, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    parser.add_argument("--output", default="docs/database/table-catalog.md")
    parser.add_argument("--check", action="store_true", help="只检查输出文件是否最新")
    parser.add_argument("--json", action="store_true", help="输出机器可读摘要")
    args = parser.parse_args(argv)

    tables = collect_catalog(args.repo_root)
    content = format_catalog(tables, collect_schema_events(args.repo_root))
    output = resolved_output(args.repo_root, args.output)
    current = output.read_text(encoding="utf-8") if output.exists() else None
    ok = current == content

    if args.check:
        if args.json:
            print(json.dumps({
                "script": "generate_table_catalog",
                "output": output.as_posix(),
                "tables": len(tables),
                "indexes": sum(len(table.indexes) for table in tables),
                "ok": ok,
            }, ensure_ascii=False))
        elif ok:
            print(f"数据库表目录已是最新：{output}")
        else:
            print(
                "数据库表目录不是最新，请运行："
                "python3 scripts/governance/generate_table_catalog.py",
                file=sys.stderr,
            )
        return 0 if ok else 1

    write_catalog(content, output)
    if args.json:
        print(json.dumps({
            "script": "generate_table_catalog",
            "output": output.as_posix(),
            "tables": len(tables),
            "indexes": sum(len(table.indexes) for table in tables),
            "ok": True,
        }, ensure_ascii=False))
    else:
        print(f"已生成数据库表目录：{output}（{len(tables)} 张表）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
