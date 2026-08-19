import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import generate_table_catalog as catalog


def seed_repo(tmp_path: Path) -> Path:
    migrations = tmp_path / "backend" / "migrations"
    migrations.mkdir(parents=True)
    migrations.joinpath("202601010001_demo.sql").write_text(
        """
        CREATE TABLE IF NOT EXISTS demo_orders (
            id UUID PRIMARY KEY,
            owner_id UUID NOT NULL,
            order_no TEXT NOT NULL,
            qty BIGINT NOT NULL CHECK (qty > 0),
            UNIQUE (owner_id, order_no)
        );

        CREATE TABLE IF NOT EXISTS demo_orders_2026_01 PARTITION OF demo_orders
            FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');

        CREATE UNIQUE INDEX IF NOT EXISTS demo_orders_owner_no_idx
            ON demo_orders (owner_id, order_no);
        """,
        encoding="utf-8",
    )
    return tmp_path


def test_table_catalog_parses_tables_columns_indexes_and_partitions(tmp_path):
    repo = seed_repo(tmp_path)

    tables = catalog.collect_catalog(repo)
    rendered = catalog.format_catalog(tables)

    assert [table.name for table in tables] == [
        "demo_orders",
        "demo_orders_2026_01",
    ]
    assert [column.name for column in tables[0].columns] == [
        "id",
        "owner_id",
        "order_no",
        "qty",
    ]
    assert tables[0].indexes == ["UNIQUE demo_orders_owner_no_idx"]
    assert tables[1].partition_of == "demo_orders"
    assert "`demo_orders`" in rendered
    assert "货主字段：有" in rendered
    assert "字段继承 `demo_orders`" in rendered


def test_table_catalog_check_mode_detects_stale_output(tmp_path, capsys):
    repo = seed_repo(tmp_path)
    output = tmp_path / "docs" / "database" / "table-catalog.md"

    assert catalog.main([
        "--repo-root",
        str(repo),
        "--output",
        str(output),
    ]) == 0
    assert catalog.main([
        "--repo-root",
        str(repo),
        "--output",
        str(output),
        "--check",
    ]) == 0

    output.write_text("stale\n", encoding="utf-8")

    assert catalog.main([
        "--repo-root",
        str(repo),
        "--output",
        str(output),
        "--check",
    ]) == 1
    assert "不是最新" in capsys.readouterr().err


def test_table_catalog_tracks_create_without_if_alter_references_and_schema_events(tmp_path):
    migrations = tmp_path / "backend" / "migrations"
    migrations.mkdir(parents=True)
    migrations.joinpath("202601010001_create.sql").write_text(
        """
        CREATE TABLE parent_orders (
            id UUID PRIMARY KEY
        );
        CREATE TABLE child_events (
            id UUID PRIMARY KEY,
            parent_id UUID REFERENCES parent_orders(id)
        );
        CREATE TABLE legacy_events (
            id UUID PRIMARY KEY
        );
        """,
        encoding="utf-8",
    )
    migrations.joinpath("202601010002_changes.sql").write_text(
        """
        ALTER TABLE child_events ADD COLUMN note TEXT;
        ALTER TABLE child_events
            ADD CONSTRAINT child_events_parent_fk FOREIGN KEY (parent_id)
            REFERENCES parent_orders(id);
        ALTER TABLE child_events RENAME TO child_event_log;
        DROP TABLE legacy_events;
        """,
        encoding="utf-8",
    )

    tables = catalog.collect_catalog(tmp_path)
    child = next(table for table in tables if table.name == "child_events")
    events = catalog.collect_schema_events(tmp_path)
    rendered = catalog.format_catalog(tables, events)

    assert child.references == ["parent_orders"]
    assert child.alter_migrations == ["backend/migrations/202601010002_changes.sql"]
    assert [table.name for table in tables] == ["parent_orders", "child_events"]
    assert {(event.kind, event.table, event.target) for event in events} == {
        ("alter", "child_events", None),
        ("rename", "child_events", "child_event_log"),
        ("drop", "legacy_events", None),
    }
    assert "## Schema 变更事件" in rendered
    assert "`child_event_log`" in rendered
