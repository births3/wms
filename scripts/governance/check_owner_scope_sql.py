#!/usr/bin/env python3
"""check_owner_scope_sql.py — 后端仓储 SQL 货主隔离静态检查

类别：5. 代码治理
Tier：T1（< 10s，纯静态扫描）
输入：backend/migrations/*.sql + backend/crates/api/src/*repository.rs
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

说明：
  该脚本从 migration 中识别包含 owner_id 的租户表，并检查仓储层 SQL：
  - INSERT INTO 租户表时，字段列表必须包含 owner_id
  - SELECT / UPDATE / DELETE 访问租户表时，WHERE 谓词必须包含 owner_id

  这是项目级静态门禁，用来补足 RTM 中“货主隔离缺少自动扫描”的缺口；
  不替代 handler / repository 集成测试。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MIGRATIONS_DIR = REPO_ROOT / "backend" / "migrations"
API_SRC_DIR = REPO_ROOT / "backend" / "crates" / "api" / "src"

CREATE_TABLE_RE = re.compile(
    r"CREATE\s+TABLE\s+IF\s+NOT\s+EXISTS\s+([a-z_][a-z0-9_]*)\s*\((.*?)\)\s*(?:PARTITION\s+BY\s+[^;]+)?;",
    re.IGNORECASE | re.DOTALL,
)
RAW_STRING_RE = re.compile(r'(?<![A-Za-z0-9_])r(?P<hashes>#*)"(.*?)"(?P=hashes)', re.DOTALL)
NORMAL_STRING_RE = re.compile(r'"(?:\\.|[^"\\])*"', re.DOTALL)
SQL_RE = re.compile(r"\b(SELECT|INSERT\s+INTO|UPDATE|DELETE\s+FROM)\b", re.IGNORECASE)


@dataclass(frozen=True)
class SqlLiteral:
    path: Path
    line: int
    sql: str


@dataclass(frozen=True)
class Issue:
    kind: str
    path: str
    line: int
    table: str
    detail: str


@dataclass(frozen=True)
class Result:
    scoped_tables: list[str]
    scanned_files: list[str]
    issues: list[Issue]


def owner_scoped_tables(migrations_dir: Path = MIGRATIONS_DIR) -> set[str]:
    tables: set[str] = set()
    for path in sorted(migrations_dir.glob("*.sql")):
        text = path.read_text(encoding="utf-8")
        for match in CREATE_TABLE_RE.finditer(text):
            table = match.group(1).lower()
            body = match.group(2).lower()
            if re.search(r"\bowner_id\b", body):
                tables.add(table)
    return tables


def repository_files(api_src_dir: Path = API_SRC_DIR) -> list[Path]:
    if not api_src_dir.exists():
        return []
    return sorted(api_src_dir.glob("*repository.rs"))


def extract_string_literals(path: Path) -> list[SqlLiteral]:
    text = path.read_text(encoding="utf-8")
    literals: list[SqlLiteral] = []
    masked = list(text)

    for match in RAW_STRING_RE.finditer(text):
        sql = match.group(2)
        literals.append(SqlLiteral(path, text.count("\n", 0, match.start()) + 1, sql))
        for index in range(match.start(), match.end()):
            masked[index] = " "

    masked_text = "".join(masked)
    for match in NORMAL_STRING_RE.finditer(masked_text):
        raw = match.group(0)[1:-1]
        sql = raw.encode("utf-8").decode("unicode_escape")
        literals.append(SqlLiteral(path, text.count("\n", 0, match.start()) + 1, sql))

    return sorted(literals, key=lambda item: (item.path.as_posix(), item.line))


def normalize_sql(sql: str) -> str:
    sql = re.sub(r"--.*?$", " ", sql, flags=re.MULTILINE)
    return re.sub(r"\s+", " ", sql).strip().lower()


def predicate_region(sql: str, table: str) -> str:
    match = re.search(
        rf"(\bfrom\s+{re.escape(table)}\b|\bjoin\s+{re.escape(table)}\b|\bupdate\s+{re.escape(table)}\b|\bdelete\s+from\s+{re.escape(table)}\b)(?P<predicate>.*?)(\border\s+by\b|\bgroup\s+by\b|\blimit\b|\bfor\s+update\b|\breturning\b|$)",
        sql,
        flags=re.DOTALL,
    )
    return match.group("predicate") if match else ""


def has_owner_predicate(sql: str, table: str) -> bool:
    return bool(re.search(r"\bowner_id\b", predicate_region(sql, table)))


def insert_has_owner_id(sql: str, table: str) -> bool:
    match = re.search(
        rf"\binsert\s+into\s+{re.escape(table)}\s*\((?P<columns>.*?)\)\s*(values|select)",
        sql,
        flags=re.DOTALL,
    )
    return bool(match and re.search(r"\bowner_id\b", match.group("columns")))


def referenced(sql: str, table: str, pattern: str) -> bool:
    return bool(re.search(pattern.format(table=re.escape(table)), sql, flags=re.DOTALL))


def inspect_literal(literal: SqlLiteral, scoped_tables: set[str], repo_root: Path) -> list[Issue]:
    sql = normalize_sql(literal.sql)
    if not SQL_RE.search(sql):
        return []

    issues: list[Issue] = []
    path = literal.path.relative_to(repo_root).as_posix()
    for table in sorted(scoped_tables):
        insert_target = referenced(sql, table, r"\binsert\s+into\s+{table}\b")
        predicate_target = any(
            referenced(sql, table, pattern)
            for pattern in (
                r"\bfrom\s+{table}\b",
                r"\bjoin\s+{table}\b",
                r"\bupdate\s+{table}\b",
                r"\bdelete\s+from\s+{table}\b",
            )
        )

        if insert_target and not insert_has_owner_id(sql, table):
            issues.append(Issue(
                "missing_insert_owner_id",
                path,
                literal.line,
                table,
                "INSERT INTO 租户表的字段列表缺少 owner_id",
            ))
        if predicate_target and not has_owner_predicate(sql, table):
            issues.append(Issue(
                "missing_owner_predicate",
                path,
                literal.line,
                table,
                "访问租户表的 SELECT/UPDATE/DELETE/INSERT-SELECT 缺少 owner_id 谓词或 JOIN 条件",
            ))
    return issues


def run(repo_root: Path = REPO_ROOT) -> Result:
    migrations_dir = repo_root / "backend" / "migrations"
    api_src_dir = repo_root / "backend" / "crates" / "api" / "src"
    scoped_tables = owner_scoped_tables(migrations_dir)
    files = repository_files(api_src_dir)
    issues: list[Issue] = []
    for path in files:
        for literal in extract_string_literals(path):
            issues.extend(inspect_literal(literal, scoped_tables, repo_root))
    return Result(
        scoped_tables=sorted(scoped_tables),
        scanned_files=[path.relative_to(repo_root).as_posix() for path in files],
        issues=issues,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    result = run()
    payload = {
        "check": "check_owner_scope_sql",
        "tier": "T1",
        "category": "代码治理",
        "scoped_table_count": len(result.scoped_tables),
        "scanned_files": result.scanned_files,
        "issues": [asdict(issue) for issue in result.issues],
        "ok": not result.issues,
    }

    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_owner_scope_sql (T1, 代码治理)")
        print(f"  · owner scoped tables: {len(result.scoped_tables)}")
        print(f"  · scanned repository files: {len(result.scanned_files)}")
        if result.issues:
            print(f"  ✘ {len(result.issues)} 处 owner scope SQL 缺口:")
            for issue in result.issues:
                print(f"    [{issue.kind}] {issue.path}:{issue.line} {issue.table}: {issue.detail}")
        else:
            print("  ✓ 仓储层租户表 SQL 均显式包含 owner_id 写入或过滤")

    return 0 if not result.issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"script error: {exc}", file=sys.stderr)
        sys.exit(2)
