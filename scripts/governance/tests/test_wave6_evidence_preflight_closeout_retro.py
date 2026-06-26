"""Wave 6 evidence preflight closeout retro 顺序测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import write_closeout_preflight_fixture


def test_wave6_evidence_preflight_requires_retro_missing_evidence_prerequisites(
    tmp_path,
    monkeypatch,
):
    """Closeout 提到 retro 时必须写清 missing_evidence 清空前置条件。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
            "然后写 `docs/retros/wave-6-retro.md`",
            "just wave-6-complete-check",
        ],
        just_text="wave-6-evidence-preflight:\nwave-6-complete-check:\n",
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "missing_evidence_item_ids" in joined_errors
    assert "missing_evidence_files" in joined_errors
    assert "写 retro 前必须为空" in joined_errors


def test_wave6_evidence_preflight_requires_retro_before_complete_check(
    tmp_path,
    monkeypatch,
):
    """Closeout 完成口径不能暗示 complete-check 可在 retro 写入前通过。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
            "just wave-6-evidence-check",
            "`missing_evidence_item_ids` / `missing_evidence_files` 写 retro 前必须为空",
            "just wave-6-complete-check",
            "然后写 `docs/retros/wave-6-retro.md`",
        ],
        just_text=(
            "wave-6-evidence-preflight:\n"
            "wave-6-evidence-check:\n"
            "wave-6-complete-check:\n"
        ),
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "docs/retros/wave-6-retro.md" in joined_errors
    assert "just wave-6-complete-check" in joined_errors
    assert "之前" in joined_errors


def test_wave6_evidence_preflight_checks_completion_criteria_retro_order(
    tmp_path,
    monkeypatch,
):
    """导语提到 retro 顺序时，完成口径段落仍必须单独保持 retro 在 complete-check 前。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "导语：`docs/retros/wave-6-retro.md` 写入后再跑 `just wave-6-complete-check`。",
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "just wave-6-status",
            "just wave-6-missing-evidence-commands",
            "## 完成口径",
            "0. just wave-6-evidence-preflight",
            "1. just wave-6-evidence-check",
            "2. `missing_evidence_item_ids` / `missing_evidence_files` 写 retro 前必须为空",
            "3. `just wave-6-complete-check` 退出 0",
            "4. 然后写 `docs/retros/wave-6-retro.md`",
            "## 当前 Gate",
        ],
        just_text=(
            "wave-6-evidence-preflight:\n"
            "wave-6-status:\n"
            "wave-6-evidence-check:\n"
            "wave-6-missing-evidence-commands:\n"
            "wave-6-complete-check:\n"
        ),
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "完成口径" in joined_errors
    assert "docs/retros/wave-6-retro.md" in joined_errors
    assert "just wave-6-complete-check" in joined_errors


def test_wave6_evidence_preflight_requires_evidence_check_before_retro_in_completion_criteria(
    tmp_path,
    monkeypatch,
):
    """完成口径必须把 evidence-check 明确列在 retro 写入之前。"""
    import check_wave6_evidence_preflight as check

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_lines=[
            "正文其他位置提到 just wave-6-evidence-check。",
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "```bash",
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            "```",
            "## 完成口径",
            "0. just wave-6-evidence-preflight",
            "1. `docs/retros/wave-6-retro.md` 已写入本轮真实 evidence 结果和剩余风险",
            "2. `just wave-6-complete-check` 退出 0",
            "## 当前 Gate",
            "`missing_evidence_item_ids` / `missing_evidence_files` 写 retro 前必须为空",
        ],
        just_text=(
            "wave-6-evidence-preflight:\n"
            "    @python3 scripts/governance/check_wave6_evidence_preflight.py\n"
            "wave-6-status:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py\n"
            "wave-6-evidence-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict --evidence-only\n"
            "wave-6-missing-evidence-commands:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --commands-only --strict --evidence-only\n"
            "wave-6-complete-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
        ),
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "完成口径" in joined_errors
    assert "just wave-6-evidence-check" in joined_errors
    assert "docs/retros/wave-6-retro.md" in joined_errors


def test_wave6_closeout_completion_criteria_runs_status_after_retro():
    """完成口径中 status 无阻塞缺口必须发生在 retro 写入之后。"""
    text = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")
    criteria = text.split(
        "Wave 6 完成需要以下全部条件成立：",
        maxsplit=1,
    )[1].split("## 当前 Gate", maxsplit=1)[0]

    evidence_check_index = criteria.index("`just wave-6-evidence-check`")
    retro_index = criteria.index("`docs/retros/wave-6-retro.md`")
    status_index = criteria.index("`just wave-6-status` 无阻塞缺口")
    complete_check_index = criteria.index("`just wave-6-complete-check`")

    assert evidence_check_index < retro_index < status_index < complete_check_index
