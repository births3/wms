"""Wave 6 evidence preflight just 入口执行脚本绑定测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import write_single_gate_preflight_fixture


def test_wave6_evidence_preflight_rejects_gate_just_entry_calling_wrong_script(
    tmp_path,
    monkeypatch,
):
    """gate record / validate just 入口必须调用 preflight 推导出的执行脚本。"""
    import check_wave6_evidence_preflight as check

    gate = check.GateSpec(
        "W6.G",
        "Wave 5 M10 TMS+ evidence",
        "docs/runbooks/wave-5-tms-evidence.md",
        "docs/retros/wave-5-tms-evidence.json",
        ("wave-5-tms-evidence-record", "wave-5-tms-evidence-validate"),
    )
    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        gate=gate,
        runbook_lines=[
            "```bash",
            "just wave-5-tms-evidence-record",
            "just wave-5-tms-evidence-validate",
            "```",
        ],
        just_text=(
            "wave-6-evidence-preflight:\n"
            "    @python3 scripts/governance/check_wave6_evidence_preflight.py\n"
            "wave-6-status:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py\n"
            "wave-6-evidence-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict --evidence-only\n"
            "wave-6-missing-evidence-commands:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --commands-only --strict --evidence-only\n"
            "wave-6-complete-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
            "wave-5-tms-evidence-record *args:\n"
            "    @python3 scripts/governance/record_wave5_hardware_evidence.py {{args}}\n"
            "wave-5-tms-evidence-validate:\n"
            "    @python3 scripts/governance/validate_wave5_tms_evidence.py\n"
        ),
        execution_files=(
            "scripts/governance/record_wave5_tms_evidence.py",
            "scripts/governance/validate_wave5_tms_evidence.py",
        ),
    )
    (tmp_path / "scripts/governance/record_wave5_tms_evidence.py").write_text(
        "\n".join([
            "parser.add_argument('--force', action='store_true')",
            "if output_path.exists() and not args.force:",
            "    raise SystemExit('already exists; pass --force to overwrite')",
            "output_path.write_text('{}')",
        ]),
        encoding="utf-8",
    )

    top_errors, gate_results = check.collect_results()

    assert top_errors == []
    joined_gate_errors = " ".join(
        error for result in gate_results for error in result.errors
    )
    assert "wave-5-tms-evidence-record" in joined_gate_errors
    assert "record_wave5_tms_evidence.py" in joined_gate_errors


def test_wave6_evidence_preflight_wave2_collector_just_entries_have_expected_modes():
    """W6.C readiness 必须只读，smoke 必须执行真实采集。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")
    collector_file = "scripts/governance/collect_wave2_runtime_evidence.py"

    readiness_commands = check.just_recipe_commands(
        just_text,
        "wave-2-runtime-evidence-readiness",
    )
    smoke_commands = check.just_recipe_commands(
        just_text,
        "wave-2-runtime-evidence-smoke",
    )
    assert readiness_commands == [
        ["python3", collector_file, "--check-only", "{{args}}"],
    ]
    assert smoke_commands == [["python3", collector_file, "{{args}}"]]


