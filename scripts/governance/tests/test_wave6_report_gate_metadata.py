"""Wave 6 report gate metadata 派生与一致性测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import (
    assert_wave6_missing_evidence_files_are_consistent,
    patch_wave6_report_io,
    wave6_missing_file_validator,
)


def test_wave6_report_gate_metadata_is_derived_from_preflight_gates():
    """report 的 Gate 元数据必须从 preflight GateSpec 派生，避免双份清单漂移。"""
    from check_wave6_evidence_preflight import validation_commands

    import report_wave6_pre_release as report

    assert report.derive_wave6_gate_ids(report.PREFLIGHT_GATES) == [
        gate.gate_id for gate in report.PREFLIGHT_GATES
    ]
    assert report.derive_wave6_evidence_files(report.PREFLIGHT_GATES) == [
        gate.evidence_file for gate in report.PREFLIGHT_GATES
    ]
    assert report.WAVE6_GATE_IDS == report.derive_wave6_gate_ids(report.PREFLIGHT_GATES)
    assert report.WAVE6_EVIDENCE_FILES == report.derive_wave6_evidence_files(
        report.PREFLIGHT_GATES
    )
    assert len(report.WAVE6_GATE_IDS) == len(report.WAVE6_EVIDENCE_FILES) == len(
        report.PREFLIGHT_GATES
    )
    assert report.evidence_file_by_gate_id() == {
        gate.gate_id: gate.evidence_file for gate in report.PREFLIGHT_GATES
    }
    assert report.WAVE6_VALIDATION_COMMANDS == report.derive_wave6_validation_commands()
    assert set(validation_commands()) <= set(report.WAVE6_VALIDATION_COMMANDS)
    assert [
        command
        for command in report.WAVE6_VALIDATION_COMMANDS
        if command not in validation_commands()
    ] == [
        "just wave-6-evidence-preflight",
        "just wave-6-evidence-check",
        "just wave-6-status",
        "just gov-t1",
        "just task-check",
        "git diff --check",
    ]


def test_wave6_pre_release_evidence_boundary_stays_w6_a_to_w6_h():
    """Wave 6 预发布 evidence 边界必须保持 W6.A-H 八个真实 JSON 文件。"""
    import report_wave6_pre_release as report

    assert report.WAVE6_GATE_IDS == [
        "W6.A",
        "W6.B",
        "W6.C",
        "W6.D",
        "W6.E",
        "W6.F",
        "W6.G",
        "W6.H",
    ]
    assert report.WAVE6_EVIDENCE_FILES == [
        "docs/retros/wave-1-h2-runtime-evidence.json",
        "docs/retros/wave-1-runtime-evidence.json",
        "docs/retros/wave-2-runtime-evidence.json",
        "docs/retros/wave-3-pda-runtime-evidence.json",
        "docs/retros/wave-4-external-dependencies.json",
        "docs/retros/wave-5-hardware-evidence.json",
        "docs/retros/wave-5-tms-evidence.json",
        "docs/retros/wave-6-deploy-evidence.json",
    ]


def test_wave6_missing_evidence_consistency_assertion_uses_gate_file_mapping(
    monkeypatch,
    capsys,
):
    """missing evidence 一致性断言必须按 gate->file 映射核对，而不是只 zip 顺序。"""
    import pytest

    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave3_pda_runtime_evidence.py": (
                "docs/retros/wave-3-pda-runtime-evidence.json"
            ),
            "validate_wave4_external_dependencies.py": (
                "docs/retros/wave-4-external-dependencies.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    swapped_files = list(reversed(payload["missing_evidence_files"]))
    payload["missing_evidence_files"] = swapped_files
    for detail, swapped_file in zip(payload["missing_evidence_details"], swapped_files):
        detail["evidence_file"] = swapped_file

    with pytest.raises(AssertionError):
        assert_wave6_missing_evidence_files_are_consistent(payload)


def test_wave6_report_evidence_file_map_does_not_zip_mutable_report_lists(
    monkeypatch,
):
    """gate -> evidence 映射必须使用 preflight GateSpec，不能被 report 列表错配截断。"""
    import report_wave6_pre_release as report

    monkeypatch.setattr(report, "WAVE6_GATE_IDS", ["W6.A"])
    monkeypatch.setattr(report, "WAVE6_EVIDENCE_FILES", [])

    assert report.evidence_file_by_gate_id() == {
        gate.gate_id: gate.evidence_file for gate in report.PREFLIGHT_GATES
    }
