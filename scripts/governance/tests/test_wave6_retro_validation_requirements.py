"""Wave 6 retro 验证命令与边界声明测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _patch_report_with_retro_text(monkeypatch, report, retro_text: str) -> None:
    existing_files = set(report.WAVE6_TOOLING_FILES) | {report.WAVE6_RETRO_FILE}

    def fake_file_contains(path, *needles):
        if path == report.WAVE6_RETRO_FILE:
            return all(needle in retro_text for needle in needles)
        return True

    monkeypatch.setattr(report, "file_exists", lambda path: path in existing_files)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "run_validator", lambda *_args: (True, "ok"))


def test_wave6_retro_item_requires_all_validation_commands(monkeypatch):
    """Wave 6 retro 不能漏掉最终 T1/T2/diff 或任一 evidence validator 结果。"""
    import report_wave6_pre_release as report

    retro_without_task_check = "\n".join([
        *report.WAVE6_GATE_IDS,
        *report.WAVE6_EVIDENCE_FILES,
        *(command for command in report.WAVE6_VALIDATION_COMMANDS if command != "just task-check"),
        "验证结果",
        "剩余风险",
        report.WAVE6_FORBIDDEN_EVIDENCE_BOUNDARY_STATEMENT,
    ])

    _patch_report_with_retro_text(monkeypatch, report, retro_without_task_check)

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "just task-check" in " ".join(item.gaps)


def test_wave6_retro_item_does_not_require_complete_check_result(monkeypatch):
    """retro 是 complete-check 的输入，不能反过来要求记录 complete-check 自己的结果。"""
    import report_wave6_pre_release as report

    retro_without_complete_check = "\n".join([
        *report.WAVE6_GATE_IDS,
        *report.WAVE6_EVIDENCE_FILES,
        *(
            command
            for command in report.WAVE6_VALIDATION_COMMANDS
            if command != "just wave-6-complete-check"
        ),
        "验证结果",
        "剩余风险",
        report.WAVE6_FORBIDDEN_EVIDENCE_BOUNDARY_STATEMENT,
    ])

    _patch_report_with_retro_text(monkeypatch, report, retro_without_complete_check)

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.PROVED_BY_STATIC_FILES
    assert item.blocks_strict is False


def test_wave6_retro_item_requires_preflight_result(monkeypatch):
    """Wave 6 retro 必须记录 wave-6-evidence-preflight 的执行结果。"""
    import report_wave6_pre_release as report

    retro_without_preflight = "\n".join([
        *report.WAVE6_GATE_IDS,
        *report.WAVE6_EVIDENCE_FILES,
        *(
            command
            for command in report.WAVE6_VALIDATION_COMMANDS
            if command != "just wave-6-evidence-preflight"
        ),
        "验证结果",
        "剩余风险",
        report.WAVE6_FORBIDDEN_EVIDENCE_BOUNDARY_STATEMENT,
    ])

    _patch_report_with_retro_text(monkeypatch, report, retro_without_preflight)

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "just wave-6-evidence-preflight" in " ".join(item.gaps)


def test_wave6_retro_item_requires_production_boundary_statement(monkeypatch):
    """Wave 6 retro 必须声明没有使用 production 证据，不只是不使用 prod。"""
    import report_wave6_pre_release as report

    retro_without_production = "\n".join([
        *report.WAVE6_GATE_IDS,
        *report.WAVE6_EVIDENCE_FILES,
        *report.WAVE6_VALIDATION_COMMANDS,
        "验证结果",
        "剩余风险",
        "没有使用 local/mock/fake/stub/example/prod",
    ])

    _patch_report_with_retro_text(monkeypatch, report, retro_without_production)

    item = {item.item_id: item for item in report.collect_items()}["W6-retro"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert "production" in " ".join(item.gaps)
