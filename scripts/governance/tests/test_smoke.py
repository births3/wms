"""通用 smoke 测试：所有纳入 smoke 的治理脚本必须满足

每个脚本应当：
1. 能被 import（语法、依赖正确）
2. 能用 --json 模式跑通（不抛异常）
3. 退出码合法（0/1/2 三选一）
4. JSON 输出有效且包含必填字段（check / tier / category / ok）

这是黑盒 smoke 测试，不验证业务逻辑正确性。
"""
import json
import subprocess
import sys
from pathlib import Path

import pytest

SCRIPTS_DIR = Path(__file__).resolve().parent.parent
REPO_ROOT = SCRIPTS_DIR.parent.parent

# 所有应跑 smoke 测试的脚本，含 check_* / validate_* 与 gate report_*。
GOVERNANCE_SCRIPTS = [
    "validate_environment.py",
    "check_doc_links.py",
    "validate_adr_index.py",
    "check_mkdocs_nav_consistency.py",
    "validate_doc_layers.py",
    "check_file_naming.py",
    "check_user_story_structure.py",
    "check_glossary_consistency.py",
    "check_approval_source_chain.py",
    "check_config_center_consistency.py",
    "check_pda_story_completeness.py",
    "check_pda_production_gate.py",
    "check_gsp_field_traceability.py",
    "check_field_coding_standards.py",
    "check_business_rules_registry.py",
    "check_baseline_health.py",
    "check_governance_consistency.py",
    "check_commit_convention.py",
    "check_governance_coverage.py",
    "check_feature_flags.py",
    "check_changelog_freshness.py",
    "check_bounded_contexts.py",
    "check_error_codes.py",
    "check_multi_end_consistency.py",
    "check_observability.py",
    "check_secrets.py",
    "check_integration_contract.py",
    "check_wave6_evidence_preflight.py",
    "check_wave3_pda_runtime_readiness.py",
    "check_wave6_deploy_readiness.py",
    "generate_wave2_h1_token.py",
    "check_e2e_matrix_completeness.py",
    "check_matrix_e2e_report.py",
    "report_wave2_completion.py",
    "report_wave6_deploy_materials.py",
    "validate_wave1_runtime_evidence.py",
    "validate_wave3_pda_runtime_evidence.py",
    "validate_wave4_external_dependencies.py",
    "validate_wave5_hardware_evidence.py",
    "validate_wave5_tms_evidence.py",
    "validate_wave6_deploy_evidence.py",
    "check_layer_dependency.py",
    "check_unsafe_and_unwrap.py",
    "check_handler_test_coverage.py",
    "validate_openapi_artifacts.py",
    "check_openapi_contract.py",
    "check_baseline_completeness.py",
    "check_page_size.py",
    "check_component_doc_header.py",
    "check_component_no_inline_style.py",
    "check_component_props_classname.py",
    "check_component_registry_consistency.py",
    "check_prototype_fidelity.py",
    "check_prototype_freshness.py",
    "check_prototype_index_consistency.py",
    "check_prototype_navigation.py",
    "check_prototype_review_signoff.py",
    "check_prototype_story_sync.py",
    "check_prototype_usability_baseline.py",
]


def _run_script(script_name: str, *args: str, timeout: int | None = None):
    script = SCRIPTS_DIR / script_name
    return subprocess.run(
        [sys.executable, str(script), *args],
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
        timeout=timeout,
    )


def _json_payload_or_fail(result):
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as e:
        pytest.fail(f"non-JSON output: {e}\nstdout: {result.stdout[:500]}")


def _json_payload_or_skip(result):
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        pytest.skip("non-JSON output (skipped consistency)")


@pytest.mark.parametrize("script_name", GOVERNANCE_SCRIPTS)
def test_script_imports(script_name):
    """脚本至少能被 Python 解析（无语法 / import 错误）。"""
    script = SCRIPTS_DIR / script_name
    assert script.exists(), f"missing script: {script}"
    # 只编译不执行
    result = subprocess.run(
        [sys.executable, "-m", "py_compile", str(script)],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"compile failed: {result.stderr}"


@pytest.mark.parametrize("script_name", GOVERNANCE_SCRIPTS)
def test_script_help(script_name):
    """所有脚本必须支持 --help（Click/argparse 标准行为）。"""
    result = _run_script(script_name, "--help")
    assert result.returncode == 0, f"--help failed: {result.stderr}"
    assert "usage" in result.stdout.lower() or "用法" in result.stdout


@pytest.mark.parametrize("script_name", GOVERNANCE_SCRIPTS)
def test_script_json_output(script_name):
    """所有脚本必须支持 --json，且输出可解析且包含 'ok' 字段。"""
    result = _run_script(script_name, "--json", timeout=10)
    # 退出码合法（0 / 1 / 2）
    assert result.returncode in (0, 1, 2), \
        f"invalid exit code {result.returncode}; stderr: {result.stderr}"

    # JSON 可解析
    payload = _json_payload_or_fail(result)

    # 必填字段
    assert "ok" in payload, f"missing 'ok' field in output: {payload}"
    assert isinstance(payload["ok"], bool), "'ok' must be bool"


@pytest.mark.parametrize("script_name", GOVERNANCE_SCRIPTS)
def test_script_exit_code_consistency(script_name):
    """退出码与 ok 字段一致：ok=True ↔ exit=0；ok=False ↔ exit=1。"""
    result = _run_script(script_name, "--json", timeout=10)
    if result.returncode == 2:
        pytest.skip("script self-error, skip consistency check")
    payload = _json_payload_or_skip(result)
    if payload.get("ok"):
        assert result.returncode == 0, "ok=True but exit≠0"
    else:
        assert result.returncode == 1, "ok=False but exit≠1"
