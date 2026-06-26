"""Wave 3 PDA evidence 导出模板测试。"""
from __future__ import annotations

import json
import shlex
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave3_pda_export_template_prints_vars_and_record_command(capsys):
    """导出模板必须输出 WAVE_3_PDA_* 变量和 check-only 命令。"""
    import record_wave3_pda_runtime_evidence as recorder

    result = recorder.main(["--export-template"])
    output = capsys.readouterr().out

    assert result == 0
    assert "WAVE_3_PDA_ENVIRONMENT='staging'" in output
    assert "WAVE_3_PDA_SERVICE_URL=''" in output
    assert "WAVE_3_PDA_PDA_MODEL=''" in output
    assert "WAVE_3_PDA_PDA_MODEL=Honeywell EDA52" not in output
    assert "WAVE_3_PDA_ANDROID_VERSION=''" in output
    assert "WAVE_3_PDA_SCAN_INPUT_METHOD=''" in output
    assert "WAVE_3_PDA_STACK_CANDIDATE='react-native'" in output
    assert "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL=''" in output
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY=''" in output
    assert "just wave-3-pda-materials-checklist --json" in output
    assert "just wave-3-pda-field-work-request" in output
    assert "just wave-3-pda-field-execution-summary --json" in output
    assert "\njust wave-3-pda-field-precheck-summary --from-env\n" in output
    assert "just wave-3-pda-field-precheck-summary --from-env --json" in output
    assert "\njust wave-3-pda-field-owner-gap-actions\n" in output
    assert "just wave-3-pda-field-owner-gap-actions --json" in output
    assert "just wave-3-pda-field-handoff-bundle --json" in output
    assert "just wave-3-pda-evidence-package-template" in output
    assert "just wave-3-pda-intake-template --json" in output
    assert "just wave-3-pda-intake-check --json" in output
    assert "just wave-3-pda-intake-record --json" in output
    assert "just wave-3-pda-service-precheck --from-env --json" in output
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in output
    assert "just wave-3-pda-runtime-readiness --from-env --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --check-only --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in output
    materials_checklist_index = output.index("just wave-3-pda-materials-checklist --json")
    field_work_request_index = output.index("just wave-3-pda-field-work-request")
    field_execution_summary_index = output.index(
        "just wave-3-pda-field-execution-summary --json",
    )
    field_precheck_summary_markdown_index = output.index(
        "\njust wave-3-pda-field-precheck-summary --from-env\n",
        field_execution_summary_index,
    )
    field_precheck_summary_json_index = output.index(
        "\njust wave-3-pda-field-precheck-summary --from-env --json\n",
        field_precheck_summary_markdown_index,
    )
    field_owner_gap_actions_markdown_index = output.index(
        "\njust wave-3-pda-field-owner-gap-actions\n",
        field_precheck_summary_json_index,
    )
    field_owner_gap_actions_json_index = output.index(
        "\njust wave-3-pda-field-owner-gap-actions --json\n",
        field_owner_gap_actions_markdown_index,
    )
    field_handoff_bundle_index = output.index(
        "\njust wave-3-pda-field-handoff-bundle --json\n",
        field_owner_gap_actions_json_index,
    )
    package_template_index = output.index("just wave-3-pda-evidence-package-template")
    intake_template_index = output.index("just wave-3-pda-intake-template --json")
    intake_check_index = output.index("just wave-3-pda-intake-check --json")
    service_precheck_index = output.index("just wave-3-pda-service-precheck")
    trace_code_precheck_index = output.index("just wave-3-pda-trace-code-openapi-precheck")
    readiness_index = output.index("just wave-3-pda-runtime-readiness --from-env --json")
    check_only_index = output.index(
        "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
    )
    formal_record_index = output.index(
        "\njust wave-3-pda-runtime-evidence-record --from-env --json\n",
        check_only_index,
    )
    intake_record_index = output.index(
        "\njust wave-3-pda-intake-record --json\n",
        formal_record_index,
    )
    validate_index = output.index("just wave-3-pda-runtime-evidence-validate")
    assert (
        materials_checklist_index
        < field_work_request_index
        < field_execution_summary_index
        < field_precheck_summary_markdown_index
        < field_precheck_summary_json_index
        < field_owner_gap_actions_markdown_index
        < field_owner_gap_actions_json_index
        < field_handoff_bundle_index
        < package_template_index
        < intake_template_index
        < intake_check_index
        < service_precheck_index
        < trace_code_precheck_index
        < readiness_index
        < check_only_index
        < formal_record_index
        < intake_record_index
        < validate_index
    )
    service_precheck_command = output[service_precheck_index:readiness_index]
    assert "--from-env" in service_precheck_command
    assert "--json" in service_precheck_command
    trace_code_command = output[trace_code_precheck_index:readiness_index]
    assert "--from-env" in trace_code_command
    assert "--json" in trace_code_command
    assert '--environment "$WAVE_3_PDA_ENVIRONMENT"' not in output
    assert '--service-url "$WAVE_3_PDA_SERVICE_URL"' not in output
    readiness_command = output[readiness_index:check_only_index]
    assert "--json" in readiness_command
    assert "WAVE_3_PDA_NATIVE_ARGS=()" not in output
    assert 'if [ "$WAVE_3_PDA_STACK_CANDIDATE" = "webview-capacitor" ]; then' not in output
    assert "WAVE_3_PDA_NATIVE_SHELL_REF=''" in output
    assert "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF=''" in output
    assert "--native-shell-ref \"$WAVE_3_PDA_NATIVE_SHELL_REF\"" in output
    assert "--native-scan-plugin-ref \"$WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF\"" in output
    assert output.count('"${WAVE_3_PDA_NATIVE_ARGS[@]}" \\') == 0
    assert "WAVE_3_PDA_FLAG_ARGS=()" not in output
    assert "WAVE_3_PDA_REAL_PDA_USED='false'" in output
    assert 'if [ "$WAVE_3_PDA_REAL_PDA_USED" = "true" ]; then' not in output
    assert output.count('"${WAVE_3_PDA_FLAG_ARGS[@]}"') == 0
    assert "WAVE_3_PDA_SPIKE_RESULT_REF=''" in output
    assert "docs/retros/wave-3-pda-runtime-evidence.json" not in output
    assert "xxxxx" not in output
    assert "W6.D" in output
    assert "Evidence refs must include environment, PDA asset, executed_at" in output
    assert "Save readiness --json output as a field precheck attachment" in output
    assert "cannot close W6.D" in output
    assert "Operator usability checklist belongs in WAVE_3_PDA_USABILITY_REVIEW_REF" in output
    assert "device grip, scan key reachability, scan feedback, offline prompts" in output
    assert "Do not invent local L7 thresholds" in output
    assert "Use docs/runbooks/wave-3-pda-readiness.md W6.D L7 and usability templates" in output
    assert "readiness and record/check-only read native refs through --from-env" in output
    assert "real PDA flags are controlled by WAVE_3_PDA_* boolean variables" in output
    assert "Normal closeout must not use --force" in output
    assert "Only use --force after backing up or confirming replacement" in output


