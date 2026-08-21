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


def test_quality_matrix_runtime_guard_requires_resilience_layers():
    """运行时保护故事不能降级成只有 L2/L9 的 API 契约检查。"""
    from check_quality_matrix import derive_required_layers

    assert derive_required_layers(["api_change", "runtime_guard", "audit_compliance"]) == [
        "L1",
        "L2",
        "L4",
        "L5",
        "L7",
        "L8",
        "L9",
        "L10",
        "L11",
    ]


def test_quality_matrix_derives_existing_deferred_special_types():
    """既有延期故事的权限、离线和监控类型也必须得到确定测试层。"""
    from check_quality_matrix import derive_acceptance_level, derive_required_layers

    assert derive_required_layers(["permission"]) == ["L8"]
    assert derive_required_layers(["monitoring"]) == ["L1", "L2", "L3", "L7", "L10"]
    assert derive_acceptance_level(["offline_sync"]) == "S4"


def test_quality_matrix_derives_acceptance_level_from_story_types():
    """验收深度必须由故事风险推导，不能由执行人手工降级。"""
    from check_quality_matrix import derive_acceptance_level

    assert derive_acceptance_level(["read_only", "frontend_interaction"]) == "S1"
    assert derive_acceptance_level(["write"]) == "S2"
    assert derive_acceptance_level(["write", "inventory_change"]) == "S3"
    assert derive_acceptance_level(["integration", "external_runtime"]) == "S4"


def test_module_completion_rejects_deferred_stories():
    """模块验收时，只要仍有延期故事就不能声明模块完成。"""
    from check_quality_matrix import check_module_completion, Issue

    matrix = {
        "stories": [{"id": "US-M2-001", "module": "M2"}],
        "deferred_stories": [
            {"id": "US-M2-002", "title": "收货", "module": "M2"},
            {"id": "US-M3-001", "title": "库存查询", "module": "M3"},
        ],
    }

    assert (
        Issue("US-M2-002", "module_completion", "模块 M2 仍有延期故事: 收货")
        in check_module_completion(matrix, "M2")
    )


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


def test_quality_matrix_registers_alert_module_and_navigation_check():
    """H-AL 独立故事模块和管理端导航检查必须可进入矩阵。"""
    from check_quality_matrix import ALLOWED_MODULES, NAVIGATION_CHECK_SOURCES

    assert "AL" in ALLOWED_MODULES
    assert (
        "node apps/web-admin/self-checks/hal-alert-definition-slice-self-check.mjs"
        in NAVIGATION_CHECK_SOURCES
    )
    assert (
        "node apps/web-admin/self-checks/hal-alert-runtime-slice-self-check.mjs"
        in NAVIGATION_CHECK_SOURCES
    )


def test_quality_matrix_registers_drug_inspection_module_and_portal_contract():
    """M-DI 完成故事必须允许登记独立客户平台 OpenAPI，而不是被当作未知模块/接口。"""
    from check_quality_matrix import ALLOWED_MODULES, openapi_paths

    assert "DI" in ALLOWED_MODULES
    assert "POST /api/v1/internal/projections" in openapi_paths()
    assert "POST /api/v1/report-versions/{report_version_id}/download" in openapi_paths()


def test_quality_matrix_registers_reconciliation_module():
    from check_quality_matrix import ALLOWED_MODULES

    assert "RC" in ALLOWED_MODULES


def test_quality_matrix_accepts_h9_field_library_slice():
    """H9 字段库第一切片必须能独立进入质量矩阵。"""
    from check_quality_matrix import check_story

    story = {
        "id": "US-H9-002",
        "title": "字段库生成与字段元数据维护第一切片",
        "module": "H9",
        "types": ["read_only", "api_change", "frontend_interaction"],
        "story_file": "docs/domain/user-stories-h9-print-template.md",
        "api_paths": ["GET /api/v1/print-templates/field-libraries"],
        "dimensions": {
            "requirement": "verified",
            "fields": "verified",
            "frontend": "verified",
            "api": "verified",
            "backend": "verified",
            "database": "verified",
            "security": "verified",
            "audit": "not_applicable",
            "tests": "verified",
            "evidence": "verified",
            "docs": "verified",
            "governance": "verified",
        },
        "not_applicable_reasons": {
            "audit": "本矩阵项登记 H9 字段库列表读取切片；字段库发布审计由后续写入切片单独登记。",
        },
        "tests": {
            "required_layers": ["L1", "L2", "L3", "L7", "L8", "L9"],
            "covered_layers": ["L1", "L2", "L3", "L7", "L8", "L9"],
        },
    }

    assert check_story(
        story,
        story_files={"docs/domain/user-stories-h9-print-template.md"},
        openapi_paths={"GET /api/v1/print-templates/field-libraries"},
    ) == []


