"""Wave 3 PDA runtime readiness 预检测试。"""
import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _valid_args() -> list[str]:
    return [
        "--environment",
        "staging",
        "--service-url",
        "http://wms-staging.internal",
        "--pda-model",
        "Honeywell EDA52",
        "--android-version",
        "Android 11",
        "--scan-input-method",
        "physical-scan-key-intent",
        "--pda-stack-candidate",
        "react-native",
        "--pda-device-ref",
        "asset://wms-staging/pda/honeywell-eda52-01",
        "--spike005-result-ref",
        "s3://wms-staging-evidence/wave3/pda/spike-005-runtime-20260604.md",
        "--m2-scan-log-ref",
        "ci/staging/wave3-pda-m2-scan/123",
        "--m3-scan-log-ref",
        "ci/staging/wave3-pda-m3-scan/123",
        "--offline-replay-log-ref",
        "ci/staging/wave3-pda-offline-replay/123",
        "--idempotency-replay-log-ref",
        "ci/staging/wave3-pda-idempotency-replay/123",
        "--audit-event-query-ref",
        "ci/staging/wave3-pda-audit-event/123",
        "--l7-run-ref",
        "ci/staging/wave3-pda-l7/123",
        "--usability-review-ref",
        "s3://wms-staging-evidence/wave3/pda/usability-review.md",
        "--barcode-samples-scanned",
        "50",
        "--m2-operations-exercised",
        "1",
        "--m3-operations-exercised",
        "1",
        "--offline-replays-exercised",
        "50",
        "--idempotency-replays-exercised",
        "50",
        "--real-pda-used",
        "--physical-scan-key-verified",
        "--dev-or-staging-service-verified",
        "--audit-event-verified",
        "--l7-review-completed",
        "--usability-review-completed",
    ]


