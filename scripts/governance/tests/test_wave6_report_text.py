"""Wave 6 pre-release report text-mode governance tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import (
    patch_wave6_report_io,
    wave6_missing_file_validator,
)


def test_wave6_evidence_only_mode_ignores_retro_but_complete_check_does_not(monkeypatch, capsys):
    """写 retro 前可只检查 evidence gate；最终 complete-check 仍必须要求 retro。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(monkeypatch, report)

    assert report.main(["--strict", "--evidence-only"]) == 0
    evidence_only_output = capsys.readouterr().out
    assert "W6-retro" in evidence_only_output
    assert "evidence-only 不阻塞" in evidence_only_output
    assert "缺少 docs/retros/wave-6-retro.md" not in evidence_only_output

    assert report.main(["--strict"]) == 1
    complete_output = capsys.readouterr().out
    assert "缺少 docs/retros/wave-6-retro.md" in complete_output


def test_wave6_evidence_only_mode_still_blocks_missing_evidence(monkeypatch):
    """evidence-only 只忽略 retro，不能忽略任一真实 evidence gate。"""
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

    assert report.main(["--strict", "--evidence-only"]) == 1


def test_wave6_evidence_only_mode_still_blocks_non_evidence_prerequisites(monkeypatch):
    """evidence-only 只忽略 W6-retro，不忽略 startup/tooling 等前置阻塞项。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(monkeypatch, report, existing_files=())

    assert report.main(["--strict", "--evidence-only"]) == 1


def test_wave6_cli_description_mentions_non_evidence_blockers():
    """脚本说明必须避免把 strict 误写成只阻塞 evidence gate。"""
    import report_wave6_pre_release as report

    assert "任一 strict blocking item 未关闭返回 1" in report.__doc__
    assert "evidence-only 仅忽略 W6-retro" in report.__doc__


def test_wave6_text_report_prints_missing_evidence_commands(monkeypatch, capsys):
    """人类可读报告也必须给出缺失 evidence 的采集和验证命令。"""
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

    assert report.main(["--strict", "--evidence-only"]) == 1
    output = capsys.readouterr().out

    assert "W6.D-wave3-pda-l7" in output
    assert "external-prereq: 真 PDA" in output
    assert "external-prereq: 幂等 replay 条件" in output
    assert "minimum-evidence-ref: PDA 资产引用" in output
    assert "minimum-evidence-ref: idempotency replay 日志" in output
    assert "minimum-evidence-ref: L7 执行记录" in output
    assert "readiness: just wave-3-pda-runtime-readiness --from-env --json" in output
    assert "record: just wave-3-pda-runtime-evidence-record --from-env --json" in output
    assert "record: just wave-3-pda-intake-record --json" in output
    assert "validate: just wave-3-pda-runtime-evidence-validate" in output
    assert "W6-retro" in output
    assert "ignored:" in output


def test_wave6_text_report_prints_readiness_commands_for_missing_evidence(
    monkeypatch,
    capsys,
):
    """人类可读报告必须给出需要前置 readiness 的 evidence gate 完整命令链。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind h2": (
                "docs/retros/wave-1-h2-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only"]) == 1
    output = capsys.readouterr().out

    assert "W6.A-wave1-h2-runtime" in output
    assert "readiness: just wave-1-runtime-prereq-h2" in output
    assert "readiness: just wave-1-h2-runtime-readiness" in output
    assert "record: just wave-1-h2-runtime-evidence" in output
    assert "validate: just wave-1-runtime-evidence-validate" in output


def test_wave6_text_report_prints_w6h_deploy_audit_before_record(
    monkeypatch,
    capsys,
):
    """人类可读报告必须提示 W6.H 先写 deploy audit_event，再写 evidence JSON。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave6_deploy_evidence.py": (
                "docs/retros/wave-6-deploy-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only"]) == 1
    output = capsys.readouterr().out

    audit = output.index("record: just wave-6-deploy-audit")
    record = output.index("record: just wave-6-deploy-evidence-record")
    validate = output.index("validate: just wave-6-deploy-evidence-validate")
    assert audit < record < validate


def test_wave6_text_report_marks_w1d_rollback_as_deployment_choice(
    monkeypatch,
    capsys,
):
    """普通文本报告也必须提示 W6.B rollback 按部署形态二选一。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()
    title_index = next(
        index
        for index, line in enumerate(output_lines)
        if "W6.B-wave1-rollback-runtime" in line
    )
    choice_index = output_lines.index(
        f"    {report.W6B_ROLLBACK_DEPLOYMENT_CHOICE_LINE}"
    )
    readiness_index = output_lines.index(
        "    readiness: just wave-1-runtime-prereq-rollback-k8s"
    )

    assert title_index < choice_index < readiness_index
    assert output_lines.count(
        f"    {report.W6B_ROLLBACK_DEPLOYMENT_CHOICE_LINE}"
    ) == 1
    assert "    readiness: just wave-1-rollback-runtime-readiness-compose" in output_lines
    assert "    record: just wave-1-rollback-runtime-evidence-compose" in output_lines


def test_wave6_text_report_groups_w1d_rollback_paths(monkeypatch, capsys):
    """普通文本报告必须和 commands-only 一样分组展示 W6.B rollback 路径。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    k8s_index = output_lines.index("    path: k8s")
    compose_index = output_lines.index("    path: docker-compose")
    validate_index = output_lines.index("    validate: just wave-1-runtime-evidence-validate")

    assert output_lines.index(
        "    readiness: just wave-1-runtime-prereq-rollback-k8s",
    ) > k8s_index
    assert output_lines.index(
        "    record: just wave-1-rollback-runtime-evidence-k8s",
    ) > k8s_index
    assert output_lines.index(
        "    readiness: just wave-1-runtime-prereq-rollback-compose",
    ) > compose_index
    assert output_lines.index(
        "    record: just wave-1-rollback-runtime-evidence-compose",
    ) > compose_index
    assert k8s_index < compose_index < validate_index


def test_wave6_text_report_keeps_w1d_commands_inside_each_path_group(
    monkeypatch,
    capsys,
):
    """普通文本报告中 W6.B 命令必须落在对应 path 分组内。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    k8s_index = output_lines.index("    path: k8s")
    compose_index = output_lines.index("    path: docker-compose")
    validate_index = output_lines.index("    validate: just wave-1-runtime-evidence-validate")
    k8s_group = output_lines[k8s_index:compose_index]
    compose_group = output_lines[compose_index:validate_index]

    assert "    readiness: just wave-1-runtime-prereq-rollback-k8s" in k8s_group
    assert "    readiness: just wave-1-rollback-runtime-readiness-k8s" in k8s_group
    assert "    record: just wave-1-rollback-runtime-evidence-k8s" in k8s_group
    assert all("-compose" not in line for line in k8s_group)

    assert "    readiness: just wave-1-runtime-prereq-rollback-compose" in compose_group
    assert "    readiness: just wave-1-rollback-runtime-readiness-compose" in compose_group
    assert "    record: just wave-1-rollback-runtime-evidence-compose" in compose_group
    assert all("-k8s" not in line for line in compose_group)
