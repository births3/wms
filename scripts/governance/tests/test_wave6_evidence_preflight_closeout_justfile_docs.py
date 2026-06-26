"""Wave 6 evidence preflight closeout 文档化 just 入口测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_closeout_justfile_test_helpers import (
    EXPECTED_JUST_TEXT,
    write_closeout_justfile_fixture,
)


def test_wave6_evidence_preflight_requires_closeout_entries_in_command_blocks(
    tmp_path,
    monkeypatch,
):
    """收口入口只出现在字段说明或普通文字里，不能算已文档化的执行命令。"""
    import check_wave6_evidence_preflight as check

    write_closeout_justfile_fixture(
        tmp_path,
        check,
        monkeypatch,
        preflight_lines=[
            "字段说明：just wave-6-status",
            "字段说明：just wave-6-evidence-check",
            "字段说明：just wave-6-missing-evidence-commands",
            "字段说明：just wave-6-complete-check",
        ],
        closeout_lines=[
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
        ],
        just_text=EXPECTED_JUST_TEXT,
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "文档缺少 Wave 6 收口入口" in joined_errors
    assert "wave-6-status" in joined_errors
    assert "wave-6-evidence-check" in joined_errors
    assert "wave-6-missing-evidence-commands" in joined_errors
    assert "wave-6-complete-check" in joined_errors


def test_wave6_evidence_preflight_requires_closeout_entries_in_shell_blocks(
    tmp_path,
    monkeypatch,
):
    """普通代码块里的 just 文本不能算可执行 shell 命令。"""
    import check_wave6_evidence_preflight as check

    write_closeout_justfile_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "just wave-6-evidence-preflight",
            "```",
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            "```",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
        ],
        just_text=EXPECTED_JUST_TEXT,
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "文档缺少 Wave 6 收口入口" in joined_errors
    assert "wave-6-status" in joined_errors