def test_quality_matrix_markdown_lists_deferred_stories():
    """展示页必须把明确延期的范围展示出来，避免把未实现故事当已完成。"""
    from check_quality_matrix import build_markdown

    markdown = build_markdown(
        {
            "stories": [],
            "deferred_stories": [
                {
                    "id": "US-H9-003",
                    "title": "模板设计与版本管理",
                    "module": "H9",
                    "types": ["write", "frontend_interaction"],
                    "reason": "后续切片实现模板设计器。",
                },
                {
                    "id": "US-H4-003",
                    "title": "企业微信审批流对接",
                    "module": "H4",
                    "reason": "尚未分类。",
                }
            ],
        }
    )

    assert "## 未完成 / 延期故事" in markdown
    assert (
        "| US-H9-003 模板设计与版本管理 | H9 | S2 | "
        "L1、L2、L3、L4、L5、L7、L8、L11 | 后续切片实现模板设计器。 |"
    ) in markdown
    assert "| US-H4-003 企业微信审批流对接 | H4 | - | - | 尚未分类。 |" in markdown


def test_quality_matrix_rejects_unknown_deferred_story_types():
    """延期故事一旦声明类型，就只能使用可推导验收要求的已知类型。"""
    from check_quality_matrix import Issue, check_deferred_story

    unknown = check_deferred_story(
        {
            "id": "US-H9-012",
            "title": "Print Agent 注册",
            "module": "H9",
            "story_file": "docs/domain/user-stories-h9-print-orchestration.md",
            "types": ["machine_magic"],
        }
    )

    assert unknown == [Issue("US-H9-012", "types", "未知故事类型: machine_magic")]


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


def test_quality_matrix_rejects_unregistered_navigation_command():
    from check_quality_matrix import Issue, check_story

    story = {
        "id": "US-M2-002",
        "title": "收货",
        "module": "M2",
        "types": ["frontend_interaction"],
        "story_file": "docs/domain/user-stories-m2-inbound-asn.md",
        "navigation_checks": ["echo fake-e2e"],
        "dimensions": {dimension: "verified" for dimension in (
            "requirement", "fields", "frontend", "api", "backend", "database",
            "security", "audit", "tests", "evidence", "docs", "governance",
        )},
        "tests": {
            "required_layers": ["L1", "L3", "L7"],
            "covered_layers": ["L1", "L3", "L7"],
        },
    }

    issues = check_story(story, story_files=set(), openapi_paths=set())

    assert Issue("US-M2-002", "evidence", "未登记的导航检查命令: echo fake-e2e") in issues


def test_verified_story_requires_module_evidence_profile(tmp_path):
    from check_quality_matrix import check_evidence_profiles

    source = tmp_path / "handler.rs"
    source.write_text("handler", encoding="utf-8")
    stories = [{"id": "US-M2-002", "module": "M2"}]

    missing = check_evidence_profiles({}, stories, repo_root=tmp_path)
    assert [issue.message for issue in missing] == ["模块 M2 缺少 evidence_profiles.M2"]

    matrix = {
        "evidence_profiles": {
            "M2": {
                "backend_files": ["handler.rs"],
                "database_objects": ["receiving_orders"],
                "test_checks": ["cargo test -p wms-api --test wave3_postgres"],
                "evidence_refs": ["handler.rs"],
            }
        }
    }
    assert check_evidence_profiles(matrix, stories, repo_root=tmp_path) == []