def _valid_wave3_pda_env() -> dict[str, str]:
    return {
        "WAVE_3_PDA_ENVIRONMENT": "staging",
        "WAVE_3_PDA_SERVICE_URL": "http://wms-staging.internal",
        "WAVE_3_PDA_PDA_MODEL": "Honeywell EDA52",
        "WAVE_3_PDA_ANDROID_VERSION": "Android 11",
        "WAVE_3_PDA_SCAN_INPUT_METHOD": "physical-scan-key-intent",
        "WAVE_3_PDA_STACK_CANDIDATE": "react-native",
        "WAVE_3_PDA_PDA_DEVICE_REF": "asset://wms-staging/pda/honeywell-eda52-01",
        "WAVE_3_PDA_SPIKE_RESULT_REF": (
            "s3://wms-staging-evidence/wave3/pda/spike-005-runtime-20260604.md"
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


def _expected_next_commands() -> list[str]:
    return [
        "just wave-3-pda-preaudit-kit --json",
        "just wave-3-pda-materials-checklist --json",
        "just wave-3-pda-field-work-request",
        "just wave-3-pda-field-execution-summary --json",
        "just wave-3-pda-field-precheck-summary --from-env",
        "just wave-3-pda-field-precheck-summary --from-env --json",
        "just wave-3-pda-field-owner-gap-actions",
        "just wave-3-pda-field-owner-gap-actions --json",
        "just wave-3-pda-field-handoff-bundle --json",
        "just wave-3-pda-evidence-package-template",
        "just wave-3-pda-intake-template --json",
        "just wave-3-pda-intake-check --json",
        "just wave-3-pda-intake-record --json",
        "just wave-3-pda-service-precheck --from-env --json",
        "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
        "just wave-3-pda-runtime-evidence-record --export-template",
        "just wave-3-pda-runtime-readiness --from-env --json",
        "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
        "just wave-3-pda-runtime-evidence-record --from-env --json",
        "just wave-3-pda-runtime-evidence-validate",
    ]


def _valid_trace_code_openapi_yaml() -> str:
    return """
openapi: 3.0.3
info:
  title: 药品追溯码库 WMS 外部接口
  version: 1.0.0
servers:
  - url: http://43.128.77.47:9100
paths:
  /api/codes/{code}:
    get: {}
  /api/codes/{code}/children:
    get: {}
  /api/codes/batch:
    post: {}
  /api/codes/verify:
    post: {}
  /api/wms-products:
    post: {}
components:
  securitySchemes:
    ApiKeyAuth:
      type: apiKey
      in: header
      name: X-API-Key
"""


def test_wave3_pda_field_handoff_bundle_reuses_precheck_attachment_in_preaudit(
    monkeypatch,
    tmp_path,
    capsys,
):
    """field-handoff-bundle 内各摘要应一致复用脱敏前置附件。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    attachment = tmp_path / "wave-3-pda-field-precheck.json"
    attachment.write_text(
        json.dumps(
            {
                "kind": "wave3-pda-field-precheck-attachment",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "runtime_evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
                "service_precheck": {
                    "ok": True,
                    "environment": "staging",
                    "service_url": "http://wms-staging.internal:18080",
                    "healthz_status": 200,
                    "healthz_payload_status": "ok",
                    "wave3_route_status": 401,
                    "wave3_route_error_code": "AUTH-001",
                },
                "trace_code_openapi_precheck": {
                    "ok": True,
                    "status": 200,
                    "openapi": "3.0.3",
                    "api_key_header_name": "X-API-Key",
                    "required_paths_present": [
                        "/api/codes/{code}",
                        "/api/codes/{code}/children",
                        "/api/codes/batch",
                        "/api/codes/verify",
                        "/api/wms-products",
                    ],
                },
            },
        ),
        encoding="utf-8",
    )

    assert readiness.main(
        [
            "--field-handoff-bundle",
            "--field-precheck-attachment",
            str(attachment),
            "--json",
        ],
    ) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    expected_satisfied = [
        "WAVE_3_PDA_ENVIRONMENT",
        "WAVE_3_PDA_SERVICE_URL",
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "WAVE_3_PDA_TRACE_CODE_API_KEY",
    ]
    preaudit_status = payload["preaudit_kit"]["current_env_status"]
    field_summary_status = payload["field_execution_summary"]["current_env_status"]
    assert preaudit_status["missing_now_env_vars"] == []
    assert preaudit_status["satisfied_by_precheck_attachment_env_vars"] == (
        expected_satisfied
    )
    assert field_summary_status["missing_now_env_vars"] == []
    assert field_summary_status["satisfied_by_precheck_attachment_env_vars"] == (
        expected_satisfied
    )
    assert payload["ready_for_record_from_env_vars"] is False

def test_wave3_pda_field_handoff_bundle_can_write_sanitized_handoff_file(
    monkeypatch,
    tmp_path,
    capsys,
):
    """handoff bundle 可写入脱敏交接包文件，但不能写 runtime evidence。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    attachment = tmp_path / "wave-3-pda-field-precheck.json"
    handoff_output = tmp_path / "handoff" / "wave-3-pda-field-handoff.json"
    attachment.write_text(
        json.dumps(
            {
                "kind": "wave3-pda-field-precheck-attachment",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "runtime_evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
                "service_precheck": {
                    "ok": True,
                    "environment": "staging",
                    "service_url": "http://wms-staging.internal:18080",
                    "healthz_status": 200,
                    "healthz_payload_status": "ok",
                    "wave3_route_status": 401,
                    "wave3_route_error_code": "AUTH-001",
                },
                "trace_code_openapi_precheck": {
                    "ok": True,
                    "status": 200,
                    "openapi": "3.0.3",
                    "api_key_header_name": "X-API-Key",
                    "required_paths_present": [
                        "/api/codes/{code}",
                        "/api/codes/{code}/children",
                        "/api/codes/batch",
                        "/api/codes/verify",
                        "/api/wms-products",
                    ],
                    "required_operations_present": [
                        "GET /api/codes/{code}",
                        "GET /api/codes/{code}/children",
                        "POST /api/codes/batch",
                        "POST /api/codes/verify",
                        "POST /api/wms-products",
                    ],
                },
            },
        ),
        encoding="utf-8",
    )

    assert readiness.main(
        [
            "--field-handoff-bundle",
            "--field-precheck-attachment",
            str(attachment),
            "--field-handoff-output",
            str(handoff_output),
            "--json",
        ],
    ) == 0
    stdout_payload = json.loads(capsys.readouterr().out)
    file_payload = json.loads(handoff_output.read_text(encoding="utf-8"))

    assert stdout_payload["field_handoff_output"] == str(handoff_output)
    assert file_payload["field_handoff_output"] == str(handoff_output)
    assert file_payload["writes_runtime_evidence"] is False
    assert file_payload["closes_gate"] is False
    assert file_payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert file_payload["preaudit_kit"]["current_env_status"][
        "missing_now_env_vars"
    ] == []
    assert file_payload["ready_for_record_from_env_vars"] is False
    assert file_payload["real_pda_missing_env_vars_count"] > 0

def test_wave3_pda_field_handoff_bundle_rejects_overwrite_without_force(
    tmp_path,
    capsys,
):
    """field handoff 归档文件已存在时必须显式 force，避免覆盖现场附件。"""
    import check_wave3_pda_runtime_readiness as readiness

    handoff_output = tmp_path / "handoff" / "wave-3-pda-field-handoff.json"
    handoff_output.parent.mkdir(parents=True)
    handoff_output.write_text('{"existing": true}\n', encoding="utf-8")

    assert readiness.main([
        "--field-handoff-bundle",
        "--field-handoff-output",
        str(handoff_output),
        "--json",
    ]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["writes_field_handoff_bundle"] is False
    assert "already exists; pass --field-handoff-force to overwrite" in payload[
        "message"
    ]
    assert json.loads(handoff_output.read_text(encoding="utf-8")) == {"existing": True}

def test_wave3_pda_field_handoff_bundle_force_overwrites_handoff_file(
    tmp_path,
    capsys,
):
    """field handoff 明确 force 后才可覆盖已有交接包。"""
    import check_wave3_pda_runtime_readiness as readiness

    handoff_output = tmp_path / "handoff" / "wave-3-pda-field-handoff.json"
    handoff_output.parent.mkdir(parents=True)
    handoff_output.write_text('{"existing": true}\n', encoding="utf-8")

    assert readiness.main([
        "--field-handoff-bundle",
        "--field-handoff-output",
        str(handoff_output),
        "--field-handoff-force",
        "--json",
    ]) == 0
    stdout_payload = json.loads(capsys.readouterr().out)
    file_payload = json.loads(handoff_output.read_text(encoding="utf-8"))

    assert stdout_payload["ok"] is True
    assert stdout_payload["writes_runtime_evidence"] is False
    assert stdout_payload["closes_gate"] is False
    assert stdout_payload["writes_field_handoff_bundle"] is True
    assert "wrote" in stdout_payload["message"]
    assert file_payload["mode"] == "wave3-pda-field-handoff-bundle"
    assert file_payload["writes_field_handoff_bundle"] is True
    assert "existing" not in file_payload

def test_wave3_pda_field_handoff_bundle_exports_markdown(capsys):
    """field-handoff-bundle 默认 Markdown 适合现场转发，且声明不能关门禁。"""
    import check_wave3_pda_runtime_readiness as readiness

    assert readiness.main(["--field-handoff-bundle"]) == 0
    output = capsys.readouterr().out

    assert "# W6.D PDA Field Handoff Bundle" in output
    assert "This bundle is read-only. It does not write runtime evidence and cannot close W6.D." in output
    assert "writes_runtime_evidence=false" in output
    assert "closes_gate=false" in output
    assert "include_precheck=false" in output
    assert "## Bundle Scope" in output
    assert "`preaudit_kit`" in output
    assert "`materials_checklist`" in output
    assert "`field_owner_gap_actions`" in output
    assert "`evidence_package_template`" in output
    assert "`intake_template`" in output
    assert "ready_for_record_from_env_vars=" in output
    assert "| Owner | Missing now | Real evidence vars | False flags |" in output
    assert "section_count=" in output
    assert "owner_action_count=" in output
    assert "## Intake Template" in output
    assert "intake_mode=wave3-pda-runtime-evidence-intake-template" in output
    assert "intake_kind=wave3-pda-runtime-evidence-intake" in output
    assert "intake_writes_runtime_evidence=false" in output
    assert "intake_closes_gate=false" in output
    assert "just wave-3-pda-field-handoff-bundle --from-env --json" in output
    assert "just wave-3-pda-intake-template --json" in output
    assert "just wave-3-pda-intake-check --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in output

def test_wave3_pda_trace_code_openapi_precheck_from_env_validates_contract_without_key_leak(
    monkeypatch,
    capsys,
):
    """trace-code OpenAPI 预检只读验证合约，不能打印 API key 或关闭 W6.D。"""
    import check_wave3_pda_runtime_readiness as readiness

    secret = "wms_secret_should_not_leak"
    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", secret)

    calls = []

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        calls.append((url, api_key, timeout_seconds))
        return type("HttpText", (), {
            "status": 200,
            "text": _valid_trace_code_openapi_yaml(),
        })()

    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
        "--json",
    ]) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert secret not in output
    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-trace-code-openapi-precheck"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert payload["facts"]["status"] == 200
    assert payload["facts"]["openapi"] == "3.0.3"
    assert payload["facts"]["title"] == "药品追溯码库 WMS 外部接口"
    assert payload["facts"]["required_paths_present"] == [
        "/api/codes/{code}",
        "/api/codes/{code}/children",
        "/api/codes/batch",
        "/api/codes/verify",
        "/api/wms-products",
    ]
    assert payload["facts"]["required_operations_present"] == [
        "GET /api/codes/{code}",
        "GET /api/codes/{code}/children",
        "POST /api/codes/batch",
        "POST /api/codes/verify",
        "POST /api/wms-products",
    ]
    assert payload["facts"]["api_key_header_name"] == "X-API-Key"
    assert payload["next_commands"] == _expected_next_commands()
    assert calls == [
        (
            "http://trace-code.internal/openapi/wms-openapi.yaml",
            secret,
            10,
        ),
    ]

def test_wave3_pda_trace_code_openapi_precheck_rejects_wrong_method(
    monkeypatch,
    capsys,
):
    """路径存在但 GET/POST method 漂移时，预检必须失败。"""
    import check_wave3_pda_runtime_readiness as readiness

    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")

    wrong_method_yaml = _valid_trace_code_openapi_yaml().replace(
        "  /api/codes/batch:\n    post: {}",
        "  /api/codes/batch:\n    get: {}",
    )

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        return readiness.HttpTextResult(200, wrong_method_yaml)

    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
        "--json",
    ]) == 1
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert "wms_secret_should_not_leak" not in output
    assert payload["ok"] is False
    assert payload["facts"]["required_paths_present"] == [
        "/api/codes/{code}",
        "/api/codes/{code}/children",
        "/api/codes/batch",
        "/api/codes/verify",
        "/api/wms-products",
    ]
    assert "POST /api/codes/batch operation is required" in payload["issues"]

