"""Wave 6 tooling、preflight 和 record/validate 资产治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import patch_wave6_report_io


def _patch_wave6_tooling_checks(report, monkeypatch):
    justfile_text = "\n".join(report.WAVE6_JUST_ENTRIES)
    closeout_text = "\n".join([
        "just wave-6-evidence-preflight",
        "just wave-6-complete-check",
        "docs/retros/wave-6-retro.md",
        "Wave 6 完成需要以下全部条件成立",
    ])

    def fake_file_contains(path, *needles):
        if path == "justfile":
            return all(needle in justfile_text for needle in needles)
        if path == "docs/runbooks/wave-6-closeout.md":
            return all(needle in closeout_text for needle in needles)
        return True

    patch_wave6_report_io(monkeypatch, report)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)


def test_wave6_tooling_item_proves_record_validate_and_closeout_assets(monkeypatch):
    """Wave 6 status 必须单独证明 evidence 工具链已齐备。"""
    import report_wave6_pre_release as report

    _patch_wave6_tooling_checks(report, monkeypatch)

    item = {item.item_id: item for item in report.collect_items()}["W6-tooling"]

    assert item.status == report.PROVED_BY_STATIC_FILES
    assert item.blocks_strict is False


def test_wave6_tooling_files_cover_preflight_execution_files():
    """Wave 6 status 报告的 tooling 证据清单必须覆盖 preflight 执行文件。"""
    import check_wave6_evidence_preflight as preflight
    import report_wave6_pre_release as report

    missing = set(preflight.REQUIRED_EXECUTION_FILES) - set(report.WAVE6_TOOLING_FILES)

    assert missing == set()


def test_wave6_tooling_item_blocks_when_report_gate_list_drifts_from_preflight(
    monkeypatch,
):
    """Wave 6 报告的 gate / evidence 清单必须与 preflight 单一事实源一致。"""
    import report_wave6_pre_release as report

    monkeypatch.setattr(report, "WAVE6_GATE_IDS", report.WAVE6_GATE_IDS[:-1])
    _patch_wave6_tooling_checks(report, monkeypatch)

    item = {item.item_id: item for item in report.collect_items()}["W6-tooling"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "preflight" in " ".join(item.gaps)
    assert "W6.H" in " ".join(item.gaps)


def test_wave6_tooling_item_blocks_when_tooling_docs_drift_from_preflight(
    monkeypatch,
):
    """Wave 6 tooling 文档清单必须覆盖 preflight gate 的 runbook。"""
    import report_wave6_pre_release as report

    removed_doc = "docs/runbooks/wave-6-deploy-evidence.md"
    drifted_docs = [doc for doc in report.WAVE6_TOOLING_DOCS if doc != removed_doc]

    monkeypatch.setattr(report, "WAVE6_TOOLING_DOCS", drifted_docs)
    _patch_wave6_tooling_checks(report, monkeypatch)

    item = {item.item_id: item for item in report.collect_items()}["W6-tooling"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert removed_doc in " ".join(item.gaps)


def test_wave6_tooling_item_blocks_when_report_just_entries_drift_from_preflight(
    monkeypatch,
):
    """Wave 6 报告的 just 入口清单必须覆盖 preflight gate 与收口入口。"""
    import report_wave6_pre_release as report

    removed_entry = "wave-6-deploy-evidence-validate"
    drifted_entries = [
        entry for entry in report.WAVE6_JUST_ENTRIES if entry != removed_entry
    ]

    monkeypatch.setattr(report, "WAVE6_JUST_ENTRIES", drifted_entries)
    _patch_wave6_tooling_checks(report, monkeypatch)

    item = {item.item_id: item for item in report.collect_items()}["W6-tooling"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert removed_entry in " ".join(item.gaps)


def test_wave6_tooling_item_blocks_when_validation_commands_drift_from_preflight(
    monkeypatch,
):
    """Wave 6 retro 验证命令清单必须覆盖 preflight gate 的 validator 入口。"""
    import report_wave6_pre_release as report

    removed_command = "just wave-6-deploy-evidence-validate"
    drifted_commands = [
        command
        for command in report.WAVE6_VALIDATION_COMMANDS
        if command != removed_command
    ]

    monkeypatch.setattr(report, "WAVE6_VALIDATION_COMMANDS", drifted_commands)
    _patch_wave6_tooling_checks(report, monkeypatch)

    item = {item.item_id: item for item in report.collect_items()}["W6-tooling"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert removed_command in " ".join(item.gaps)
