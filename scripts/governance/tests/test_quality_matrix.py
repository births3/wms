"""全链路质量矩阵治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_quality_matrix_derives_test_layers_from_story_types():
    """故事类型自动推导 S2 测试层。"""
    from check_quality_matrix import derive_required_layers

    assert derive_required_layers(["write", "inventory_change", "frontend_interaction"]) == [
        "L1",
        "L2",
        "L3",
        "L4",
        "L5",
        "L7",
        "L8",
        "L10",
        "L11",
    ]


def test_quality_matrix_rejects_non_strict_dimension_status():
    """矩阵维度状态只能是 verified 或 not_applicable。"""
    from check_quality_matrix import check_story, Issue

    story = {
        "id": "US-M2-002",
        "title": "收货",
        "module": "M2",
        "types": ["write"],
        "requirement": {"status": "verified", "story_file": "docs/domain/user-stories-m2-inbound-asn.md"},
        "fields": {"status": "partial"},
        "frontend": {"status": "verified"},
        "api": {"status": "verified"},
        "backend": {"status": "verified"},
        "database": {"status": "verified"},
        "security": {"status": "verified"},
        "audit": {"status": "verified"},
        "tests": {"status": "verified", "required_layers": ["L1", "L2", "L3", "L4", "L5", "L8", "L11"], "covered_layers": ["L1", "L2", "L3", "L4", "L5", "L8", "L11"]},
        "evidence": {"status": "verified"},
        "docs": {"status": "verified"},
        "governance": {"status": "verified"},
    }

    assert check_story(story, story_files=set(), openapi_paths=set()) == [
        Issue("US-M2-002", "fields", "状态 partial 不允许；只能是 verified 或 not_applicable"),
    ]


def test_quality_matrix_accepts_registered_horizontal_modules():
    """H1/H3/H9 等横向能力进入执行后必须能纳入质量矩阵。"""
    from check_quality_matrix import check_story

    story = {
        "id": "US-H9-001",
        "title": "打印模板类型字典",
        "module": "H9",
        "types": ["read_only", "config_rule", "frontend_interaction"],
        "story_file": "docs/domain/user-stories-h9-print-template.md",
        "api_paths": ["GET /api/v1/system-dictionaries/print_template_type/items"],
        "dimensions": {
            "requirement": "verified",
            "fields": "verified",
            "frontend": "verified",
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
        "tests": {
            "required_layers": ["L1", "L2", "L3", "L4", "L7", "L8", "L9"],
            "covered_layers": ["L1", "L2", "L3", "L4", "L7", "L8", "L9"],
        },
    }

    assert check_story(
        story,
        story_files={"docs/domain/user-stories-h9-print-template.md"},
        openapi_paths={"GET /api/v1/system-dictionaries/print_template_type/items"},
    ) == []


def test_quality_matrix_rejects_module_mismatch_and_missing_openapi_method():
    """故事 ID 模块和 OpenAPI method 必须都对齐。"""
    from check_quality_matrix import Issue, check_story

    story = {
        "id": "US-M2-002",
        "title": "收货",
        "module": "M1",
        "types": ["read_only"],
        "story_file": "docs/domain/user-stories-m2-inbound-asn.md",
        "api_paths": ["POST /api/v1/inbound/receiving-orders/{id}/receive"],
        "dimensions": {
            "requirement": "verified",
            "fields": "verified",
            "frontend": "verified",
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
        "tests": {
            "required_layers": ["L1", "L2", "L3", "L8"],
            "covered_layers": ["L1", "L2", "L3", "L8"],
        },
    }

    assert check_story(
        story,
        story_files=set(),
        openapi_paths={"GET /api/v1/inbound/receiving-orders/{id}/receive"},
    ) == [
        Issue("US-M2-002", "requirement", "story id 模块 M2 与 module M1 不一致"),
        Issue("US-M2-002", "api", "OpenAPI 缺少 operation: POST /api/v1/inbound/receiving-orders/{id}/receive"),
    ]