def test_wave3_pda_trace_code_openapi_precheck_rejects_version_drift(
    monkeypatch,
    capsys,
):
    """OpenAPI 版本漂移时，预检必须失败。"""
    import check_wave3_pda_runtime_readiness as readiness

    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        return readiness.HttpTextResult(
            200,
            _valid_trace_code_openapi_yaml().replace(
                "openapi: 3.0.3",
                "openapi: 3.1.0",
            ),
        )

    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
        "--json",
    ]) == 1
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert "wms_secret_should_not_leak" not in output
    assert payload["ok"] is False
    assert payload["facts"]["openapi"] == "3.1.0"
    assert "OpenAPI version 3.0.3 is required" in payload["issues"]

def test_wave3_pda_trace_code_openapi_precheck_reports_proxy_troubleshooting_on_502(
    monkeypatch,
    capsys,
):
    """trace-code OpenAPI 502 时必须提示先排查代理，不泄露 key。"""
    import check_wave3_pda_runtime_readiness as readiness

    secret = "wms_secret_should_not_leak"
    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://43.128.77.47:9100/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", secret)

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        assert api_key == secret
        return readiness.HttpTextResult(502, "")

    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
        "--json",
    ]) == 1
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert secret not in output
    assert payload["ok"] is False
    assert payload["facts"]["status"] == 502
    assert "trace-code OpenAPI expected HTTP 200, got 502" in payload["issues"]
    assert payload["troubleshooting"][0] == (
        "HTTP 502 is often produced by the proxy path for this endpoint; "
        "verify direct no-proxy access before escalating the OpenAPI service."
    )
    assert any("NO_PROXY='*' no_proxy='*'" in tip for tip in payload["troubleshooting"])
    assert any("curl --noproxy '*'" in tip for tip in payload["troubleshooting"])
    assert any("43.128.77.47:9100" in tip for tip in payload["troubleshooting"])
    assert any("9200" in tip for tip in payload["troubleshooting"])

