import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_admin_navigation as check


def test_admin_navigation_accepts_current_repaired_menu_ids():
    assert check.scan_menu_id_collisions(check.MIGRATIONS_DIR, check.MENU_ID_COLLISION_REPAIR) == []


def test_admin_navigation_rejects_new_literal_menu_id_collision(tmp_path):
    migrations = tmp_path / "migrations"
    migrations.mkdir()
    node_id = "00000000-0000-0000-0000-000000130099"
    parent_id = "00000000-0000-0000-0000-000000120099"
    for index, code in enumerate(("platform.first", "platform.second"), start=1):
        (migrations / f"{index}.sql").write_text(
            f"VALUES ('{node_id}', '{parent_id}', 3, '{code}', 'path', 'title', NULL, 'Box', 'permission', 10, TRUE);",
            encoding="utf-8",
        )

    issues = check.scan_menu_id_collisions(migrations, tmp_path / "missing-repair.sql")

    assert len(issues) == 1
    assert node_id in issues[0].message
    assert "platform.first" in issues[0].message
    assert "platform.second" in issues[0].message
