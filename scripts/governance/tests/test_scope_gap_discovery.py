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


def test_scope_gap_module_filter_keeps_requested_module_without_other_gaps():
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

    assert result.active_modules == ["H9"]
    assert result.ok
    assert result.strict_ok
    assert result.gaps == []


def test_scope_gap_module_filter_scans_requested_module_without_existing_matrix_story():
    from check_scope_gap_discovery import scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={
            "docs/domain/user-stories-h2-audit-trail.md": "## US-H2-001：审计事件统一记录"
        },
        matrix_stories=[],
        admin_pages={},
        modules={"H2"},
    )

    assert result.active_modules == ["H2"]
    assert result.ok
    assert not result.strict_ok
    assert [gap.story_id for gap in result.gaps] == ["US-H2-001"]
    assert result.gaps[0].kind == "unregistered_story_in_active_module"


def test_scope_gap_accepts_deferred_story_with_reason():
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
        deferred_stories=[
            {
                "id": "US-H9-003",
                "module": "H9",
                "story_file": "docs/domain/user-stories-h9-print-template.md",
                "reason": "后续切片实现模板设计器。",
            }
        ],
        admin_pages={"h9-print-templates": "H9 打印模板"},
        modules={"H9"},
    )

    assert result.ok
    assert result.strict_ok
    assert result.deferred_story_ids == ["US-H9-003"]
    assert result.gaps == []


def test_scope_gap_blocks_invalid_deferred_story():
    from check_scope_gap_discovery import scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-h9-print-template.md": "## US-H9-001：打印模板类型字典"},
        matrix_stories=[
            {
                "id": "US-H9-001",
                "module": "H9",
                "frontend_pages": ["h9-print-templates"],
                "api_paths": [],
            }
        ],
        deferred_stories=[
            {
                "id": "US-H9-003",
                "module": "H9",
                "story_file": "docs/domain/user-stories-h9-print-template.md",
                "reason": "",
            }
        ],
        admin_pages={"h9-print-templates": "H9 打印模板"},
        modules={"H9"},
    )

    assert not result.ok
    assert [gap.kind for gap in result.gaps] == [
        "deferred_story_missing_from_story_docs",
        "deferred_story_missing_reason",
    ]


def test_scope_gap_blocks_deferred_story_without_id():
    from check_scope_gap_discovery import scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-h9-print-template.md": "## US-H9-001：打印模板类型字典"},
        matrix_stories=[
            {
                "id": "US-H9-001",
                "module": "H9",
                "frontend_pages": ["h9-print-templates"],
                "api_paths": [],
            }
        ],
        deferred_stories=[{"reason": "缺少故事 ID。"}],
        admin_pages={"h9-print-templates": "H9 打印模板"},
    )

    assert not result.ok
    assert result.gaps[0].kind == "deferred_story_missing_id"
