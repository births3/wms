"""Wave 6 tooling 失败路径治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave6_tooling_item_blocks_when_preflight_fails(monkeypatch):
    """Wave 6 工具链不能只看文件存在；preflight 失败时必须阻断。"""
    import report_wave6_pre_release as report

    justfile_text = "\n".join(report.WAVE6_JUST_ENTRIES)

    monkeypatch.setattr(report, "file_exists", lambda path: path in set(report.WAVE6_TOOLING_FILES))
    monkeypatch.setattr(
        report,
        "file_contains",
        lambda path, *needles: (
            all(needle in justfile_text for needle in needles)
            if path == "justfile"
            else True
        ),
    )
    monkeypatch.setattr(
        report,
        "run_validator",
        lambda *_args: (False, "check_wave6_evidence_preflight failed"),
    )

    item = {item.item_id: item for item in report.collect_items()}["W6-tooling"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert item.blocks_strict is True
    assert "preflight" in " ".join(item.gaps)


def test_wave6_tooling_item_blocks_when_record_script_is_missing(monkeypatch):
    """缺少任一 record 脚本时，Wave 6 工具链不能算完成。"""
    import report_wave6_pre_release as report

    missing = "scripts/governance/record_wave5_tms_evidence.py"
    justfile_text = "\n".join(report.WAVE6_JUST_ENTRIES)

    def fake_file_exists(path):
        return path in set(report.WAVE6_TOOLING_FILES) and path != missing

    def fake_file_contains(path, *needles):
        if path == "justfile":
            return all(needle in justfile_text for needle in needles)
        if path == "docs/runbooks/wave-6-closeout.md":
            return True
        return True

    monkeypatch.setattr(report, "file_exists", fake_file_exists)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "run_validator", lambda *_args: (False, "missing evidence"))

    item = {item.item_id: item for item in report.collect_items()}["W6-tooling"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert item.blocks_strict is True
    assert missing in " ".join(item.gaps)
