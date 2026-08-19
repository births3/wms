"""Wave 6 closeout runbook 的 report JSON 合同测试。"""
from pathlib import Path


def test_wave6_closeout_runbook_documents_report_json_contract():
    """Wave 6 closeout runbook 必须写明机器可消费的 report JSON 字段。"""
    text = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")

    for field in (
        "schema_version",
        "available_modes",
        "writes_runtime_evidence",
        "closes_gate",
        "report_command",
        "evidence_only_command",
        "commands_only_command",
        "evidence_gate_ids",
        "evidence_gate_evidence_files",
        "evidence_gate_just_entries",
        "evidence_gate_execution_files",
        "required_top_level_files",
        "required_runbooks",
        "required_execution_files",
        "validation_commands",
        "closeout_just_entries",
        "evidence_gate_item_ids",
        "non_evidence_item_ids",
        "blocking_count",
        "ignored_count",
        "blocking_details",
        "ignored_details",
        "evidence_blocking_count",
        "evidence_ignored_count",
        "missing_evidence_count",
        "evidence_blocking_item_ids",
        "non_evidence_ignored_item_ids",
        "missing_evidence_files",
        "missing_evidence_item_ids",
        "missing_evidence_details",
        "readiness_commands",
        "record_commands",
        "validate_commands",
        "deployment_choice_required",
        "deployment_choice_label",
        "deployment_choice_options",
        "deployment_path_commands",
    ):
        assert f"`{field}`" in text


def test_wave6_closeout_runbook_requires_empty_missing_evidence_before_retro():
    """Wave 6 retro 只能在 8 个真实 evidence gate 全部关闭后编写。"""
    text = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")

    assert "`missing_evidence_item_ids` / `missing_evidence_files`" in text
    assert "写 retro 前必须为空" in text
    assert "just wave-6-evidence-check" in text
    assert "然后写 `docs/retros/wave-6-retro.md`" in text


def test_wave6_closeout_completion_criteria_orders_retro_before_complete_check():
    """完成口径不能暗示 complete-check 可在 retro 写入前通过。"""
    text = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")
    criteria = text.split("## 当前 Gate", maxsplit=1)[0]

    retro_index = criteria.index("`docs/retros/wave-6-retro.md`")
    complete_check_index = criteria.index("`just wave-6-complete-check`")

    assert retro_index < complete_check_index