def test_evidence_profiles_do_not_gate_missing_runtime_screenshots(tmp_path):
    """gitignore 的 E2E 截图产物不是 T1 存在性门禁；源码证据仍要在磁盘上。"""
    from check_quality_matrix import check_evidence_profiles

    source = tmp_path / "handler.rs"
    source.write_text("handler", encoding="utf-8")
    stories = [{"id": "US-RC-001", "module": "RC"}]
    matrix = {
        "evidence_profiles": {
            "RC": {
                "backend_files": ["handler.rs"],
                "database_objects": ["reconciliation_runs"],
                "test_checks": ["cargo test -p wms-api --test reconciliation_postgres"],
                "evidence_refs": [
                    "handler.rs",
                    "artifacts/screenshot-portal/real-web/mrc-reconciliation/difference-list.png",
                    "apps/web-admin/.e2e-artifacts/m1-real/screenshots/docks-imported.png",
                ],
            }
        }
    }
    assert check_evidence_profiles(matrix, stories, repo_root=tmp_path) == []

    matrix["evidence_profiles"]["RC"]["evidence_refs"] = [
        "handler.rs",
        "missing-source.rs",
    ]
    issues = check_evidence_profiles(matrix, stories, repo_root=tmp_path)
    assert [issue.message for issue in issues] == ["证据文件不存在: missing-source.rs"]


def test_quality_matrix_real_e2e_resolves_package_script_config_and_spec(tmp_path):
    from check_quality_matrix import check_e2e_checks

    (tmp_path / "apps/web-admin").mkdir(parents=True)
    (tmp_path / "prototypes/e2e").mkdir(parents=True)
    (tmp_path / "apps/web-admin/package.json").write_text(
        '{"scripts":{"test:e2e:h5-real":"pnpm --dir ../../prototypes exec playwright test --config=playwright-web-admin-h5-real-config.ts"}}',
        encoding="utf-8",
    )
    (tmp_path / "prototypes/playwright-web-admin-h5-real-config.ts").write_text(
        'testMatch: /web-admin-h5-real\\.spec\\.ts/', encoding="utf-8"
    )
    (tmp_path / "prototypes/e2e/web-admin-h5-real.spec.ts").write_text(
        'test("real", async ({ page }) => page.goto("/"));', encoding="utf-8"
    )

    story = {
        "id": "US-H5-001",
        "frontend_pages": ["h5-express"],
        "e2e_checks": ["pnpm --dir apps/web-admin run test:e2e:h5-real"],
        "e2e_screenshots": [
            {
                "page": "h5-express",
                "spec": "prototypes/e2e/web-admin-h5-real.spec.ts",
                "screenshot": "artifacts/screenshot-portal/real-web/h5-express/carrier.png",
            }
        ],
        "evidence_refs": [
            "prototypes/e2e/web-admin-h5-real.spec.ts",
            "artifacts/screenshot-portal/real-web/h5-express/carrier.png",
        ],
    }
    assert check_e2e_checks(story, repo_root=tmp_path, verified=True) == []


def test_quality_matrix_rejects_verified_shell_dev_only_e2e(tmp_path):
    from check_quality_matrix import check_e2e_checks, Issue

    (tmp_path / "apps/web-admin").mkdir(parents=True)
    (tmp_path / "prototypes").mkdir()
    (tmp_path / "apps/web-admin/package.json").write_text(
        '{"scripts":{"test:e2e:shell-dev":"pnpm --dir ../../prototypes exec playwright test --config=playwright-web-admin-dev-config.ts"}}',
        encoding="utf-8",
    )
    (tmp_path / "prototypes/playwright-web-admin-dev-config.ts").write_text(
        'env: { WMS_WEB_ADMIN_DEV_MOCK: "1" }', encoding="utf-8"
    )

    issues = check_e2e_checks(
        {"id": "US-H1-007", "e2e_checks": ["pnpm --dir apps/web-admin run test:e2e:shell-dev"]},
        repo_root=tmp_path,
        verified=True,
    )
    assert Issue("US-H1-007", "evidence", "verified 故事不能只用 shell-dev/dev mock 作为真实 E2E 证据") in issues


def test_quality_matrix_rejects_missing_e2e_package_script(tmp_path):
    from check_quality_matrix import check_e2e_checks, Issue

    (tmp_path / "apps/web-admin").mkdir(parents=True)
    (tmp_path / "apps/web-admin/package.json").write_text('{"scripts":{}}', encoding="utf-8")
    issues = check_e2e_checks(
        {"id": "US-H5-001", "e2e_checks": ["pnpm --dir apps/web-admin run test:e2e:missing"]},
        repo_root=tmp_path,
    )
    assert Issue("US-H5-001", "evidence", "e2e_checks package script 不存在: test:e2e:missing") in issues