def test_wave6_evidence_preflight_wave3_pda_readiness_is_read_only_checker():
    """W6.D readiness 必须调用只读 checker，不能写 PDA runtime evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-runtime-readiness",
    ) == [[
        "python3",
        "scripts/governance/check_wave3_pda_runtime_readiness.py",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_service_precheck_is_service_only():
    """W6.D 无 PDA 服务前置入口必须固定使用 service-precheck-only。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-service-precheck",
    ) == [[
        "python3",
        "scripts/governance/check_wave3_pda_runtime_readiness.py",
        "--service-precheck-only",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_trace_code_openapi_precheck_is_read_only():
    """W6.D 追溯码 OpenAPI 预检入口必须调用只读 checker，不写 evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-trace-code-openapi-precheck",
    ) == [[
        "python3",
        "scripts/governance/check_wave3_pda_runtime_readiness.py",
        "--trace-code-openapi-precheck",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_materials_checklist_is_read_only():
    """W6.D 现场材料清单入口必须固定使用 materials-checklist，不写 evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-materials-checklist",
    ) == [[
        "python3",
        "scripts/governance/check_wave3_pda_runtime_readiness.py",
        "--materials-checklist",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_field_work_request_is_read_only():
    """W6.D 现场资源申请包入口必须固定使用 field-work-request，不写 evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-field-work-request",
    ) == [[
        "python3",
        "scripts/governance/check_wave3_pda_runtime_readiness.py",
        "--field-work-request",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_field_execution_summary_is_read_only():
    """W6.D 现场执行摘要入口必须固定使用 field-execution-summary，不写 evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-field-execution-summary",
    ) == [[
        "python3",
        "scripts/governance/check_wave3_pda_runtime_readiness.py",
        "--field-execution-summary",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_field_owner_gap_actions_is_read_only():
    """W6.D owner 缺口动作单入口必须固定使用 field-owner-gap-actions，不写 evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert "# Wave 3 现场 owner 缺口动作单" in just_text
    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-field-owner-gap-actions",
    ) == [[
        "python3",
        "scripts/governance/check_wave3_pda_runtime_readiness.py",
        "--field-owner-gap-actions",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_package_template_is_read_only():
    """W6.D 证据包模板入口必须只导出模板，不写 PDA runtime evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert "# Wave 3 现场证据包 Markdown/JSON 模板" in just_text
    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-evidence-package-template",
    ) == [[
        "python3",
        "scripts/governance/record_wave3_pda_runtime_evidence.py",
        "--export-package-template",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave3_pda_intake_entries_use_shared_file_path():
    """W6.D intake 校验和正式记录入口必须复用同一份现场 intake 文件。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert "# Wave 3 现场 JSON intake 模板" in just_text
    assert "# Wave 3 现场 JSON intake 只读校验" in just_text
    assert "# Wave 3 现场 JSON intake 正式记录" in just_text
    assert check.just_recipe_commands(
        just_text,
        "wave-3-pda-intake-template",
    ) == [[
        "python3",
        "scripts/governance/record_wave3_pda_runtime_evidence.py",
        "--export-intake-template",
        "{{args}}",
    ]]
    intake_check_commands = check.just_recipe_commands(
        just_text,
        "wave-3-pda-intake-check",
    )
    assert any(
        "WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE" in token
        for token in intake_check_commands[0]
    )
    assert intake_check_commands[1] == [
        "python3",
        "scripts/governance/record_wave3_pda_runtime_evidence.py",
        "--from-intake-file",
        '"$WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE"',
        "--check-only",
        "{{args}}",
    ]
    intake_record_commands = check.just_recipe_commands(
        just_text,
        "wave-3-pda-intake-record",
    )
    assert any(
        "WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE" in token
        for token in intake_record_commands[0]
    )
    assert intake_record_commands[1] == [
        "python3",
        "scripts/governance/record_wave3_pda_runtime_evidence.py",
        "--from-intake-file",
        '"$WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE"',
        "{{args}}",
    ]


def test_wave6_evidence_preflight_wave4_external_readiness_is_read_only_checker():
    """W6.E readiness 必须调用只读 checker，不能写外部依赖 evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-4-external-dependencies-readiness",
    ) == [[
        "python3",
        "scripts/governance/check_wave4_external_dependencies_readiness.py",
        "{{args}}",
    ]]


def test_wave6_evidence_preflight_wave5_hardware_materials_chain_is_read_only():
    """W6.F materials/readiness 必须调用 --check-only，不写硬件 evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")
    expected = [
        "python3",
        "scripts/governance/record_wave5_hardware_evidence.py",
        "--check-only",
        "{{args}}",
    ]

    assert check.just_recipe_commands(
        just_text,
        "wave-5-hardware-materials",
    ) == [expected]
    assert check.just_recipe_commands(
        just_text,
        "wave-5-hardware-readiness",
    ) == [expected]


def test_wave6_evidence_preflight_wave5_tms_materials_chain_is_read_only():
    """W6.G materials/readiness 必须调用 --check-only，不写 TMS evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")
    expected = [
        "python3",
        "scripts/governance/record_wave5_tms_evidence.py",
        "--check-only",
        "{{args}}",
    ]

    assert check.just_recipe_commands(
        just_text,
        "wave-5-tms-materials",
    ) == [expected]
    assert check.just_recipe_commands(
        just_text,
        "wave-5-tms-readiness",
    ) == [expected]


def test_wave6_evidence_preflight_wave6_deploy_readiness_chain_is_read_only():
    """W6.H materials/readiness 必须调用只读脚本，不能写 deploy evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-6-deploy-materials",
    ) == [[
        "python3",
        "scripts/governance/report_wave6_deploy_materials.py",
        "{{args}}",
    ]]
    assert check.just_recipe_commands(
        just_text,
        "wave-6-deploy-readiness",
    ) == [[
        "python3",
        "scripts/governance/check_wave6_deploy_readiness.py",
        "{{args}}",
    ]]


def test_wave6_deploy_materials_just_entry_supports_export_template():
    """W6.H materials just 入口必须透传参数以支持非密钥 export 模板。"""
    just_text = Path("justfile").read_text(encoding="utf-8")

    assert "wave-6-deploy-materials *args:" in just_text
    assert "report_wave6_deploy_materials.py {{args}}" in just_text
