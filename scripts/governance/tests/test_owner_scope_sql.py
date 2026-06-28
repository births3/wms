import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_owner_scope_sql as check


def seed_repo(tmp_path: Path, source: str) -> Path:
    repo = tmp_path
    migrations = repo / "backend" / "migrations"
    migrations.mkdir(parents=True)
    migrations.joinpath("001.sql").write_text(
        """
        CREATE TABLE IF NOT EXISTS inbound_orders (
            id UUID PRIMARY KEY,
            owner_id UUID NOT NULL,
            status TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS auth_user_owner_bindings (
            user_id UUID NOT NULL,
            owner_id UUID NOT NULL
        );

        CREATE TABLE IF NOT EXISTS public_codes (
            code TEXT PRIMARY KEY
        );
        """,
        encoding="utf-8",
    )
    src = repo / "backend" / "crates" / "api" / "src"
    src.mkdir(parents=True)
    src.joinpath("wave9_repository.rs").write_text(source, encoding="utf-8")
    return repo


def test_owner_scope_sql_accepts_scoped_repository_queries(tmp_path):
    repo = seed_repo(
        tmp_path,
        '''
        fn ok() {
            sqlx::query(r#"
                SELECT id, owner_id, status
                  FROM inbound_orders
                 WHERE owner_id = $1 AND id = $2
            "#);
            sqlx::query(r#"
                INSERT INTO inbound_orders (id, owner_id, status)
                VALUES ($1, $2, $3)
            "#);
        }
        ''',
    )

    assert check.run(repo).issues == []


def test_owner_scope_sql_rejects_select_without_owner_predicate(tmp_path):
    repo = seed_repo(
        tmp_path,
        '''
        fn leak() {
            sqlx::query("SELECT id, owner_id, status FROM inbound_orders WHERE id = $1");
        }
        ''',
    )

    issues = check.run(repo).issues

    assert [(issue.kind, issue.table) for issue in issues] == [
        ("missing_owner_predicate", "inbound_orders")
    ]


def test_owner_scope_sql_rejects_insert_missing_owner_column(tmp_path):
    repo = seed_repo(
        tmp_path,
        '''
        fn leak() {
            sqlx::query(r#"
                INSERT INTO inbound_orders (id, status)
                VALUES ($1, $2)
            "#);
        }
        ''',
    )

    issues = check.run(repo).issues

    assert [(issue.kind, issue.table) for issue in issues] == [
        ("missing_insert_owner_id", "inbound_orders")
    ]


def test_owner_scope_sql_ignores_unscoped_tables(tmp_path):
    repo = seed_repo(
        tmp_path,
        'fn ok() { sqlx::query("SELECT code FROM public_codes WHERE code = $1"); }',
    )

    assert check.run(repo).issues == []


def test_owner_scope_sql_scans_auth_repository(tmp_path):
    repo = seed_repo(tmp_path, 'fn ok() {}')
    auth_repo = repo / "backend" / "crates" / "api" / "src" / "auth_repository.rs"
    auth_repo.write_text(
        'fn login() { sqlx::query("SELECT user_id FROM auth_user_owner_bindings WHERE user_id = $1"); }',
        encoding="utf-8",
    )

    issues = check.run(repo).issues

    assert [(issue.kind, issue.table) for issue in issues] == [
        ("missing_owner_predicate", "auth_user_owner_bindings")
    ]
