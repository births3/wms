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
