"""Wave 6 evidence preflight closeout record command hygiene tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import write_closeout_preflight_fixture


def test_wave6_evidence_preflight_rejects_closeout_placeholder_record_args(
    tmp_path,
    monkeypatch,
):
    """Wave 6 closeout 不能保留 just record <真实参数> 这类模板占位命令。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "```bash",
            "just wave-x-record <真实参数>",
            "```",
        ],
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    assert "真实参数" in " ".join(top_errors)
    assert "closeout" in " ".join(top_errors)


def test_wave6_evidence_preflight_rejects_closeout_force_record_examples(
    tmp_path,
    monkeypatch,
):
    """Wave 6 closeout 示例不能鼓励用 --force 覆盖真实 evidence。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
            "```bash",
            "just wave-x-record --force",
            "```",
        ],
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "--force" in joined_errors
    assert "closeout" in joined_errors


def test_wave6_evidence_preflight_ignores_commented_closeout_force_examples(
    tmp_path,
    monkeypatch,
):
    """closeout shell 代码块中注释掉的 --force 示例不能触发错误。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
            "```bash",
            "# just wave-x-record --force",
            "just wave-x-record",
            "```",
        ],
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    assert "--force" not in " ".join(top_errors)
