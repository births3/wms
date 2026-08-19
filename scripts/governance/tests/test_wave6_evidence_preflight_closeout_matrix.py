"""Wave 6 evidence preflight closeout 当前 Gate 矩阵测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import (
    closeout_text_lines,
    make_gate,
    write_single_gate_preflight_fixture,
)


def test_wave6_evidence_preflight_requires_closeout_gate_matrix_sync(
    tmp_path,
    monkeypatch,
):
    """Wave 6 closeout 的当前 Gate 矩阵必须同步登记每个 evidence 文件和命令入口。"""
    import check_wave6_evidence_preflight as check

    gate = make_gate(check)
    complete_closeout_lines = closeout_text_lines(check, gate)
    required_needles = (
        "W6.X",
        "docs/retros/wave-x-evidence.json",
        "wave-x-record",
        "wave-x-validate",
    )

    for missing_needle in required_needles:
        write_single_gate_preflight_fixture(
            tmp_path,
            check,
            monkeypatch,
            closeout_text="\n".join(
                line.replace(missing_needle, "")
                for line in complete_closeout_lines
            ),
        )

        top_errors, gate_results = check.collect_results()

        assert gate_results[0].ok is True
        joined_errors = " ".join(top_errors)
        assert missing_needle in joined_errors
        assert "closeout" in joined_errors

        (tmp_path / check.CLOSEOUT_DOC).write_text(
            "\n".join(complete_closeout_lines),
            encoding="utf-8",
        )

    top_errors, gate_results = check.collect_results()

    assert top_errors == []
    assert gate_results[0].ok is True


def test_wave6_evidence_preflight_requires_closeout_gate_matrix_heading(
    tmp_path,
    monkeypatch,
):
    """Wave 6 closeout 不能整段漏掉当前 Gate 矩阵。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_text="\n".join([
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
        ]),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results[0].ok is True
    assert "当前 Gate" in " ".join(top_errors)
    assert "closeout" in " ".join(top_errors)


def test_wave6_evidence_preflight_requires_closeout_gate_matrix_row_sync(
    tmp_path,
    monkeypatch,
):
    """后续代码块提到命令时，当前 Gate 矩阵行仍必须自己列全 record / validate。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_text="\n".join([
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
            "| Gate | Evidence 文件 | 记录入口 | 验证入口 |",
            "|------|---------------|----------|----------|",
            "| W6.X | docs/retros/wave-x-evidence.json |  | wave-x-validate |",
            "## 推荐执行顺序",
            "```bash",
            "just wave-x-record",
            "just wave-x-validate",
            "```",
        ]),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results[0].ok is True
    joined_errors = " ".join(top_errors)
    assert "W6.X" in joined_errors
    assert "wave-x-record" in joined_errors
    assert "矩阵" in joined_errors
