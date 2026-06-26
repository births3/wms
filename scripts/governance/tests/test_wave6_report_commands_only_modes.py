"""Wave 6 pre-release report commands-only 模式语义测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import (
    patch_wave6_report_io,
    wave6_missing_file_validator,
)


def test_wave6_commands_only_preserves_strict_exit_semantics(monkeypatch, capsys):
    """commands-only 只改变输出，不改变 strict 阻塞语义。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave3_pda_runtime_evidence.py": (
                "docs/retros/wave-3-pda-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--evidence-only"]) == 0
    non_strict_output = capsys.readouterr().out
    assert "readiness: just wave-3-pda-runtime-readiness" in non_strict_output
    assert "record: just wave-3-pda-runtime-evidence-record" in non_strict_output
    assert "record: just wave-3-pda-intake-record --json" in non_strict_output

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    strict_output = capsys.readouterr().out
    assert "readiness: just wave-3-pda-runtime-readiness" in strict_output
    assert "record: just wave-3-pda-runtime-evidence-record" in strict_output
    assert "record: just wave-3-pda-intake-record --json" in strict_output


def test_wave6_commands_only_complete_mode_excludes_retro_from_command_checklist(
    monkeypatch,
    capsys,
):
    """commands-only complete 模式也不能把 retro 当成可采集 evidence gate。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(monkeypatch, report)

    assert report.main(["--commands-only", "--strict"]) == 1
    output = capsys.readouterr().out

    assert "Wave 6 missing evidence commands: none" in output
    assert "W6-retro" not in output
    assert "docs/retros/wave-6-retro.md" not in output


def test_wave6_commands_only_complete_mode_explains_none_is_evidence_commands_only(
    monkeypatch,
    capsys,
):
    """complete 模式输出 none 但 strict 非零时，必须说明 none 只指 evidence 命令。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(monkeypatch, report)

    assert report.main(["--commands-only", "--strict"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    assert output_lines == [
        report.COMMANDS_ONLY_NONE_LINE,
        report.COMMANDS_ONLY_NONE_COMPLETE_MODE_LINE,
    ]
    assert "只表示没有缺失 evidence gate 的采集命令" in output_lines[1]
    assert "complete-check 仍可能因非 evidence blocker 返回非零" in output_lines[1]


def test_wave6_commands_only_rejects_json_mode(monkeypatch, capsys):
    """commands-only 是文本快捷清单，不和 JSON report 混用。"""
    import report_wave6_pre_release as report

    monkeypatch.setattr(report, "collect_items", lambda: [])

    assert report.main(["--commands-only", "--json"]) == 2
    captured = capsys.readouterr()
    assert "--commands-only cannot be combined with --json" in captured.err


def test_wave6_closeout_runbook_documents_commands_only_mode():
    """closeout runbook 必须同步说明 commands-only 人工收口入口。"""
    import report_wave6_pre_release as report

    runbook = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")

    assert "report_wave6_pre_release.py --commands-only --strict --evidence-only" in runbook
    assert "--commands-only cannot be combined with --json" in runbook
    assert "首行输出只读边界" in runbook
    assert "随后输出缺失 evidence gate 的命令清单" in runbook
    assert "readiness / record-check-only / record / validate" in runbook
    assert "缺失 evidence 时会因 --strict 返回非零" in runbook
    assert "这是阻塞信号，不代表命令写入或关闭 gate" in runbook
    assert "不会写入 runtime evidence，也不会关闭 evidence gate" in runbook
    assert report.COMMANDS_ONLY_BOUNDARY_LINE in runbook
    assert report.COMMANDS_ONLY_STRICT_EXIT_LINE in runbook
    assert report.COMMANDS_ONLY_NONE_LINE in runbook
    assert report.COMMANDS_ONLY_NONE_COMPLETE_MODE_LINE in runbook
    assert report.W6B_ROLLBACK_DEPLOYMENT_CHOICE_LINE in runbook
    assert "`path: k8s` / `path: docker-compose`" in runbook
    assert "just wave-6-status 普通文本报告" in runbook
    assert (
        "just wave-6-status 普通文本报告和 commands-only 清单都会在 W6.B 缺失时提示二选一"
        in runbook
    )
    assert (
        "just wave-6-status 普通文本报告和 commands-only 清单都会用 "
        "`path: k8s` / `path: docker-compose` 分组"
        in runbook
    )
    assert "record_check_only_commands" in runbook
