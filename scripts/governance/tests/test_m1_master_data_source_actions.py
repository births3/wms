"""M1 基础档案拆分源码治理测试。"""
import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))


def test_m1_governance_scans_split_sources_without_compatibility_markers():
    import check_m1_master_data_source_actions as check

    assert check.QUERY_SOURCE.is_dir()
    assert check.DEV_MOCK_SOURCE.is_dir()
    assert not check.scan()

    for wrapper in (
        check.WEB_ADMIN / "src" / "features" / "master-data" / "master-data-queries.ts",
        check.WEB_ADMIN / "vite.config.ts",
    ):
        source = wrapper.read_text(encoding="utf-8")
        assert "compatibility markers" not in source
        assert "governance compatibility" not in source


def test_m1_split_source_tokens_are_bound_to_owning_files():
    """拆分目录中的行为证据不能由无关兄弟文件代为满足。"""
    import check_m1_master_data_source_actions as check

    split_specs = [
        spec
        for spec in check.TOKEN_SPECS
        if spec.path == check.QUERY_SOURCE or spec.path == check.DEV_MOCK_SOURCE
    ]

    assert not split_specs


def test_m1_governance_requires_erp_authoritative_product_write_guard():
    import check_m1_master_data_source_actions as check

    handler_tokens = {
        spec.token
        for spec in check.TOKEN_SPECS
        if spec.path == check.HANDLERS_RS
    }

    assert "state.create_product" not in handler_tokens
    assert "pub(super) fn require_internal_product_write" in handler_tokens
    assert "商品主数据只能由 ERP 通过 H8 商品主数据防腐层同步" in handler_tokens


def test_m1_governance_recursively_reports_missing_split_source_token(tmp_path, monkeypatch):
    import check_m1_master_data_source_actions as check

    source = tmp_path / "split-source"
    nested = source / "nested"
    nested.mkdir(parents=True)
    (nested / "queries.ts").write_text("export const present = true;\n", encoding="utf-8")
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(
        check,
        "TOKEN_SPECS",
        (check.TokenSpec(source, "required_token", "拆分源码缺少必需 token"),),
    )

    issues = check.scan()

    assert [(issue.file, issue.message) for issue in issues] == [
        ("split-source", "拆分源码缺少必需 token")
    ]


def test_m1_governance_reads_rust_source_family(tmp_path):
    import check_m1_master_data_source_actions as check

    source = tmp_path / "master_data_handlers.rs"
    source.write_text('include!("master_data_handlers_part2.rs");', encoding="utf-8")
    (tmp_path / "master_data_handlers_part2.rs").write_text(
        "state.create_product state.create_supplier state.create_customer",
        encoding="utf-8",
    )

    text = check.read_source(source)

    assert "state.create_product" in text
    assert "state.create_supplier" in text
    assert "state.create_customer" in text


def test_m1_dev_mock_changes_trigger_source_action_check():
    """拆分后的 dev mock 变更必须触发 M1 来源动作治理。"""
    from _diff import load_gate_rules, match_rules

    triggered = match_rules(
        ["apps/web-admin/dev-mocks/web-admin-dev-mock-core.ts"],
        load_gate_rules(),
    )

    assert "check_m1_master_data_source_actions" in triggered