def test_wave3_pda_check_only_without_required_args_still_fails(tmp_path):
    """check-only 未补齐参数时仍应失败，不可静默通过。"""
    import record_wave3_pda_runtime_evidence as recorder

    with pytest.raises(SystemExit) as exc_info:
        recorder.main(["--check-only", "--output", str(tmp_path / "wave-3-pda-runtime-evidence.json")])

    assert exc_info.value.code == 2


def test_wave3_pda_export_template_with_check_only_still_prints_template(capsys, tmp_path):
    """导出模式可叠加 check-only，仍只输出模板且不做实参校验。"""
    import record_wave3_pda_runtime_evidence as recorder

    result = recorder.main([
        "--check-only",
        "--export-template",
        "--output",
        str(tmp_path / "wave-3-pda-runtime-evidence.json"),
    ])
    output = capsys.readouterr().out

    assert result == 0
    assert "WAVE_3_PDA_STACK_CANDIDATE='react-native'" in output
    assert "just wave-3-pda-materials-checklist --json" in output
    assert "just wave-3-pda-field-work-request" in output
    assert "just wave-3-pda-field-execution-summary --json" in output
    assert "\njust wave-3-pda-field-precheck-summary --from-env\n" in output
    assert "just wave-3-pda-field-precheck-summary --from-env --json" in output
    assert "\njust wave-3-pda-field-owner-gap-actions\n" in output
    assert "just wave-3-pda-field-owner-gap-actions --json" in output
    assert "just wave-3-pda-field-handoff-bundle --json" in output
    assert "just wave-3-pda-evidence-package-template" in output
    assert "just wave-3-pda-intake-template --json" in output
    assert "just wave-3-pda-intake-check --json" in output
    assert "just wave-3-pda-intake-record --json" in output
    assert "just wave-3-pda-service-precheck --from-env --json" in output
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in output
    assert "just wave-3-pda-runtime-readiness --from-env --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --check-only --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in output
    assert "just wave-3-pda-runtime-evidence-validate" in output
    assert not (tmp_path / "wave-3-pda-runtime-evidence.json").exists()


