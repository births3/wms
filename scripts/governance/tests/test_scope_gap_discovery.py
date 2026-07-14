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


def test_scope_gap_default_scan_includes_modules_not_yet_in_matrix():
    from check_scope_gap_discovery import scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={
            "docs/domain/user-stories-m1.md": "## US-M1-001：商品档案",
            "docs/domain/user-stories-m4.md": "## US-M4-001：出库订单",
        },
        matrix_stories=[{"id": "US-M1-001", "module": "M1", "frontend_pages": [], "api_paths": []}],
        admin_pages={},
    )

    assert result.active_modules == ["M1", "M4"]
    assert [gap.story_id for gap in result.gaps] == ["US-M4-001"]


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


def test_scope_gap_blocks_matrix_frontend_page_that_is_not_in_default_tree_or_route():
    from check_scope_gap_discovery import AdminNavigation, scan_scope_gaps

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
        admin_pages={"m2-receiving": "M2 收货管理"},
        admin_navigation=AdminNavigation(
            menu_sections={"m2-receiving": "M2 收货管理"},
            default_menu_tree=set(),
            routed_views=set(),
        ),
    )

    assert not result.ok
    assert [gap.kind for gap in result.gaps] == [
        "frontend_page_not_in_default_menu_tree",
        "frontend_page_not_routed",
    ]


def test_scope_gap_blocks_matrix_frontend_page_that_is_not_in_dev_mock_published_menu():
    from check_scope_gap_discovery import AdminNavigation, scan_scope_gaps

    result = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-h4-wechat-notify.md": "## US-H4-001：通知配置"},
        matrix_stories=[
            {
                "id": "US-H4-001",
                "module": "H4",
                "frontend_pages": ["h4-notify-configs"],
                "api_paths": [],
            }
        ],
        admin_pages={"h4-notify-configs": "H4 通知配置"},
        admin_navigation=AdminNavigation(
            menu_sections={"h4-notify-configs": "H4 通知配置"},
            default_menu_tree={"h4-notify-configs"},
            routed_views={"h4-notify-configs"},
            dev_mock_published_views={"h1-menu-management"},
        ),
    )

    assert not result.ok
    assert [gap.kind for gap in result.gaps] == ["frontend_page_not_in_dev_mock_published_menu"]
    assert result.gaps[0].severity == "block"


def test_scope_gap_discovers_frontend_story_without_e2e_checks():
    from check_scope_gap_discovery import AdminNavigation, scan_scope_gaps

    navigation = AdminNavigation(
        menu_sections={"m2-receiving": "M2 收货管理"},
        default_menu_tree={"m2-receiving"},
        routed_views={"m2-receiving"},
    )
    result = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-m2-inbound.md": "## US-M2-002：收货管理"},
        matrix_stories=[
            {
                "id": "US-M2-002",
                "module": "M2",
                "types": ["frontend_interaction"],
                "frontend_pages": ["m2-receiving"],
                "api_paths": [],
            }
        ],
        admin_pages=navigation.menu_sections,
        admin_navigation=navigation,
    )

    assert result.ok
    assert not result.strict_ok
    assert [gap.kind for gap in result.gaps] == ["frontend_story_missing_e2e_check"]
    assert result.gaps[0].severity == "discover"

    with_e2e = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-m2-inbound.md": "## US-M2-002：收货管理"},
        matrix_stories=[
            {
                "id": "US-M2-002",
                "module": "M2",
                "types": ["frontend_interaction"],
                "frontend_pages": ["m2-receiving"],
                "api_paths": [],
                "e2e_checks": ["pnpm --dir apps/web-admin run test:self-checks"],
            }
        ],
        admin_pages=navigation.menu_sections,
        admin_navigation=navigation,
    )

    assert with_e2e.ok
    assert with_e2e.strict_ok
    assert with_e2e.gaps == []


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


def test_scope_gap_navigation_reader_supports_split_app_shell_renderer():
    from check_scope_gap_discovery import read_admin_navigation

    navigation = read_admin_navigation()

    assert "m1-products" in navigation.menu_sections
    assert "m3-batches" in navigation.default_menu_tree
    assert "m1-products" in navigation.routed_views
    assert "m3-batches" in navigation.routed_views


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
                "owner": "H9 模块负责人",
                "resume_when": "H9 模板设计器进入开发波次。",
            }
        ],
        admin_pages={"h9-print-templates": "H9 打印模板"},
        modules={"H9"},
    )

    assert result.ok
    assert result.strict_ok
    assert result.deferred_story_ids == ["US-H9-003"]
    assert result.gaps == []


def test_scope_gap_deferred_story_can_cover_existing_menu_but_requires_e2e() -> None:
    from check_scope_gap_discovery import AdminNavigation, scan_scope_gaps

    navigation = AdminNavigation(
        menu_sections={"m2-inspecting": "M2 验收管理"},
        default_menu_tree={"m2-inspecting"},
        routed_views={"m2-inspecting"},
    )
    deferred = {
        "id": "US-M2-003",
        "module": "M2",
        "reason": "PDA 尚未实现。",
        "owner": "M2 模块负责人",
        "resume_when": "PDA 波次启动。",
        "frontend_pages": ["m2-inspecting"],
    }
    missing = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-m2.md": "## US-M2-003：PDA/PC Web 验收"},
        matrix_stories=[],
        deferred_stories=[deferred],
        admin_pages=navigation.menu_sections,
        admin_navigation=navigation,
    )
    assert [gap.kind for gap in missing.gaps] == ["deferred_frontend_story_missing_e2e_check"]

    deferred["e2e_checks"] = ["pnpm --dir apps/web-admin run test:e2e:m2-real"]
    covered = scan_scope_gaps(
        story_docs={"docs/domain/user-stories-m2.md": "## US-M2-003：PDA/PC Web 验收"},
        matrix_stories=[],
        deferred_stories=[deferred],
        admin_pages=navigation.menu_sections,
        admin_navigation=navigation,
    )
    assert covered.gaps == []


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
        "deferred_story_missing_owner",
        "deferred_story_missing_resume_when",
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
