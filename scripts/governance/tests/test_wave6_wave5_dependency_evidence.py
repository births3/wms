"""Wave 6 对 Wave 5 外部证据依赖的治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave6_report_uses_wave5_validators_when_evidence_is_missing(monkeypatch):
    """W6.F/W6.G 已有 validator 时，缺 evidence 应归类为外部状态缺失。"""
    import report_wave6_pre_release as report

    def fake_run_validator(*args):
        command = " ".join(args)
        if "validate_wave5_hardware_evidence.py" in command:
            return False, "missing file: docs/retros/wave-5-hardware-evidence.json"
        if "validate_wave5_tms_evidence.py" in command:
            return False, "missing file: docs/retros/wave-5-tms-evidence.json"
        return True, "ok"

    validators = {
        "scripts/governance/validate_wave5_hardware_evidence.py",
        "scripts/governance/validate_wave5_tms_evidence.py",
    }
    monkeypatch.setattr(report, "file_exists", lambda path: path in validators)
    monkeypatch.setattr(report, "file_contains", lambda _path, *_needles: True)
    monkeypatch.setattr(report, "run_validator", fake_run_validator)

    items = {item.item_id: item for item in report.collect_items()}

    assert items["W6.F-wave5-hardware"].status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert items["W6.G-wave5-tms"].status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "wave-5-hardware-evidence.json" in " ".join(items["W6.F-wave5-hardware"].gaps)
    assert "wave-5-tms-evidence.json" in " ".join(items["W6.G-wave5-tms"].gaps)