def test_wave3_pda_export_template_can_be_sourced_with_space_values(tmp_path):
    """导出模板的单引号必须支持现场填写带空格的 PDA 型号和 Android 版本。"""
    import record_wave3_pda_runtime_evidence as recorder

    replacements = {
        "WAVE_3_PDA_SERVICE_URL": "https://wms-staging.internal",
        "WAVE_3_PDA_PDA_MODEL": "Honeywell EDA52",
        "WAVE_3_PDA_ANDROID_VERSION": "Android 11",
        "WAVE_3_PDA_SCAN_INPUT_METHOD": "physical-scan-key-intent",
        "WAVE_3_PDA_PDA_DEVICE_REF": "asset://wms-staging/pda/honeywell-eda52-01",
        "WAVE_3_PDA_SPIKE_RESULT_REF": (
            "s3://wms-staging-evidence/wave3/pda/spike-005-runtime.md"
        ),
        "WAVE_3_PDA_M2_SCAN_LOG_REF": "ci/staging/wave3-pda-m2-scan/123",
        "WAVE_3_PDA_M3_SCAN_LOG_REF": "ci/staging/wave3-pda-m3-scan/123",
        "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF": "ci/staging/wave3-pda-offline-replay/123",
        "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF": (
            "ci/staging/wave3-pda-idempotency-replay/123"
        ),
        "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF": "ci/staging/wave3-pda-audit-event/123",
        "WAVE_3_PDA_L7_RUN_REF": "ci/staging/wave3-pda-l7/123",
        "WAVE_3_PDA_USABILITY_REVIEW_REF": (
            "s3://wms-staging-evidence/wave3/pda/usability-review.md"
        ),
        "WAVE_3_PDA_REAL_PDA_USED": "true",
        "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED": "true",
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED": "true",
        "WAVE_3_PDA_AUDIT_EVENT_VERIFIED": "true",
        "WAVE_3_PDA_L7_REVIEW_COMPLETED": "true",
        "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED": "true",
    }
    export_lines: list[str] = []
    for line in recorder.EXPORT_TEMPLATE.splitlines():
        if not line.startswith("export WAVE_3_PDA_"):
            continue
        name = line.split("=", 1)[0].removeprefix("export ")
        if name in replacements:
            line = f"export {name}={shlex.quote(replacements[name])}"
        export_lines.append(line)

    env_file = tmp_path / "wave3-pda-env.sh"
    env_file.write_text("\n".join(export_lines) + "\n", encoding="utf-8")
    output_path = tmp_path / "wave-3-pda-runtime-evidence.json"
    script = f"""
set -euo pipefail
source {shlex.quote(str(env_file))}
python3 - <<'PY'
import json
import os
print(json.dumps({{
    "pda_model": os.environ["WAVE_3_PDA_PDA_MODEL"],
    "android_version": os.environ["WAVE_3_PDA_ANDROID_VERSION"],
}}, ensure_ascii=False))
PY
echo RECORDER_JSON_BEGIN
python3 scripts/governance/record_wave3_pda_runtime_evidence.py \\
  --from-env \\
  --check-only \\
  --json \\
  --output {shlex.quote(str(output_path))}
"""

    result = subprocess.run(
        ["bash", "-lc", script],
        check=False,
        capture_output=True,
        text=True,
        cwd=Path.cwd(),
    )

    assert result.returncode == 0, result.stderr
    env_output, recorder_output = result.stdout.split("RECORDER_JSON_BEGIN\n", 1)
    env_payload = json.loads(env_output.strip())
    recorder_payload = json.loads(recorder_output)
    assert env_payload == {
        "pda_model": "Honeywell EDA52",
        "android_version": "Android 11",
    }
    assert recorder_payload["ok"] is True
    assert recorder_payload["writes_runtime_evidence"] is False
    assert not output_path.exists()


