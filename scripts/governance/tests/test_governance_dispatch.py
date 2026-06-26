"""治理入口与 gate 规则一致性测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_governance_consistency_doc_parser():
    """check_governance_consistency 能正确解析 §4.6 表格。"""
    from check_governance_consistency import parse_doc_section
    scripts = parse_doc_section()
    # Wave 1+ 必需脚本
    assert "check_layer_dependency" in scripts
    assert scripts["check_layer_dependency"] == "T2"
    assert "check_unsafe_and_unwrap" in scripts
    # CI 全量的脚本不应被纳入（如 perf_baseline）
    assert "check_perf_baseline" not in scripts


def test_governance_consistency_gate_rules_parser():
    """check_governance_consistency 能正确解析 gate-rules.toml 占位规则。"""
    from check_governance_consistency import parse_gate_rules
    scripts = parse_gate_rules()
    # 已实现的脚本不应出现（如 check_doc_links）
    assert "check_doc_links" not in scripts
    # 占位脚本应出现
    assert "check_layer_dependency" in scripts


def test_governance_consistency_compares_declared_gate_patterns():
    """§4.6 声明的 gate-rules 模式必须被 gate-rules.toml 覆盖。"""
    from check_governance_consistency import parse_doc_rule_specs, parse_gate_rule_specs

    doc_specs = parse_doc_rule_specs()
    gate_specs = parse_gate_rule_specs()

    assert "*.md" in doc_specs["check_changelog_freshness"].patterns
    assert "*.md" in gate_specs["check_changelog_freshness"].patterns
    assert "docs/domain/user-stories-*.md" in gate_specs["check_field_coding_standards"].patterns
    assert "docs/domain/user-stories-*.md" in gate_specs["check_business_rules_registry"].patterns


def test_governance_checks_t2_includes_openapi_full_entrypoint():
    """T2 全量入口必须覆盖 OpenAPI 同步链路，不能只依赖 diff gate。"""
    from governance_checks import expand_tier_scripts

    scripts = expand_tier_scripts("T2")
    assert "check_openapi_in_sync.py" in scripts
    assert "validate_openapi_artifacts.py" in scripts
    assert "check_openapi_contract.py" in scripts


def test_governance_checks_t1_includes_changelog_freshness():
    """T1 全量入口必须覆盖已实现的 CHANGELOG 同步治理。"""
    from governance_checks import expand_tier_scripts

    assert "check_changelog_freshness.py" in expand_tier_scripts("T1")


def test_governance_checks_t1_includes_mkdocs_nav_consistency():
    """T1 全量入口必须覆盖 MkDocs 导航防漂移治理。"""
    from governance_checks import expand_tier_scripts

    assert "check_mkdocs_nav_consistency.py" in expand_tier_scripts("T1")


def test_governance_checks_t1_includes_governance_coverage_meta_check():
    """T1 全量入口必须覆盖治理脚本接线元检查。"""
    from governance_checks import expand_tier_scripts

    assert "check_governance_coverage.py" in expand_tier_scripts("T1")


def test_governance_script_changes_trigger_coverage_meta_check():
    """治理脚本变更时，diff gate 必须触发覆盖元检查。"""
    from _diff import load_gate_rules, match_rules

    triggered = match_rules(
        ["scripts/governance/new_check.py"],
        load_gate_rules(),
    )

    assert "check_governance_coverage" in triggered


def test_governance_routing_sources_trigger_coverage_meta_check():
    """coverage 依赖的路由源变更时，diff gate 必须触发覆盖元检查。"""
    from _diff import load_gate_rules, match_rules

    for changed_file in ["governance/gate-rules.toml", "justfile"]:
        triggered = match_rules([changed_file], load_gate_rules())
        assert "check_governance_coverage" in triggered


def test_runtime_evidence_json_changes_trigger_validators():
    """8 个真实 evidence JSON 变更必须触发对应验证入口。"""
    from _diff import load_gate_rules, match_rules

    expected = {
        "docs/retros/wave-1-h2-runtime-evidence.json": "validate_wave1_runtime_evidence",
        "docs/retros/wave-1-runtime-evidence.json": "validate_wave1_runtime_evidence",
        "docs/retros/wave-2-runtime-evidence.json": "report_wave2_completion",
        "docs/retros/wave-3-pda-runtime-evidence.json": "validate_wave3_pda_runtime_evidence",
        "docs/retros/wave-4-external-dependencies.json": "validate_wave4_external_dependencies",
        "docs/retros/wave-5-hardware-evidence.json": "validate_wave5_hardware_evidence",
        "docs/retros/wave-5-tms-evidence.json": "validate_wave5_tms_evidence",
        "docs/retros/wave-6-deploy-evidence.json": "validate_wave6_deploy_evidence",
    }

    rules = load_gate_rules()
    for changed_file, check_name in expected.items():
        triggered = match_rules([changed_file], rules)
        assert check_name in triggered

    wave3_pda_checks = match_rules(
        ["docs/retros/wave-3-pda-runtime-evidence.json"],
        rules,
    )
    assert "check_pda_production_gate" in wave3_pda_checks


def test_static_governance_paths_do_not_trigger_runtime_evidence_validators():
    """静态治理变更不能裸跑缺真实 evidence 会失败的 validator。"""
    from _diff import load_gate_rules, match_rules

    runtime_validators = {
        "validate_wave1_runtime_evidence",
        "report_wave2_completion",
        "validate_wave3_pda_runtime_evidence",
        "validate_wave4_external_dependencies",
        "validate_wave5_hardware_evidence",
        "validate_wave5_tms_evidence",
        "validate_wave6_deploy_evidence",
    }
    static_paths = [
        "governance/gate-rules.toml",
        "justfile",
        "scripts/governance/check_wave6_evidence_preflight.py",
        "scripts/governance/record_wave6_deploy_evidence.py",
    ]

    rules = load_gate_rules()
    for changed_file in static_paths:
        triggered = set(match_rules([changed_file], rules))
        assert runtime_validators.isdisjoint(triggered)


def test_runtime_evidence_validators_are_only_attached_to_evidence_json_rules():
    """缺真实 evidence 会失败的 validator 只能挂到 evidence JSON 精确路径。"""
    from _diff import load_gate_rules

    runtime_validators = {
        "validate_wave1_runtime_evidence",
        "report_wave2_completion",
        "validate_wave3_pda_runtime_evidence",
        "validate_wave4_external_dependencies",
        "validate_wave5_hardware_evidence",
        "validate_wave5_tms_evidence",
        "validate_wave6_deploy_evidence",
    }

    bad_rules = []
    for rule in load_gate_rules():
        checks = set(rule.checks)
        if runtime_validators.isdisjoint(checks):
            continue
        if not rule.match.startswith("docs/retros/") or not rule.match.endswith(".json"):
            bad_rules.append((rule.match, sorted(checks & runtime_validators)))

    assert bad_rules == []


def test_wave6_evidence_execution_file_changes_trigger_preflight():
    """Wave 6 evidence 执行脚本变更必须触发 preflight 链路检查。"""
    from _diff import load_gate_rules, match_rules
    import check_wave6_evidence_preflight as preflight

    rules = load_gate_rules()
    for changed_file in preflight.REQUIRED_EXECUTION_FILES:
        triggered = match_rules([changed_file], rules)
        assert "check_wave6_evidence_preflight" in triggered


def test_governance_coverage_has_no_orphan_or_unsmoked_scripts():
    """所有治理脚本都必须可达，并纳入 smoke 或记录明确豁免。"""
    import check_governance_coverage as coverage

    assert coverage.main(["--json"]) == 0


def test_governance_coverage_includes_report_script_used_as_evidence_gate():
    """作为 evidence gate validator 的 report_* 脚本也必须纳入覆盖统计。"""
    import check_governance_coverage as coverage

    script = "report_wave2_completion.py"

    assert script in coverage.discover_scripts()
    assert script in coverage.gate_referenced()
    assert script in coverage.smoke_listed()


def test_governance_coverage_text_mentions_gate_report_scripts(capsys):
    """coverage 文本输出必须说明 gate report_* 脚本也在统计范围内。"""
    import check_governance_coverage as coverage

    assert coverage.main([]) == 0
    output = capsys.readouterr().out
    assert "check_*/validate_* / gate report_*" in output


def test_governance_coverage_only_counts_parameterized_justfile_scripts(
    tmp_path,
    monkeypatch,
):
    """justfile 可达性只允许必须带运行参数的脚本白名单。"""
    import check_governance_coverage as coverage

    justfile = tmp_path / "justfile"
    justfile.write_text(
        "\n".join([
            "wave-1-runtime-prereq-h2:",
            "    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode h2",
            "wave-1-h2-runtime-readiness:",
            "    @python3 scripts/governance/check_wave1_h2_runtime_readiness.py --database-url \"$URL\"",
            "wave-3-pda-runtime-readiness:",
            "    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py",
            "wave-4-external-dependencies-readiness:",
            "    @python3 scripts/governance/check_wave4_external_dependencies_readiness.py",
            "wave-3-pda-runtime-evidence-validate:",
            "    @python3 scripts/governance/validate_wave3_pda_runtime_evidence.py",
            "wave-6-deploy-evidence-validate:",
            "    @python3 scripts/governance/validate_wave6_deploy_evidence.py",
        ]),
        encoding="utf-8",
    )
    monkeypatch.setattr(coverage, "JUSTFILE", justfile)

    assert coverage.justfile_referenced() == {
        "check_wave1_runtime_evidence_prereqs.py",
        "check_wave1_h2_runtime_readiness.py",
        "check_wave3_pda_runtime_readiness.py",
        "check_wave4_external_dependencies_readiness.py",
    }
