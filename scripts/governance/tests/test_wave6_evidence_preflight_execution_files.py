"""Wave 6 evidence preflight 执行文件覆盖与写入保护测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

def test_wave6_evidence_preflight_requires_closeout_report_execution_file():
    """Wave 6 closeout 包装入口背后的 report 脚本也必须纳入 preflight。"""
    import check_wave6_evidence_preflight as check

    assert "scripts/governance/report_wave6_pre_release.py" in check.REQUIRED_EXECUTION_FILES


def test_wave6_evidence_preflight_detects_missing_execution_file(tmp_path, monkeypatch):
    """Wave 6 preflight 必须覆盖 just 入口背后的实际执行文件。"""
    import check_wave6_evidence_preflight as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "GATES", ())
    monkeypatch.setattr(
        check,
        "REQUIRED_EXECUTION_FILES",
        ("scripts/governance/existing.py", "deploy/scripts/missing_probe.sh"),
    )

    files = {
        check.PREFLIGHT_DOC: "\n".join([
            "just wave-6-evidence-preflight",
            "不会写入 runtime evidence",
            "不能关闭 gate",
            "environment",
            "dev",
            "staging",
            "local",
            "prod",
            "production",
            "mock",
            "fake",
            "stub",
            "example",
        ]),
        check.CLOSEOUT_DOC: (
            f"just wave-6-evidence-preflight\n{check.PREFLIGHT_DOC}\n"
            "## 当前 Gate\nW6.X docs/retros/wave-x-evidence.json wave-x-record wave-x-validate\n"
        ),
        check.TODO_DOC: "W6 evidence preflight",
        check.JUSTFILE: "wave-6-evidence-preflight:\n",
        "docs/adr/0035-wave-6-pre-release-evidence-closeout.md": "ADR-0035",
        "scripts/governance/existing.py": "",
    }
    for rel_path, text in files.items():
        path = tmp_path / rel_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    assert "deploy/scripts/missing_probe.sh" in " ".join(top_errors)


def test_wave6_evidence_preflight_requires_gate_record_and_validate_execution_files():
    """每个 Wave 6 evidence gate 的 record / validate 脚本必须纳入 preflight。"""
    import check_wave6_evidence_preflight as check

    expected = {
        "W6.A": {
            "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
            "scripts/governance/check_wave1_h2_runtime_readiness.py",
            "scripts/governance/collect_wave1_h2_runtime_evidence.py",
            "scripts/governance/validate_wave1_runtime_evidence.py",
        },
        "W6.B": {
            "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
            "deploy/scripts/wave1_auto_rollback_probe.sh",
            "scripts/governance/validate_wave1_runtime_evidence.py",
        },
        "W6.C": {
            "scripts/governance/collect_wave2_runtime_evidence.py",
            "scripts/governance/record_wave2_runtime_evidence.py",
            "scripts/governance/report_wave2_completion.py",
        },
        "W6.D": {
            "scripts/governance/check_wave3_pda_runtime_readiness.py",
            "scripts/governance/record_wave3_pda_runtime_evidence.py",
            "scripts/governance/validate_wave3_pda_runtime_evidence.py",
        },
        "W6.E": {
            "scripts/governance/check_wave4_external_dependencies_readiness.py",
            "scripts/governance/record_wave4_external_dependencies.py",
            "scripts/governance/validate_wave4_external_dependencies.py",
        },
        "W6.F": {
            "scripts/governance/record_wave5_hardware_evidence.py",
            "scripts/governance/validate_wave5_hardware_evidence.py",
        },
        "W6.G": {
            "scripts/governance/record_wave5_tms_evidence.py",
            "scripts/governance/validate_wave5_tms_evidence.py",
        },
        "W6.H": {
            "scripts/governance/report_wave6_deploy_materials.py",
            "scripts/governance/check_wave6_deploy_readiness.py",
            "scripts/governance/record_wave6_deploy_evidence.py",
            "scripts/governance/validate_wave6_deploy_evidence.py",
        },
    }

    required_files = set(check.REQUIRED_EXECUTION_FILES)

    for gate in check.GATES:
        execution_files = check.gate_execution_files(gate)
        assert execution_files == list(dict.fromkeys(execution_files))
        assert set(execution_files) <= required_files
        if gate.gate_id in expected:
            assert expected[gate.gate_id] <= set(execution_files)


def test_wave6_evidence_preflight_maps_wave2_collector_entries_to_collector():
    """W6.C readiness/smoke/collect 都应绑定到真实 collector。"""
    import check_wave6_evidence_preflight as check

    expected = "scripts/governance/collect_wave2_runtime_evidence.py"

    assert check.execution_file_for_just_entry("wave-2-runtime-evidence-readiness") == expected
    assert check.execution_file_for_just_entry("wave-2-runtime-evidence-smoke") == expected
    assert check.execution_file_for_just_entry("wave-2-runtime-evidence-collect") == expected


def test_wave6_evidence_preflight_maps_wave3_pda_readiness_to_checker():
    """W6.D PDA readiness 应绑定到只读 checker，而不是 record 脚本。"""
    import check_wave6_evidence_preflight as check

    for entry in (
        "wave-3-pda-materials-checklist",
        "wave-3-pda-field-work-request",
        "wave-3-pda-service-precheck",
        "wave-3-pda-runtime-readiness",
    ):
        assert check.execution_file_for_just_entry(entry) == (
            "scripts/governance/check_wave3_pda_runtime_readiness.py"
        )

    assert check.execution_file_for_just_entry("wave-3-pda-runtime-readiness") == (
        "scripts/governance/check_wave3_pda_runtime_readiness.py"
    )
    assert check.execution_file_for_just_entry("wave-3-pda-evidence-package-template") == (
        "scripts/governance/record_wave3_pda_runtime_evidence.py"
    )


def test_wave6_evidence_preflight_maps_wave4_external_readiness_to_checker():
    """W6.E 外部依赖 readiness 应绑定到只读 checker，而不是 record 脚本。"""
    import check_wave6_evidence_preflight as check

    assert check.execution_file_for_just_entry(
        "wave-4-external-dependencies-readiness",
    ) == "scripts/governance/check_wave4_external_dependencies_readiness.py"


def test_wave6_evidence_preflight_maps_wave5_hardware_readiness_to_recorder_check_only():
    """W6.F materials/readiness 应复用硬件 recorder 的 --check-only 模式。"""
    import check_wave6_evidence_preflight as check

    expected = "scripts/governance/record_wave5_hardware_evidence.py"

    assert check.execution_file_for_just_entry("wave-5-hardware-materials") == expected
    assert check.execution_file_for_just_entry("wave-5-hardware-readiness") == expected


def test_wave6_evidence_preflight_maps_wave5_tms_readiness_to_recorder_check_only():
    """W6.G materials/readiness 应复用 TMS recorder 的 --check-only 模式。"""
    import check_wave6_evidence_preflight as check

    expected = "scripts/governance/record_wave5_tms_evidence.py"

    assert check.execution_file_for_just_entry("wave-5-tms-materials") == expected
    assert check.execution_file_for_just_entry("wave-5-tms-readiness") == expected


def test_wave6_evidence_preflight_maps_wave6_deploy_readiness_to_checker():
    """W6.H deploy readiness 应绑定到只读 checker，而不是 record 脚本。"""
    import check_wave6_evidence_preflight as check

    assert check.execution_file_for_just_entry("wave-6-deploy-readiness") == (
        "scripts/governance/check_wave6_deploy_readiness.py"
    )


def test_wave6_evidence_preflight_maps_wave6_deploy_materials_to_reporter():
    """W6.H deploy materials 应绑定到只读 reporter，而不是 record 脚本。"""
    import check_wave6_evidence_preflight as check

    assert check.execution_file_for_just_entry("wave-6-deploy-materials") == (
        "scripts/governance/report_wave6_deploy_materials.py"
    )
