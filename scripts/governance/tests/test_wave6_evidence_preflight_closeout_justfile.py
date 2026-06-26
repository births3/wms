"""Wave 6 evidence preflight closeout justfile 包装入口测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_closeout_justfile_test_helpers import write_closeout_justfile_fixture


def test_wave6_evidence_preflight_requires_closeout_justfile_wrapper_commands(
    tmp_path,
    monkeypatch,
):
    """Wave 6 收口包装入口必须调用对应 report 命令和参数。"""
    import check_wave6_evidence_preflight as check

    write_closeout_justfile_fixture(
        tmp_path,
        check,
        monkeypatch,
        just_text=(
            "wave-6-evidence-preflight:\n"
            "    @python3 scripts/governance/check_wave6_evidence_preflight.py\n"
            "wave-6-status:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py\n"
            "wave-6-evidence-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
            "wave-6-missing-evidence-commands:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --commands-only --strict --evidence-only\n"
            "wave-6-complete-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
        ),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "wave-6-evidence-check" in joined_errors
    assert "--evidence-only" in joined_errors


def test_wave6_evidence_preflight_ignores_commented_closeout_wrapper_tokens(
    tmp_path,
    monkeypatch,
):
    """收口 wrapper 的期望参数出现在注释里不能算作真实命令。"""
    import check_wave6_evidence_preflight as check

    write_closeout_justfile_fixture(
        tmp_path,
        check,
        monkeypatch,
        just_text=(
            "wave-6-evidence-preflight:\n"
            "    @python3 scripts/governance/check_wave6_evidence_preflight.py\n"
            "wave-6-status:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py\n"
            "wave-6-evidence-check:\n"
            "    # python3 scripts/governance/report_wave6_pre_release.py --strict --evidence-only\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
            "wave-6-missing-evidence-commands:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --commands-only --strict --evidence-only\n"
            "wave-6-complete-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
        ),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "wave-6-evidence-check" in joined_errors
    assert "--evidence-only" in joined_errors


def test_wave6_evidence_preflight_rejects_conflicting_closeout_wrapper_args(
    tmp_path,
    monkeypatch,
):
    """收口 wrapper 不能额外带会改变入口语义的参数。"""
    import check_wave6_evidence_preflight as check

    write_closeout_justfile_fixture(
        tmp_path,
        check,
        monkeypatch,
        just_text=(
            "wave-6-evidence-preflight:\n"
            "    @python3 scripts/governance/check_wave6_evidence_preflight.py\n"
            "wave-6-status:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
            "wave-6-evidence-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict --evidence-only\n"
            "wave-6-missing-evidence-commands:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --commands-only --json --strict --evidence-only\n"
            "wave-6-complete-check:\n"
            "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
        ),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    joined_errors = " ".join(top_errors)
    assert "wave-6-status" in joined_errors
    assert "--strict" in joined_errors
    assert "wave-6-missing-evidence-commands" in joined_errors
    assert "--json" in joined_errors


def test_wave6_evidence_preflight_accepts_expected_closeout_justfile_wrappers(
    tmp_path,
    monkeypatch,
):
    """Wave 6 收口包装入口参数正确时不应报错。"""
    import check_wave6_evidence_preflight as check

    write_closeout_justfile_fixture(
        tmp_path,
        check,
        monkeypatch,
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results == []
    assert top_errors == []
