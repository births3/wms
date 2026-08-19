"""Wave 6 evidence preflight closeout 入口清单测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_closeout_justfile_test_helpers import (
    write_closeout_justfile_fixture,
)
from wave6_preflight_test_helpers import write_closeout_preflight_fixture


def test_wave6_evidence_preflight_detects_missing_closeout_just_entries(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 必须发现 closeout 文档要求但 justfile 缺失的包装入口。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        preflight_lines=[
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
        ],
        closeout_lines=[
            "just wave-6-evidence-preflight",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            check.PREFLIGHT_DOC,
        ],
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "wave-6-status" in joined_errors
    assert "wave-6-evidence-check" in joined_errors
    assert "wave-6-missing-evidence-commands" in joined_errors
    assert "wave-6-complete-check" in joined_errors


def test_wave6_evidence_preflight_requires_documenting_all_closeout_just_entries(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight/closeout 文档必须登记完整收口入口清单。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        preflight_lines=[
            "just wave-6-status",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
        ],
        closeout_lines=[
            "just wave-6-evidence-preflight",
            "just wave-6-status",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
        ],
        just_text=(
            "wave-6-evidence-preflight:\n"
            "wave-6-status:\n"
            "wave-6-evidence-check:\n"
            "wave-6-missing-evidence-commands:\n"
            "wave-6-complete-check:\n"
        ),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "文档缺少 Wave 6 收口入口" in joined_errors
    assert "wave-6-evidence-check" in joined_errors


def test_wave6_evidence_preflight_requires_closeout_report_json_fields(
    tmp_path,
    monkeypatch,
):
    """preflight 必须发现 closeout 漏写 report JSON 机器消费字段。"""
    import check_wave6_evidence_preflight as check

    write_closeout_justfile_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "just wave-6-evidence-preflight",
            "```bash",
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            "```",
            check.PREFLIGHT_DOC,
            check.WAVE6_CLOSEOUT_REPORT_JSON_SECTION_MARKER,
            "## 当前 Gate",
        ],
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "closeout report JSON 字段清单缺少" in joined_errors
    assert "schema_version" in joined_errors
    assert "deployment_path_commands" in joined_errors


def test_wave6_evidence_preflight_report_json_field_contract_includes_record_check_only_commands():
    """preflight 字段清单必须覆盖 report JSON 的 record-check-only 命令字段。"""
    import check_wave6_evidence_preflight as check

    assert "record_check_only_commands" in check.WAVE6_CLOSEOUT_REPORT_JSON_FIELDS