def test_wave3_pda_trace_code_openapi_precheck_text_reports_troubleshooting_on_failure(
    monkeypatch,
    capsys,
):
    """非 JSON 输出也要给现场可执行的 trace-code OpenAPI 排障提示。"""
    import check_wave3_pda_runtime_readiness as readiness

    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://43.128.77.47:9100/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        return readiness.HttpTextResult(502, "")

    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
    ]) == 1
    captured = capsys.readouterr()

    assert "wms_secret_should_not_leak" not in captured.out
    assert "wms_secret_should_not_leak" not in captured.err
    assert "FAIL trace-code-openapi: trace-code OpenAPI expected HTTP 200, got 502" in (
        captured.err
    )
    assert "TIP trace-code-openapi: HTTP 502 is often produced by the proxy path" in (
        captured.err
    )
    assert "TIP trace-code-openapi: If trace-code OpenAPI returns 502" in captured.err

def test_wave3_pda_trace_code_openapi_precheck_reports_missing_env_without_network(
    monkeypatch,
    capsys,
):
    """trace-code OpenAPI 预检缺变量时直接给负责人，不联网、不写 evidence。"""
    import check_wave3_pda_runtime_readiness as readiness

    monkeypatch.delenv("WAVE_3_PDA_TRACE_CODE_OPENAPI_URL", raising=False)
    monkeypatch.delenv("WAVE_3_PDA_TRACE_CODE_API_KEY", raising=False)

    def fail_http_text_with_api_key(url, api_key, timeout_seconds=10):
        raise AssertionError(f"missing trace-code env must not call network: {url}")

    monkeypatch.setattr(readiness, "http_text_with_api_key", fail_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
        "--json",
    ]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["mode"] == "wave3-pda-trace-code-openapi-precheck"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["missing_env_vars"] == [
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "WAVE_3_PDA_TRACE_CODE_API_KEY",
    ]
    assert payload["missing_env_var_owners"] == [
        {
            "env_var": "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
            "source_owner": "追溯码接口负责人 / 运维",
            "no_pda_stage": "preparable",
            "requires_real_pda": False,
            "evidence_requirement": "追溯码 OpenAPI 合约",
        },
        {
            "env_var": "WAVE_3_PDA_TRACE_CODE_API_KEY",
            "source_owner": "追溯码接口负责人 / 运维",
            "no_pda_stage": "preparable",
            "requires_real_pda": False,
            "evidence_requirement": "追溯码 OpenAPI 合约",
        },
    ]

