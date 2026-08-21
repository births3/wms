"""对抗测试目录：T1 不强制覆盖，模块验收才检查。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_write_story_requires_idor_permission_state_and_idempotency():
    from check_quality_matrix import derive_required_attack_classes

    assert derive_required_attack_classes(["write"]) == ["A1", "A2", "A3", "A4"]


def test_read_only_story_requires_idor_and_permission_only():
    from check_quality_matrix import derive_required_attack_classes

    assert derive_required_attack_classes(["read_only", "frontend_interaction"]) == ["A1", "A2"]


def test_inventory_and_concurrent_union_includes_qty_gsp_and_race():
    from check_quality_matrix import derive_required_attack_classes

    assert derive_required_attack_classes(["write", "inventory_change", "concurrent_resource"]) == [
        "A1",
        "A2",
        "A3",
        "A4",
        "A5",
        "A6",
        "A7",
    ]


def test_api_change_only_does_not_require_adversarial_classes():
    from check_quality_matrix import derive_required_attack_classes

    assert derive_required_attack_classes(["api_change"]) == []


def test_integration_does_not_require_forged_receipt_class():
    from check_quality_matrix import derive_required_attack_classes

    assert derive_required_attack_classes(["read_only", "integration"]) == ["A1", "A2", "A4"]
    assert "A8" not in derive_required_attack_classes(["write", "integration"])


def test_t1_does_not_require_adversarial_checks_on_write_stories():
    from check_quality_matrix import check_story

    story = {
        "id": "US-H6-002",
        "title": "状态执行",
        "module": "H6",
        "types": ["write", "integration", "audit_compliance"],
        "story_file": "docs/domain/user-stories-h6-state-machine.md",
        "dimensions": {
            "requirement": "verified",
            "fields": "verified",
            "frontend": "not_applicable",
            "api": "verified",
            "backend": "verified",
            "database": "verified",
            "security": "verified",
            "audit": "verified",
            "tests": "verified",
            "evidence": "verified",
            "docs": "verified",
            "governance": "verified",
        },
        "not_applicable_reasons": {"frontend": "无独立页面"},
        "tests": {
            "required_layers": [
                "L1",
                "L2",
                "L3",
                "L4",
                "L5",
                "L8",
                "L9",
                "L10",
                "L11",
            ],
            "covered_layers": [
                "L1",
                "L2",
                "L3",
                "L4",
                "L5",
                "L8",
                "L9",
                "L10",
                "L11",
            ],
        },
    }

    issues = check_story(
        story,
        story_files={"docs/domain/user-stories-h6-state-machine.md"},
        openapi_paths=set(),
    )
    assert not any(issue.dimension == "adversarial" for issue in issues)


def test_t1_rejects_malformed_or_missing_adversarial_test_target():
    from check_quality_matrix import Issue, check_adversarial_checks

    story = {
        "id": "US-M1-011",
        "types": ["write"],
        "adversarial_checks": [
            {"id": "A9", "test": "not-a-file"},
            {"id": "A1", "test": "docs/missing.rs::no_such_fn"},
        ],
    }

    issues = check_adversarial_checks(story, require_coverage=False)
    messages = {issue.message for issue in issues}
    assert Issue("US-M1-011", "adversarial", "未知攻击类: A9") in issues
    assert any("对抗测试文件不存在" in message for message in messages)


def test_t1_accepts_adversarial_test_owned_by_integration_crate():
    from check_quality_matrix import check_adversarial_checks

    assert (
        check_adversarial_checks(
            {
                "id": "US-M1-004",
                "types": ["write"],
                "test_checks": [
                    "cargo test --manifest-path backend/Cargo.toml -p wms-api --test master_data_postgres"
                ],
                "adversarial_checks": [
                    {
                        "id": "A2",
                        "test": "backend/crates/api/tests/master_data_postgres/m1_permission_defaults.rs::m1_warehouse_reads_require_master_data_read_permission",
                    }
                ],
            },
            require_coverage=False,
        )
        == []
    )


def test_t1_rejects_adversarial_test_from_another_story():
    from check_quality_matrix import check_adversarial_checks

    issues = check_adversarial_checks(
        {
            "id": "US-H1-005",
            "types": ["write"],
            "evidence_refs": ["backend/crates/api/tests/auth_session_postgres.rs"],
            "test_checks": [
                "cargo test --manifest-path backend/Cargo.toml -p wms-api --test auth_session_postgres"
            ],
            "adversarial_checks": [
                {
                    "id": "A1",
                    "test": "backend/crates/api/tests/stock_adjustment_postgres.rs::cross_owner_cannot_read_stock_loss_order",
                }
            ],
        },
        require_coverage=False,
    )
    assert any("未进入本故事 evidence_refs 或 test_checks" in issue.message for issue in issues)


def test_t1_rejects_helper_function_as_adversarial_evidence():
    from check_quality_matrix import check_adversarial_checks

    issues = check_adversarial_checks(
        {
            "id": "US-M1-011",
            "types": ["write"],
            "adversarial_checks": [
                {
                    "id": "A1",
                    "test": "backend/crates/api/tests/support/adversarial.rs::ctx_with_permissions",
                }
            ],
        },
        require_coverage=False,
    )
    assert any("必须指向带" in issue.message for issue in issues)


def test_deferred_story_validates_filled_adversarial_checks_without_requiring_coverage():
    from check_quality_matrix import check_deferred_story

    issues = check_deferred_story(
        {
            "id": "US-M2-001",
            "types": ["write"],
            "evidence_refs": ["backend/crates/api/tests/m2_adversarial_postgres.rs"],
            "adversarial_checks": [
                {
                    "id": "A2",
                    "test": "backend/crates/api/tests/m2_adversarial_postgres.rs::inbound_write_http_requires_m2_write_permission",
                }
            ],
        }
    )
    assert not any(issue.dimension == "adversarial" for issue in issues)

    missing_file = check_deferred_story(
        {
            "id": "US-M2-001",
            "types": ["write"],
            "adversarial_checks": [
                {"id": "A2", "test": "backend/crates/api/tests/missing.rs::no_such_fn"}
            ],
        }
    )
    assert any("对抗测试文件不存在" in issue.message for issue in missing_file)


def test_complete_module_requires_declared_attack_classes():
    from check_quality_matrix import check_module_completion

    matrix = {
        "stories": [
            {
                "id": "US-H6-002",
                "title": "状态执行",
                "module": "H6",
                "types": ["write"],
            }
        ],
        "deferred_stories": [],
    }

    issues = check_module_completion(matrix, "H6")
    assert any(
        issue.story_id == "US-H6-002"
        and issue.dimension == "adversarial"
        and "缺少攻击类" in issue.message
        for issue in issues
    )


def test_complete_module_still_fails_on_deferred_stories():
    from check_quality_matrix import Issue, check_module_completion

    matrix = {
        "stories": [{"id": "US-M2-001", "module": "M2", "types": ["read_only"]}],
        "deferred_stories": [
            {"id": "US-M2-002", "title": "收货", "module": "M2"},
            {"id": "US-M3-001", "title": "库存查询", "module": "M3"},
        ],
    }

    assert (
        Issue("US-M2-002", "module_completion", "模块 M2 仍有延期故事: 收货")
        in check_module_completion(matrix, "M2")
    )