def test_wave3_pda_export_package_template_prints_markdown_without_writing(capsys, tmp_path):
    """证据包模板只输出现场可填写 Markdown，不写 runtime evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    output_path = tmp_path / "wave-3-pda-runtime-evidence.json"

    result = recorder.main([
        "--export-package-template",
        "--output",
        str(output_path),
    ])
    output = capsys.readouterr().out

    assert result == 0
    assert not output_path.exists()
    assert "# W6.D PDA Runtime Evidence Package" in output
    assert "This package template is not runtime evidence JSON and cannot close W6.D." in output
    assert "## 1. Execution Metadata" in output
    assert "## 2. M2 Scan Evidence" in output
    assert "## 3. M3 Scan Evidence" in output
    assert "## 4. Offline Replay Evidence" in output
    assert "## 5. Idempotency-Key Replay Evidence" in output
    assert "## 6. H2 audit_event Query Evidence" in output
    assert "## 7. L7 Run Record" in output
    assert "## 8. Operator Usability Review" in output
    assert "## 9. Trace-code OpenAPI Precheck Attachment" in output
    assert "## 10. Evidence JSON Mapping" in output
    assert "## 11. Owner Actions" in output
    assert "业务方 / 资产负责人 / 设备方" in output
    assert "WAVE_3_PDA_PDA_DEVICE_REF" in output
    assert "can_write_runtime_evidence=false" in output
    assert "OpenAPI URL variable" in output
    assert "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL" in output
    assert "Precheck command" in output
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in output
    assert "Do not paste WAVE_3_PDA_TRACE_CODE_API_KEY" in output
    assert "WAVE_3_PDA_M2_SCAN_LOG_REF" in output
    assert "WAVE_3_PDA_USABILITY_REVIEW_REF" in output
    assert "WAVE_3_PDA_NATIVE_SHELL_REF" in output
    assert "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF" in output
    assert "do not set real_pda_used=true" in output


def test_wave3_pda_export_package_template_prints_json_without_writing(capsys, tmp_path):
    """证据包模板 JSON 结构化输出现场采集项，不写 runtime evidence。"""
    import json
    import record_wave3_pda_runtime_evidence as recorder

    output_path = tmp_path / "wave-3-pda-runtime-evidence.json"

    result = recorder.main([
        "--export-package-template",
        "--json",
        "--output",
        str(output_path),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert not output_path.exists()
    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-evidence-package-template"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output_path)
    sections = {section["id"]: section for section in payload["sections"]}
    assert "execution_metadata" in sections
    assert "m2_scan_evidence" in sections
    assert "m3_scan_evidence" in sections
    assert "offline_replay_evidence" in sections
    assert "idempotency_key_replay_evidence" in sections
    assert "audit_event_query_evidence" in sections
    assert "l7_run_record" in sections
    assert "operator_usability_review" in sections
    assert "trace_code_openapi_precheck_attachment" in sections
    assert "evidence_json_mapping" in sections
    assert "owner_actions" in sections
    assert "PDA asset ref" in sections["execution_metadata"]["fields"]
    assert "Evidence ref for WAVE_3_PDA_M2_SCAN_LOG_REF" in sections["m2_scan_evidence"]["fields"]
    assert "Evidence ref for WAVE_3_PDA_USABILITY_REVIEW_REF" in sections["operator_usability_review"]["fields"]
    assert "OpenAPI URL variable" in sections["trace_code_openapi_precheck_attachment"]["fields"]
    assert "Precheck output attachment ref" in sections["trace_code_openapi_precheck_attachment"]["fields"]
    assert "WAVE_3_PDA_NATIVE_SHELL_REF" in sections["evidence_json_mapping"]["fields"]
    assert "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF" in sections["evidence_json_mapping"]["fields"]
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY" not in payload["mapping_variables"]
    assert payload["mapping_variables"] == [
        "WAVE_3_PDA_PDA_DEVICE_REF",
        "WAVE_3_PDA_SPIKE_RESULT_REF",
        "WAVE_3_PDA_M2_SCAN_LOG_REF",
        "WAVE_3_PDA_M3_SCAN_LOG_REF",
        "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
        "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF",
        "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
        "WAVE_3_PDA_L7_RUN_REF",
        "WAVE_3_PDA_USABILITY_REVIEW_REF",
        "WAVE_3_PDA_NATIVE_SHELL_REF",
        "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
    ]
    assert "real_pda_used" in payload["blocked_flags_until_refs_present"]
    assert "physical_scan_key_verified" in payload["blocked_flags_until_refs_present"]
    field_precheck_attachment = json.loads(
        Path("docs/retros/wave-3-pda-field-precheck-2026-06-14.json").read_text(
            encoding="utf-8",
        ),
    )
    assert payload["owner_actions"] == field_precheck_attachment["owner_actions"]
    assert payload["record_gate_after_owner_actions"] == field_precheck_attachment[
        "record_gate_after_owner_actions"
    ]
    assert len(payload["owner_actions"]) == 7
    assert payload["owner_actions"][:3] == [
        {
            "owner": "业务方 / 资产负责人 / 设备方",
            "action": "提供真 PDA 设备资产信息",
            "required_env_vars": [
                "WAVE_3_PDA_PDA_MODEL",
                "WAVE_3_PDA_ANDROID_VERSION",
                "WAVE_3_PDA_PDA_DEVICE_REF",
            ],
            "acceptance": "PDA 资产引用必须是 asset://.../pda/...，并记录 Android 版本",
            "can_write_runtime_evidence": False,
        },
        {
            "owner": "PDA 技术验证负责人",
            "action": "确认实体扫码键或厂商扫码通道",
            "required_env_vars": [
                "WAVE_3_PDA_SCAN_INPUT_METHOD",
                "WAVE_3_PDA_SPIKE_RESULT_REF",
            ],
            "acceptance": "扫码输入方式必须包含 scan-key / KeyEvent / Intent / DataWedge 之一",
            "can_write_runtime_evidence": False,
        },
        {
            "owner": "测试执行人",
            "action": "用真 PDA 采集 M2/M3 scan 与 offline replay 日志",
            "required_env_vars": [
                "WAVE_3_PDA_M2_SCAN_LOG_REF",
                "WAVE_3_PDA_M3_SCAN_LOG_REF",
                "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
            ],
            "acceptance": "日志引用必须包含 staging 或 dev、wave3-pda 场景名和 run ID",
            "can_write_runtime_evidence": False,
        },
    ]
    assert "readiness --json output is only a field precheck attachment" in payload["warnings"]
    assert "trace-code OpenAPI precheck output is only a preparation attachment" in payload["warnings"]
    assert "Do not paste WAVE_3_PDA_TRACE_CODE_API_KEY into evidence packages." in payload["warnings"]


def _valid_wave3_pda_env() -> dict[str, str]:
    return {
        "WAVE_3_PDA_ENVIRONMENT": "staging",
        "WAVE_3_PDA_PDA_MODEL": "Honeywell EDA52",
        "WAVE_3_PDA_ANDROID_VERSION": "Android 11",
        "WAVE_3_PDA_SCAN_INPUT_METHOD": "physical-scan-key-intent",
        "WAVE_3_PDA_STACK_CANDIDATE": "react-native",
        "WAVE_3_PDA_PDA_DEVICE_REF": "asset://wms-staging/pda/honeywell-eda52-01",
        "WAVE_3_PDA_SPIKE_RESULT_REF": "s3://wms-staging-evidence/wave3/pda/spike-005-runtime.md",
        "WAVE_3_PDA_M2_SCAN_LOG_REF": "ci/staging/wave3-pda-m2-scan/123",
        "WAVE_3_PDA_M3_SCAN_LOG_REF": "ci/staging/wave3-pda-m3-scan/123",
        "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF": "ci/staging/wave3-pda-offline-replay/123",
        "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF": (
            "ci/staging/wave3-pda-idempotency-replay/123"
        ),
        "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF": "ci/staging/wave3-pda-audit-event/123",
        "WAVE_3_PDA_L7_RUN_REF": "ci/staging/wave3-pda-l7/123",
        "WAVE_3_PDA_USABILITY_REVIEW_REF": (
            "s3://wms-staging-evidence/wave3/pda/usability-review.md"
        ),
        "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED": "50",
        "WAVE_3_PDA_M2_OPERATIONS_EXERCISED": "1",
        "WAVE_3_PDA_M3_OPERATIONS_EXERCISED": "1",
        "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED": "50",
        "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED": "50",
        "WAVE_3_PDA_REAL_PDA_USED": "true",
        "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED": "true",
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED": "true",
        "WAVE_3_PDA_AUDIT_EVENT_VERIFIED": "true",
        "WAVE_3_PDA_L7_REVIEW_COMPLETED": "true",
        "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED": "true",
    }


def test_wave3_pda_record_from_env_check_only_uses_exported_variables(
    monkeypatch,
    capsys,
    tmp_path,
):
    """from-env 应复用 export-template 变量，check-only 不写 evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    for key, value in _valid_wave3_pda_env().items():
        monkeypatch.setenv(key, value)

    output_path = tmp_path / "wave-3-pda-runtime-evidence.json"
    result = recorder.main([
        "--from-env",
        "--check-only",
        "--json",
        "--output",
        str(output_path),
    ])
    payload = __import__("json").loads(capsys.readouterr().out)

    assert result == 0
    assert output_path.exists() is False
    assert payload["ok"] is True
    assert payload["writes_runtime_evidence"] is False
    assert "check-only passed" in payload["message"]


