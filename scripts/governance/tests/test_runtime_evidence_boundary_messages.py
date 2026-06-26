"""runtime evidence 入口与边界文案测试。"""
import importlib.util
from pathlib import Path


def test_runtime_evidence_validators_are_in_smoke_suite():
    """Wave runtime evidence validator 必须纳入通用 smoke，避免接口漂移。"""
    spec = importlib.util.spec_from_file_location(
        "test_smoke_suite",
        Path("scripts/governance/tests/test_smoke.py"),
    )
    assert spec and spec.loader
    smoke = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(smoke)

    expected = {
        "report_wave2_completion.py",
        "validate_wave1_runtime_evidence.py",
        "validate_wave3_pda_runtime_evidence.py",
        "validate_wave4_external_dependencies.py",
        "validate_wave5_hardware_evidence.py",
        "validate_wave5_tms_evidence.py",
        "validate_wave6_deploy_evidence.py",
    }

    assert expected <= set(smoke.GOVERNANCE_SCRIPTS)


def test_runtime_runbook_summary_boundaries_include_production():
    """runtime / hardware / deploy runbook 的摘要边界不能只写 prod 漏掉 production。"""
    runbooks = [
        Path("docs/runbooks/wave-2-runtime-evidence.md"),
        Path("docs/runbooks/wave-4-external-dependencies.md"),
        Path("docs/runbooks/wave-5-hardware-evidence.md"),
        Path("docs/runbooks/wave-5-tms-evidence.md"),
        Path("docs/runbooks/wave-6-deploy-evidence.md"),
    ]

    checked_lines = []
    for path in runbooks:
        for line in path.read_text(encoding="utf-8").splitlines():
            normalized = line.replace("`", "")
            if "prod" not in normalized:
                continue
            if not (
                "不能指向" in normalized
                or "不得使用" in normalized
                or "不使用" in normalized
            ):
                continue
            checked_lines.append((path, line))
            assert "production" in normalized, f"{path}: {line}"

    assert checked_lines


def test_wave4_rejection_boundary_lists_prod_and_production():
    """Wave 4 外部依赖 runbook 拒绝边界必须列出 prod 和 production 引用。"""
    text = Path("docs/runbooks/wave-4-external-dependencies.md").read_text(
        encoding="utf-8",
    )
    rejection_section = text.split("## 拒绝边界", 1)[1]

    assert "`prod`" in rejection_section
    assert "`production`" in rejection_section


def test_runtime_evidence_script_boundary_messages_include_production():
    """runtime evidence 脚本的边界错误文案不能只写 prod 漏掉 production。"""
    scripts = [
        Path("scripts/governance/check_wave1_runtime_evidence_prereqs.py"),
        Path("scripts/governance/collect_wave1_h2_runtime_evidence.py"),
        Path("scripts/governance/report_wave1_completion.py"),
        Path("scripts/governance/report_wave2_completion.py"),
        Path("scripts/governance/validate_wave3_pda_runtime_evidence.py"),
        Path("scripts/governance/validate_wave4_external_dependencies.py"),
        Path("scripts/governance/validate_wave5_hardware_evidence.py"),
        Path("scripts/governance/validate_wave5_tms_evidence.py"),
        Path("scripts/governance/validate_wave6_deploy_evidence.py"),
    ]

    checked_lines = []
    for path in scripts:
        for line in path.read_text(encoding="utf-8").splitlines():
            normalized = line.replace("`", "")
            if "prod" not in normalized:
                continue
            if not (
                "不能" in normalized
                or "must not" in normalized
                or "not point" in normalized
            ):
                continue
            checked_lines.append((path, line))
            assert "production" in normalized, f"{path}: {line}"

    assert checked_lines
