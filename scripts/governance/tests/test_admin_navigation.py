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


def test_admin_navigation_view_contract_matches_all_sources():
    issues = check.scan_view_contract(
        check.APP_TSX.read_text(encoding="utf-8"),
        check.ADMIN_VIEW_TS.read_text(encoding="utf-8"),
        check.MASTER_DATA_TYPES_TS.read_text(encoding="utf-8"),
        check.ADMIN_VIEW_RENDERER_TSX.read_text(encoding="utf-8"),
        check.ADMIN_MENU_DEV_MOCK_TS.read_text(encoding="utf-8"),
    )

    assert issues == []


def test_admin_navigation_view_contract_rejects_missing_renderer_view():
    renderer = check.ADMIN_VIEW_RENDERER_TSX.read_text(encoding="utf-8")
    renderer = renderer.replace('if (view === "h5-express") return <H5ExpressPage />;', "")

    issues = check.scan_view_contract(
        check.APP_TSX.read_text(encoding="utf-8"),
        check.ADMIN_VIEW_TS.read_text(encoding="utf-8"),
        check.MASTER_DATA_TYPES_TS.read_text(encoding="utf-8"),
        renderer,
        check.ADMIN_MENU_DEV_MOCK_TS.read_text(encoding="utf-8"),
    )

    renderer_issues = [issue for issue in issues if "renderer" in issue.message and "h5-express" in issue.message]
    assert renderer_issues
    assert renderer_issues[0].file == "apps/web-admin/src/app-shell/AdminViewRenderer.tsx"