@pytest.mark.parametrize(
    ("openapi_url", "secret", "message"),
    [
        (
            "https://trace:secret-value@43.128.77.47:9100/openapi/wms-openapi.yaml",
            "secret-value",
            "trace_code_openapi_url cannot contain userinfo credentials",
        ),
        (
            "http://43.128.77.47:9100/openapi/wms-openapi.yaml?signature=secret-value",
            "secret-value",
            "trace_code_openapi_url query cannot contain sensitive parameter: signature",
        ),
    ],
)
def test_wave3_pda_trace_code_openapi_precheck_rejects_url_secrets_without_network(
    monkeypatch,
    capsys,
    openapi_url,
    secret,
    message,
):
    """trace-code OpenAPI URL 不能携带 URL secret，失败输出也不能泄露。"""
    import check_wave3_pda_runtime_readiness as readiness

    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_OPENAPI_URL", openapi_url)
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")

    def fail_http_text_with_api_key(url, api_key, timeout_seconds=10):
        raise AssertionError(f"secret-bearing OpenAPI URL must not call network: {url}")

    monkeypatch.setattr(readiness, "http_text_with_api_key", fail_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
        "--json",
    ]) == 1
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert payload["ok"] is False
    assert message in payload["issues"]
    assert secret not in output
    assert "status" not in payload["facts"]
