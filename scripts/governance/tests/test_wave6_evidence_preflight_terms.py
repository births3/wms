"""Wave 6 evidence preflight 通用术语覆盖测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import (
    COMMON_TERMS,
    PLACEHOLDER_TERMS,
    write_single_gate_preflight_fixture,
)


def test_wave6_evidence_preflight_detects_missing_forbidden_boundary_term(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 必须发现 runbook 漏掉禁用证据边界说明。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(tmp_path, check, monkeypatch)
    runbook = tmp_path / "docs/runbooks/wave-x-evidence.md"
    runbook.write_text(
        "\n".join([
            "docs/retros/wave-x-evidence.json",
            "wave-x-record",
            "wave-x-validate",
            *(term for term in COMMON_TERMS if term != "stub"),
            *PLACEHOLDER_TERMS,
        ]),
        encoding="utf-8",
    )

    top_errors, gate_results = check.collect_results()

    assert top_errors == []
    assert gate_results[0].ok is False
    assert "stub" in " ".join(gate_results[0].errors)


def test_wave6_evidence_preflight_detects_missing_template_placeholder_terms(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 必须发现 runbook 漏掉模板占位拒绝说明。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(tmp_path, check, monkeypatch)
    runbook = tmp_path / "docs/runbooks/wave-x-evidence.md"
    runbook.write_text(
        "\n".join([
            "docs/retros/wave-x-evidence.json",
            "wave-x-record",
            "wave-x-validate",
            *COMMON_TERMS,
            *(term for term in PLACEHOLDER_TERMS if term != "TBD"),
        ]),
        encoding="utf-8",
    )

    top_errors, gate_results = check.collect_results()

    assert top_errors == []
    assert gate_results[0].ok is False
    assert "TBD" in " ".join(gate_results[0].errors)
