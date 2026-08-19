"""Wave 6 report evidence gate 顺序治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import (
    patch_wave6_report_io,
    wave6_missing_file_validator,
)


def _patch_wave6_tooling_present(report, monkeypatch, run_validator=None) -> None:
    patch_wave6_report_io(monkeypatch, report, run_validator=run_validator)


def test_wave6_startup_item_requires_every_scope_doc_to_list_all_gates(monkeypatch):
    """W6-startup 不能在 ROADMAP / 依赖图漏登记某个 gate 时显示完成。"""
    import report_wave6_pre_release as report

    all_gates = " ".join(report.WAVE6_GATE_IDS)
    missing_w6h = " ".join(gate_id for gate_id in report.WAVE6_GATE_IDS if gate_id != "W6.H")
    scope_base = "当前 Wave：Wave 6\nWave 6：预发布证据与外部依赖收口\nADR-0035\n"

    def fake_read_text(path):
        if path == "ROADMAP.md":
            return scope_base + missing_w6h
        if path in {
            "TODO.md",
            "docs/architecture-dependencies.md",
            "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
        }:
            return scope_base + all_gates
        if path == "justfile":
            return "\n".join(report.WAVE6_JUST_ENTRIES)
        if path == "docs/runbooks/wave-6-closeout.md":
            return "\n".join([
                "just wave-6-evidence-preflight",
                "just wave-6-complete-check",
                "docs/retros/wave-6-retro.md",
                "Wave 6 完成需要以下全部条件成立",
            ])
        if path == "docs/retros/wave-5-retro.md":
            return "Wave 5 开发完成"
        if path == "scripts/governance/report_wave5_completion.py":
            return "W5-chain-scenario"
        return all_gates

    monkeypatch.setattr(report, "read_text", fake_read_text)
    monkeypatch.setattr(report, "file_exists", lambda _path: True)
    monkeypatch.setattr(report, "run_validator", lambda *_args: (True, "ok"))

    item = {item.item_id: item for item in report.collect_items()}["W6-startup"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "ROADMAP.md" in " ".join(item.gaps)
    assert "W6.H" in " ".join(item.gaps)


def test_wave6_report_uses_pda_and_deploy_validators_when_evidence_is_missing(monkeypatch):
    """W6.D/W6.H 已有 validator 时，缺 evidence 应归类为外部状态缺失。"""
    import report_wave6_pre_release as report

    fake_run_validator = wave6_missing_file_validator({
        "validate_wave3_pda_runtime_evidence.py": (
            "docs/retros/wave-3-pda-runtime-evidence.json"
        ),
        "validate_wave6_deploy_evidence.py": (
            "docs/retros/wave-6-deploy-evidence.json"
        ),
    })

    validators = {
        "scripts/governance/validate_wave3_pda_runtime_evidence.py",
        "scripts/governance/validate_wave6_deploy_evidence.py",
    }
    monkeypatch.setattr(report, "file_exists", lambda path: path in validators)
    monkeypatch.setattr(report, "file_contains", lambda _path, *_needles: True)
    monkeypatch.setattr(report, "run_validator", fake_run_validator)

    items = {item.item_id: item for item in report.collect_items()}

    assert items["W6.D-wave3-pda-l7"].status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert items["W6.H-gray-release"].status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "wave-3-pda-runtime-evidence.json" in " ".join(items["W6.D-wave3-pda-l7"].gaps)
    assert "wave-6-deploy-evidence.json" in " ".join(items["W6.H-gray-release"].gaps)


def test_wave6_report_keeps_gate_order_aligned_with_closeout_matrix(monkeypatch):
    """Wave 6 报告顺序必须对齐 closeout/preflight 的 W6.A-H gate 顺序。"""
    import report_wave6_pre_release as report

    _patch_wave6_tooling_present(report, monkeypatch)

    item_ids = [item.item_id for item in report.collect_items()]

    assert item_ids.index("W6.D-wave3-pda-l7") < item_ids.index(
        "W6.E-wave4-traceability-external"
    )


def test_wave6_report_splits_wave1_h2_and_rollback_gates(monkeypatch):
    """Wave 6 报告必须把 W6.A H2 与 W6.B W1.D 回滚列为两个真实 evidence gate。"""
    import report_wave6_pre_release as report

    wave1_commands: list[str] = []

    def fake_run_validator(*args):
        command = " ".join(args)
        if "validate_wave1_runtime_evidence.py" in command:
            wave1_commands.append(command)
        return True, "ok"

    _patch_wave6_tooling_present(report, monkeypatch, fake_run_validator)

    item_ids = [item.item_id for item in report.collect_items()]

    assert "W6.A-wave1-h2-runtime" in item_ids
    assert "W6.B-wave1-rollback-runtime" in item_ids
    assert "W6.AB-wave1-runtime" not in item_ids
    assert item_ids.index("W6.A-wave1-h2-runtime") < item_ids.index(
        "W6.B-wave1-rollback-runtime"
    )
    assert any("--kind h2" in command for command in wave1_commands)
    assert any("--kind w1d" in command for command in wave1_commands)
