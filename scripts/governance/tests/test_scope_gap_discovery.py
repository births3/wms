import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_scope_gap_reports_unregistered_stories_in_active_module_without_default_blocking():
    from check_scope_gap_discovery import scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={
            "docs/domain/user-stories-h9-print-template.md": "\n".join(
                [
                    "## US-H9-001：打印模板类型字典",
                    "## US-H9-003：模板设计与版本管理",
                ]
            )
        },
        matrix_stories=[
            {
                "id": "US-H9-001",
                "module": "H9",
                "frontend_pages": ["h9-print-templates"],
                "api_paths": [],
            }
        ],
        admin_pages={"h9-print-templates": "H9 打印模板"},
    )

    assert result.ok
    assert not result.strict_ok
    assert [gap.story_id for gap in result.gaps] == ["US-H9-003"]
    assert result.gaps[0].kind == "unregistered_story_in_active_module"
    assert result.gaps[0].severity == "discover"


def test_scope_gap_blocks_matrix_frontend_page_that_is_not_in_menu():
    from check_scope_gap_discovery import scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-m2-inbound.md": "## US-M2-002：收货管理"},
        matrix_stories=[
            {
                "id": "US-M2-002",
                "module": "M2",
                "frontend_pages": ["m2-receiving"],
                "api_paths": [],
            }
        ],
        admin_pages={},
    )

    assert not result.ok
    assert not result.strict_ok
    assert result.gaps[0].kind == "frontend_page_not_in_menu"
    assert result.gaps[0].severity == "block"


def test_scope_gap_module_filter_does_not_scan_other_active_modules():
    from check_scope_gap_discovery import scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-m2-inbound.md": "## US-M2-002：收货管理"},
        matrix_stories=[
            {
                "id": "US-M2-002",
                "module": "M2",
                "frontend_pages": ["m2-receiving"],
                "api_paths": [],
            }
        ],
        admin_pages={},
        modules={"H9"},
    )

    assert result.active_modules == []
    assert result.ok
    assert result.strict_ok
    assert result.gaps == []
