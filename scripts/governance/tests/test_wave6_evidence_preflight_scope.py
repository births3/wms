"""Wave 6 evidence preflight just 覆盖与 PDA handoff 测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import write_single_gate_preflight_fixture


def test_wave6_evidence_preflight_detects_missing_just_entry(tmp_path, monkeypatch):
    """Wave 6 preflight 必须能发现 runbook 中登记但 justfile 缺失的入口。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(tmp_path, check, monkeypatch)

    justfile = tmp_path / check.JUSTFILE
    closeout_entries = (
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
    )
    justfile.write_text(
        f"{closeout_entries}wave-x-record:\nwave-x-validate:\n",
        encoding="utf-8",
    )
    top_errors, gate_results = check.collect_results()
    assert top_errors == []
    assert gate_results[0].ok is True

    justfile.write_text(f"{closeout_entries}wave-x-validate:\n", encoding="utf-8")
    _top_errors, gate_results = check.collect_results()
    assert gate_results[0].ok is False
    assert "wave-x-record" in " ".join(gate_results[0].errors)


def test_wave6_evidence_preflight_requires_pda_printing_handoff_terms():
    """W6.D PDA 证据必须显式说明蓝牙打印由 Wave 5 硬件 evidence 接住。"""
    import check_wave6_evidence_preflight as check

    w6d_gate = next(gate for gate in check.GATES if gate.gate_id == "W6.D")

    assert "蓝牙打印" in w6d_gate.required_terms
    assert "Wave 5" in w6d_gate.required_terms
    assert "docs/retros/wave-5-hardware-evidence.json" in w6d_gate.required_terms
