"""Wave 6 retro 收口证据要求治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave6_retro_item_blocks_when_retro_is_missing(monkeypatch):
    """Wave 6 complete-check 必须把收口 retro 作为独立阻塞项。"""
    import report_wave6_pre_release as report

    existing_files = set(report.WAVE6_TOOLING_FILES)

    monkeypatch.setattr(report, "file_exists", lambda path: path in existing_files)
    monkeypatch.setattr(report, "file_contains", lambda _path, *_needles: True)
    monkeypatch.setattr(report, "run_validator", lambda *_args: (True, "ok"))

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert item.blocks_strict is True
    assert "wave-6-retro.md" in " ".join(item.gaps)

def test_wave6_retro_item_requires_evidence_paths_and_risk_statement(monkeypatch):
    """Wave 6 retro 必须列出 8 个 evidence 文件、验证命令结果和剩余风险。"""
    import report_wave6_pre_release as report

    existing_files = set(report.WAVE6_TOOLING_FILES) | {report.WAVE6_RETRO_FILE}
    incomplete_retro = "\n".join([
        "docs/retros/wave-1-h2-runtime-evidence.json",
        *report.WAVE6_VALIDATION_COMMANDS,
        "验证结果",
        "没有使用 local/mock/fake/stub/example/prod",
    ])

    def fake_file_contains(path, *needles):
        if path == report.WAVE6_RETRO_FILE:
            return all(needle in incomplete_retro for needle in needles)
        return True

    monkeypatch.setattr(report, "file_exists", lambda path: path in existing_files)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "run_validator", lambda *_args: (True, "ok"))

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "wave-2-runtime-evidence.json" in " ".join(item.gaps)
    assert "剩余风险" in " ".join(item.gaps)

    complete_retro = "\n".join([
        *report.WAVE6_GATE_IDS,
        *report.WAVE6_EVIDENCE_FILES,
        *report.WAVE6_VALIDATION_COMMANDS,
        "验证结果",
        "剩余风险",
        report.WAVE6_FORBIDDEN_EVIDENCE_BOUNDARY_STATEMENT,
    ])

    def complete_file_contains(path, *needles):
        if path == report.WAVE6_RETRO_FILE:
            return all(needle in complete_retro for needle in needles)
        return True

    monkeypatch.setattr(report, "file_contains", complete_file_contains)

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.PROVED_BY_STATIC_FILES
    assert item.blocks_strict is False


def test_wave6_retro_item_requires_all_gate_ids(monkeypatch):
    """Wave 6 retro 必须显式列出 W6.A-W6.H，不能只贴 evidence 文件路径。"""
    import report_wave6_pre_release as report

    existing_files = set(report.WAVE6_TOOLING_FILES) | {report.WAVE6_RETRO_FILE}
    retro_without_gate_ids = "\n".join([
        *report.WAVE6_EVIDENCE_FILES,
        *report.WAVE6_VALIDATION_COMMANDS,
        "验证结果",
        "剩余风险",
        report.WAVE6_FORBIDDEN_EVIDENCE_BOUNDARY_STATEMENT,
    ])

    def fake_file_contains(path, *needles):
        if path == report.WAVE6_RETRO_FILE:
            return all(needle in retro_without_gate_ids for needle in needles)
        return True

    monkeypatch.setattr(report, "file_exists", lambda path: path in existing_files)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "run_validator", lambda *_args: (True, "ok"))

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "W6.A" in " ".join(item.gaps)
    assert "W6.H" in " ".join(item.gaps)
