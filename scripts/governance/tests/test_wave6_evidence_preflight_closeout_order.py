"""Wave 6 evidence preflight closeout 推荐执行顺序测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import (
    make_gate,
    write_files,
    write_single_gate_preflight_fixture,
)


def test_wave6_evidence_preflight_requires_closeout_record_before_validate(
    tmp_path,
    monkeypatch,
):
    """推荐执行顺序必须显式列出 record，再列 validate，避免现场跳过采集。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_text="\n".join([
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "```bash",
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            "```",
            "## 当前 Gate",
            "| W6.X | docs/retros/wave-x-evidence.json | wave-x-record | wave-x-validate |",
            "## 推荐执行顺序",
            "```bash",
            "# 按 runbook 的变量化 record 命令采集",
            "just wave-x-validate",
            "```",
        ]),
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results[0].ok is True
    joined_errors = " ".join(top_errors)
    assert "推荐执行顺序" in joined_errors
    assert "wave-x-record" in joined_errors
    assert "wave-x-validate" in joined_errors


def test_wave6_evidence_preflight_ignores_prose_when_checking_closeout_order(
    tmp_path,
    monkeypatch,
):
    """普通文字提到 record，不能替代 shell 代码块里的真实执行顺序。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_text="\n".join([
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "```bash",
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            "```",
            "## 当前 Gate",
            "| W6.X | docs/retros/wave-x-evidence.json | wave-x-record | wave-x-validate |",
            "## 推荐执行顺序",
            "先运行 just wave-x-record，再运行下面命令确认结果。",
            "```bash",
            "just wave-x-validate",
            "```",
        ]),
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results[0].ok is True
    joined_errors = " ".join(top_errors)
    assert "推荐执行顺序" in joined_errors
    assert "wave-x-record" in joined_errors
    assert "wave-x-validate" in joined_errors


def test_wave6_evidence_preflight_accepts_closeout_record_before_validate(
    tmp_path,
    monkeypatch,
):
    """推荐执行顺序中 record 在 validate 前时通过。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        closeout_text="\n".join([
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "```bash",
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            "```",
            "## 当前 Gate",
            "| W6.X | docs/retros/wave-x-evidence.json | wave-x-record | wave-x-validate |",
            "## 推荐执行顺序",
            "```bash",
            "just wave-x-record",
            "just wave-x-validate",
            "```",
        ]),
        execution_files=("scripts/governance/x.py",),
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results[0].ok is True
    assert top_errors == []


def test_wave6_closeout_w6d_order_lists_pda_field_preparation_commands():
    """W6.D 推荐执行顺序必须展开现场准备和服务前置命令。"""
    closeout = Path("docs/runbooks/wave-6-closeout.md").read_text(
        encoding="utf-8",
    )

    section_start = closeout.index("### 4. Wave 3 PDA / L7 evidence")
    section_end = closeout.index("### 5. Wave 4 M-TC", section_start)
    section = closeout[section_start:section_end]

    expected_order = [
        "just wave-3-pda-preaudit-kit --json",
        "just wave-3-pda-materials-checklist --json",
        "just wave-3-pda-field-work-request",
        "just wave-3-pda-field-execution-summary --json",
        "just wave-3-pda-service-precheck",
        "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
        "just wave-3-pda-field-precheck-summary --from-env --json",
        "just wave-3-pda-field-owner-gap-actions --json",
        "just wave-3-pda-field-handoff-bundle --json",
        "just wave-3-pda-evidence-package-template",
        "just wave-3-pda-intake-template --json",
        "just wave-3-pda-runtime-readiness --from-env --json",
        "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
        "just wave-3-pda-runtime-evidence-record --from-env --json",
        "just wave-3-pda-intake-check --json",
        "just wave-3-pda-intake-record --json",
        "just wave-3-pda-runtime-evidence-validate",
    ]

    indexes = [section.index(command) for command in expected_order]
    assert indexes == sorted(indexes)


def test_wave6_evidence_preflight_requires_w6h_deploy_materials_audit_and_readiness_order(
    tmp_path,
    monkeypatch,
):
    """W6.H closeout 必须先输出 materials，再 deploy audit，最后 readiness/record/validate。"""
    import check_wave6_evidence_preflight as check

    gate = make_gate(
        check,
        gate_id="W6.H",
        title="Wave 6 gray release evidence",
        runbook="docs/runbooks/wave-6-deploy-evidence.md",
        evidence_file="docs/retros/wave-6-deploy-evidence.json",
        just_entries=(
            "wave-6-deploy-materials",
            "wave-6-deploy-readiness",
            "wave-6-deploy-audit",
            "wave-6-deploy-evidence-record",
            "wave-6-deploy-evidence-validate",
        ),
    )

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        gate=gate,
        closeout_text="\n".join([
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "```bash",
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
            "```",
            "## 当前 Gate",
            (
                "| W6.H | docs/retros/wave-6-deploy-evidence.json | "
                "wave-6-deploy-materials | wave-6-deploy-readiness | "
                "wave-6-deploy-audit | wave-6-deploy-evidence-record | "
                "wave-6-deploy-evidence-validate |"
            ),
            "## 推荐执行顺序",
            "```bash",
            "just wave-6-deploy-materials",
            "just wave-6-deploy-audit --check-only",
            "just wave-6-deploy-audit",
            "just wave-6-deploy-readiness",
            "just wave-6-deploy-evidence-record --check-only --json",
            "just wave-6-deploy-evidence-record",
            "just wave-6-deploy-evidence-validate",
            "```",
        ]),
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
            "wave-6-deploy-materials:\n"
            "    @python3 scripts/governance/report_wave6_deploy_materials.py\n"
            "wave-6-deploy-readiness:\n"
            "    @python3 scripts/governance/check_wave6_deploy_readiness.py\n"
            "wave-6-deploy-audit:\n"
            "    @python3 scripts/governance/record_wave6_deploy_evidence.py --audit-only\n"
            "wave-6-deploy-evidence-record:\n"
            "    @python3 scripts/governance/record_wave6_deploy_evidence.py\n"
            "wave-6-deploy-evidence-validate:\n"
            "    @python3 scripts/governance/validate_wave6_deploy_evidence.py\n"
        ),
        execution_files=(
            "scripts/governance/report_wave6_deploy_materials.py",
            "scripts/governance/check_wave6_deploy_readiness.py",
            "scripts/governance/record_wave6_deploy_evidence.py",
            "scripts/governance/validate_wave6_deploy_evidence.py",
        ),
    )
    write_files(
        tmp_path,
        {
            "scripts/governance/record_wave6_deploy_evidence.py": (
                "force = False\n"
                "target.exists() and not force\n"
                "target.write_text('{}')\n"
                "--force\n"
                "already exists\n"
                "pass --force to overwrite\n"
            ),
        },
    )

    top_errors, gate_results = check.collect_results()

    assert gate_results[0].ok is True
    joined_errors = " ".join(top_errors)
    assert "W6.H" in joined_errors
    assert "just wave-6-deploy-materials --export-template" in joined_errors
    assert "just wave-6-deploy-materials --from-env --json" in joined_errors
