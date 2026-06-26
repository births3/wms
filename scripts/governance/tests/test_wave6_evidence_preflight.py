"""Wave 6 evidence preflight diff 触发与 runbook 合同测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave6_evidence_preflight_diff_rules_cover_scope_and_tooling_paths():
    """Wave 6 范围 / runbook / 工具链变更必须触发 evidence preflight。"""
    from _diff import load_gate_rules, match_rules

    changed = [
        "TODO.md",
        "ROADMAP.md",
        "docs/architecture-dependencies.md",
        "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
        "docs/runbooks/wave-1-runtime-evidence.md",
        "docs/runbooks/wave-2-runtime-evidence.md",
        "docs/runbooks/wave-3-pda-readiness.md",
        "docs/runbooks/wave-4-external-dependencies.md",
        "docs/runbooks/wave-5-hardware-evidence.md",
        "docs/runbooks/wave-5-tms-evidence.md",
        "docs/runbooks/wave-6-deploy-evidence.md",
        "docs/runbooks/wave-6-closeout.md",
        "docs/runbooks/wave-6-evidence-preflight.md",
        "justfile",
        "scripts/governance/check_wave6_evidence_preflight.py",
        "scripts/governance/report_wave6_pre_release.py",
    ]
    rules = [rule for rule in load_gate_rules() if rule.tier == "T1"]
    triggered = match_rules(changed, rules)

    assert set(changed) <= set(triggered.get("check_wave6_evidence_preflight", []))


def test_wave6_evidence_preflight_runbook_documents_json_contract():
    """Wave 6 preflight runbook 必须写明机器可消费 JSON 字段。"""
    text = Path("docs/runbooks/wave-6-evidence-preflight.md").read_text(
        encoding="utf-8",
    )

    for field in (
        "error_count",
        "top_error_count",
        "gate_error_count",
        "error_details",
        "top_error_details",
        "gate_error_details",
        "schema_version",
        "mode",
        "writes_runtime_evidence",
        "closes_gate",
        "preflight_command",
        "gate_count",
        "ok_gate_count",
        "failed_gate_count",
        "evidence_gate_ids",
        "evidence_gate_evidence_files",
        "evidence_gate_runbooks",
        "evidence_gate_just_entries",
        "evidence_gate_execution_files",
        "required_top_level_files",
        "required_runbooks",
        "required_execution_files",
        "overwrite_guard_execution_files",
        "overwrite_guard_required_markers",
        "gate_commands_by_phase",
        "validation_commands",
        "closeout_just_entries",
        "top_errors",
        "gate_errors",
        "failed_gate_ids",
        "failed_gates",
        "gate_specs",
        "gates",
    ):
        assert f"`{field}`" in text


def test_wave6_evidence_preflight_runbook_documents_commands_only_strict_exit():
    """preflight 执行顺序必须说明 missing-evidence-commands 缺 evidence 时会非零。"""
    text = Path("docs/runbooks/wave-6-evidence-preflight.md").read_text(
        encoding="utf-8",
    )

    assert "`just wave-6-missing-evidence-commands` 是只读命令清单" in text
    assert "`--strict --evidence-only` 返回非零" in text
    assert "不代表写入 runtime evidence 或关闭 gate" in text


def test_wave6_evidence_preflight_runbook_marks_w1d_as_deployment_choice():
    """preflight Gate 矩阵必须把 W6.B rollback 写成部署形态二选一。"""
    text = Path("docs/runbooks/wave-6-evidence-preflight.md").read_text(
        encoding="utf-8",
    )
    w6b_line = next(
        line for line in text.splitlines()
        if line.startswith("| W6.B |")
    )

    assert "二选一" in w6b_line
    assert "k8s 路径" in w6b_line
    assert "docker-compose 路径" in w6b_line
    assert "共用 `just wave-1-runtime-evidence-validate`" in w6b_line


def test_wave6_evidence_preflight_runbook_lists_w6d_readiness():
    """W6.D 矩阵必须把 PDA readiness 放在 record 前。"""
    text = Path("docs/runbooks/wave-6-evidence-preflight.md").read_text(
        encoding="utf-8",
    )
    w6d_gate_line = next(
        line for line in text.splitlines()
        if (
            line.startswith("| W6.D |")
            and "wave-3-pda-runtime-evidence.json" in line
        )
    )
    w6d_resource_line = next(
        line for line in text.splitlines()
        if line.startswith("| W6.D |") and "真 PDA" in line
    )

    assert "just wave-3-pda-field-precheck-summary" in w6d_gate_line
    assert "just wave-3-pda-field-owner-gap-actions" in w6d_gate_line
    assert "just wave-3-pda-field-handoff-bundle" in w6d_gate_line
    assert "just wave-3-pda-runtime-readiness" in w6d_gate_line
    assert w6d_gate_line.index("just wave-3-pda-field-precheck-summary") < w6d_gate_line.index(
        "just wave-3-pda-field-owner-gap-actions"
    )
    assert w6d_gate_line.index("just wave-3-pda-field-owner-gap-actions") < w6d_gate_line.index(
        "just wave-3-pda-field-handoff-bundle"
    )
    assert w6d_gate_line.index("just wave-3-pda-field-handoff-bundle") < w6d_gate_line.index(
        "just wave-3-pda-runtime-readiness"
    )
    assert w6d_gate_line.index("just wave-3-pda-runtime-readiness") < w6d_gate_line.index(
        "just wave-3-pda-runtime-evidence-record"
    )
    assert "idempotency replay 日志" in w6d_resource_line
    assert "L7 执行记录" in w6d_resource_line


def test_wave6_evidence_preflight_text_output_declares_static_boundary(capsys):
    """Wave 6 preflight 文本输出不能让人误读为真实证据已关闭。"""
    import check_wave6_evidence_preflight as check

    assert check.main([]) == 0
    output = capsys.readouterr().out

    assert "静态预检通过" in output
    assert "不会写入 runtime evidence" in output
    assert "不能关闭 evidence gate" in output


def test_wave6_evidence_preflight_failure_output_declares_static_boundary(
    capsys,
    monkeypatch,
):
    """Wave 6 preflight 失败文本也必须声明不会写证据或关闭 gate。"""
    import check_wave6_evidence_preflight as check

    monkeypatch.setattr(check, "collect_results", lambda: (["缺少文件: TODO.md"], []))

    assert check.main([]) == 1
    output = capsys.readouterr().out

    assert "静态预检未通过" in output
    assert "不会写入 runtime evidence" in output
    assert "不能关闭 evidence gate" in output