def test_wave3_pda_record_from_env_rejects_missing_required_env(monkeypatch):
    """from-env 缺少必填 WAVE_3_PDA_* 时必须失败，不可静默补默认。"""
    import record_wave3_pda_runtime_evidence as recorder

    monkeypatch.setenv("WAVE_3_PDA_ENVIRONMENT", "staging")

    with pytest.raises(SystemExit) as exc_info:
        recorder.main(["--from-env", "--check-only"])

    assert exc_info.value.code == 2


def test_wave3_pda_record_from_env_rejects_invalid_boolean(monkeypatch):
    """from-env 布尔变量拼错时必须失败，避免误当 false。"""
    import record_wave3_pda_runtime_evidence as recorder

    for key, value in _valid_wave3_pda_env().items():
        monkeypatch.setenv(key, value)
    monkeypatch.setenv("WAVE_3_PDA_REAL_PDA_USED", "TRUEE")

    with pytest.raises(SystemExit) as exc_info:
        recorder.main(["--from-env", "--check-only"])

    assert exc_info.value.code == 2


def test_wave3_pda_record_from_env_check_only_reports_false_flag_owners(
    monkeypatch,
    capsys,
    tmp_path,
):
    """字段已齐但确认布尔项未置 true 时，应直接输出变量和负责人。"""
    import record_wave3_pda_runtime_evidence as recorder

    for key, value in _valid_wave3_pda_env().items():
        monkeypatch.setenv(key, value)
    monkeypatch.setenv("WAVE_3_PDA_REAL_PDA_USED", "false")
    monkeypatch.setenv("WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED", "false")

    output_path = tmp_path / "wave-3-pda-runtime-evidence.json"
    result = recorder.main([
        "--from-env",
        "--check-only",
        "--json",
        "--output",
        str(output_path),
    ])
    payload = __import__("json").loads(capsys.readouterr().out)

    assert result == 1
    assert output_path.exists() is False
    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["false_flag_env_vars"] == [
        "WAVE_3_PDA_REAL_PDA_USED",
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
    ]
    assert payload["false_flag_env_var_owners"] == [
        {
            "env_var": "WAVE_3_PDA_REAL_PDA_USED",
            "source_owner": "现场负责人",
            "no_pda_stage": "blocked_until_real_scan",
            "requires_real_pda": True,
            "evidence_requirement": "PDA 资产引用",
        },
        {
            "env_var": "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
            "source_owner": "运维 / 部署负责人",
            "no_pda_stage": "preparable",
            "requires_real_pda": False,
            "evidence_requirement": "dev/staging M2/M3 API",
        },
    ]


def test_wave3_pda_record_from_env_supports_webview_native_refs(
    monkeypatch,
    capsys,
    tmp_path,
):
    """from-env 在 WebView/Capacitor 候选下必须读取 native refs。"""
    import record_wave3_pda_runtime_evidence as recorder

    env = _valid_wave3_pda_env()
    env.update({
        "WAVE_3_PDA_STACK_CANDIDATE": "webview-capacitor",
        "WAVE_3_PDA_SPIKE_RESULT_REF": (
            "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime.md"
        ),
        "WAVE_3_PDA_NATIVE_SHELL_REF": (
            "ci/staging/wave3-pda-native-shell/123"
        ),
        "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF": (
            "ci/staging/wave3-pda-native-scan-plugin/123"
        ),
    })
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    result = recorder.main([
        "--from-env",
        "--check-only",
        "--json",
        "--output",
        str(tmp_path / "wave-3-pda-runtime-evidence.json"),
    ])
    payload = __import__("json").loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
