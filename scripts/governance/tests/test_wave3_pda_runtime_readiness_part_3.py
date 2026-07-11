"""Wave 3 PDA runtime readiness 预检测试。"""
import json
import os
import subprocess
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
        "--spike-result-ref",
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


def test_wave3_pda_trace_code_openapi_precheck_rejects_incomplete_contract(
    monkeypatch,
    capsys,
):
    """trace-code OpenAPI 预检必须阻断缺路径或缺 X-API-Key scheme 的合约。"""
    import check_wave3_pda_runtime_readiness as readiness

    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        return type("HttpText", (), {
            "status": 200,
            "text": """
openapi: 3.0.3
info:
  title: incomplete
paths:
  /api/codes/{code}:
    get: {}
components:
  securitySchemes: {}
""",
        })()

    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main([
        "--trace-code-openapi-precheck",
        "--from-env",
        "--json",
    ]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert "/api/codes/batch path is required" in payload["issues"]
    assert "ApiKeyAuth header X-API-Key is required" in payload["issues"]

def test_wave3_pda_field_precheck_summary_combines_service_trace_and_field_status_without_key_leak(
    monkeypatch,
    capsys,
):
    """一键现场前置预检应组合服务、追溯码和字段摘要，但不能关闭 W6.D。"""
    import check_wave3_pda_runtime_readiness as readiness

    secret = "wms_secret_should_not_leak"
    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    monkeypatch.setenv("WAVE_3_PDA_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE_3_PDA_SERVICE_URL", "http://wms-staging.internal")
    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", secret)

    calls = []

    def fake_http_json(url, timeout_seconds=10):
        calls.append(("json", url, timeout_seconds))
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        calls.append(("text", url, api_key, timeout_seconds))
        return readiness.HttpTextResult(200, _valid_trace_code_openapi_yaml())

    monkeypatch.setattr(readiness, "http_json", fake_http_json)
    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main(["--field-precheck-summary", "--from-env", "--json"]) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert secret not in output
    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-field-precheck-summary"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert payload["service_precheck"]["ok"] is True
    assert payload["service_precheck"]["mode"] == "wave3-pda-service-precheck"
    assert payload["service_precheck"]["facts"]["healthz_status"] == 200
    assert payload["service_precheck"]["facts"]["wave3_route_error_code"] == "AUTH-001"
    assert payload["trace_code_openapi_precheck"]["ok"] is True
    assert payload["trace_code_openapi_precheck"]["facts"]["api_key_header_name"] == "X-API-Key"
    assert payload["field_execution_summary"]["mode"] == "wave3-pda-field-execution-summary"
    assert payload["field_execution_summary"]["ready_for_record_from_env_vars"] is False
    assert payload["no_pda_precheck_verified_flag_env_vars"] == [
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
    ]
    assert payload["no_pda_precheck_verified_flag_env_var_owners"] == [
        {
            "env_var": "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
            "source_owner": "运维 / 部署负责人",
            "no_pda_stage": "preparable",
            "requires_real_pda": False,
            "evidence_requirement": "dev/staging M2/M3 API",
        },
    ]
    assert payload["remaining_no_pda_precheck_false_flag_env_vars"] == []
    assert "WAVE_3_PDA_REAL_PDA_USED" in payload[
        "remaining_real_evidence_false_flag_env_vars"
    ]
    assert "WAVE_3_PDA_PDA_MODEL" in payload["field_execution_summary"][
        "real_pda_missing_env_vars"
    ]
    nested_real_pda_missing_owners = {
        item["env_var"]: item
        for item in payload["field_execution_summary"][
            "real_pda_missing_env_var_owners"
        ]
    }
    assert nested_real_pda_missing_owners["WAVE_3_PDA_PDA_MODEL"][
        "source_owner"
    ] == "设备借测 / 资产负责人"
    nested_false_truth_flag_owners = {
        item["env_var"]: item
        for item in payload["field_execution_summary"][
            "false_truth_flag_env_var_owners"
        ]
    }
    assert nested_false_truth_flag_owners["WAVE_3_PDA_REAL_PDA_USED"][
        "source_owner"
    ] == "现场负责人"
    assert payload["issues"] == []
    assert payload["next_commands"] == _expected_next_commands()
    assert calls == [
        ("json", "http://wms-staging.internal/healthz", 10),
        ("json", "http://wms-staging.internal/api/v1/inventory/batches", 10),
        ("text", "http://trace-code.internal/openapi/wms-openapi.yaml", secret, 10),
    ]

def test_wave3_pda_field_precheck_summary_exports_markdown_for_field_handoff(
    monkeypatch,
    capsys,
):
    """一键现场前置预检默认输出 Markdown，便于直接转发现场总览。"""
    import check_wave3_pda_runtime_readiness as readiness

    secret = "wms_secret_should_not_leak"
    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    monkeypatch.setenv("WAVE_3_PDA_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE_3_PDA_SERVICE_URL", "http://wms-staging.internal")
    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", secret)

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        return readiness.HttpTextResult(200, _valid_trace_code_openapi_yaml())

    monkeypatch.setattr(readiness, "http_json", fake_http_json)
    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main(["--field-precheck-summary", "--from-env"]) == 0
    output = capsys.readouterr().out

    assert secret not in output
    assert "# W6.D PDA Field Precheck Summary" in output
    assert "This summary is read-only and cannot close W6.D." in output
    assert "writes_runtime_evidence=false" in output
    assert "closes_gate=false" in output
    assert "evidence_file=docs/retros/wave-3-pda-runtime-evidence.json" in output
    assert "## Service Precheck" in output
    assert "service_precheck.ok=true" in output
    assert "healthz_status=200" in output
    assert "wave3_route_status=401" in output
    assert "wave3_route_error_code=AUTH-001" in output
    assert "## Trace-code OpenAPI Precheck" in output
    assert "trace_code_openapi_precheck.ok=true" in output
    assert "openapi=3.0.3" in output
    assert "api_key_header_name=X-API-Key" in output
    assert "missing_required_paths_count=0" in output
    assert "## Field Gaps" in output
    assert "ready_for_record_from_env_vars=false" in output
    assert "missing_now_env_vars_count=0" in output
    assert "real_pda_missing_env_vars_count=" in output
    assert "false_truth_flag_env_vars_count=" in output
    assert "## Precheck Verified Flags" in output
    assert "- `WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED`" in output
    assert "remaining_no_pda_precheck_false_flag_env_vars_count=0" in output
    assert "remaining_real_evidence_false_flag_env_vars_count=5" in output
    assert "## Issues" in output
    assert "- none" in output
    assert "## Commands" in output
    assert "just wave-3-pda-field-precheck-summary --from-env --json" in output
    assert "just wave-3-pda-field-owner-gap-actions" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in output

def test_wave3_pda_field_precheck_summary_markdown_lists_missing_now_owners_without_network(
    monkeypatch,
    capsys,
):
    """现场前置总览缺变量时应直接列出负责人，且不发起网络探测。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"missing env precheck must not call network: {url}")

    def fail_http_text_with_api_key(url, api_key, timeout_seconds=10):
        raise AssertionError(f"missing env precheck must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)
    monkeypatch.setattr(readiness, "http_text_with_api_key", fail_http_text_with_api_key)

    assert readiness.main(["--field-precheck-summary", "--from-env"]) == 1
    output = capsys.readouterr().out

    assert "# W6.D PDA Field Precheck Summary" in output
    assert "## Missing Now Env Vars" in output
    assert "- `WAVE_3_PDA_ENVIRONMENT`: 运维 / 部署负责人" in output
    assert "- `WAVE_3_PDA_SERVICE_URL`: 运维 / 部署负责人" in output
    assert "- `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL`: 追溯码接口负责人 / 运维" in output
    assert "- `WAVE_3_PDA_TRACE_CODE_API_KEY`: 追溯码接口负责人 / 运维" in output
    assert "writes_runtime_evidence=false" in output
    assert "closes_gate=false" in output

def test_wave3_pda_field_precheck_summary_returns_nonzero_with_structured_failures(
    monkeypatch,
    capsys,
):
    """一键现场前置预检失败时应聚合子检查问题，并仍输出字段缺口摘要。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    monkeypatch.setenv("WAVE_3_PDA_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE_3_PDA_SERVICE_URL", "http://wms-staging.internal")
    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "http://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(503, {"status": "down"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(404, {"code": "NOT_FOUND"})
        raise AssertionError(url)

    def fake_http_text_with_api_key(url, api_key, timeout_seconds=10):
        return readiness.HttpTextResult(
            200,
            """
openapi: 3.0.3
info:
  title: incomplete
paths:
  /api/codes/{code}:
    get: {}
components:
  securitySchemes: {}
""",
        )

    monkeypatch.setattr(readiness, "http_json", fake_http_json)
    monkeypatch.setattr(readiness, "http_text_with_api_key", fake_http_text_with_api_key)

    assert readiness.main(["--field-precheck-summary", "--from-env", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["mode"] == "wave3-pda-field-precheck-summary"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["service_precheck"]["ok"] is False
    assert "healthz expected HTTP 200, got 503" in payload["service_precheck"]["issues"]
    assert payload["trace_code_openapi_precheck"]["ok"] is False
    assert "/api/codes/batch path is required" in payload["trace_code_openapi_precheck"][
        "issues"
    ]
    assert payload["field_execution_summary"]["mode"] == "wave3-pda-field-execution-summary"
    assert payload["no_pda_precheck_verified_flag_env_vars"] == []
    assert payload["remaining_no_pda_precheck_false_flag_env_vars"] == [
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
    ]
    assert "service: healthz expected HTTP 200, got 503" in payload["issues"]
    assert "trace-code-openapi: /api/codes/batch path is required" in payload["issues"]
    assert payload["next_commands"] == _expected_next_commands()

def test_wave3_pda_field_work_request_exports_markdown_without_network(
    monkeypatch,
    capsys,
):
    """field-work-request 输出可转发资源申请包，不联网、不写 evidence、不关闭 W6.D。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"field work request must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--field-work-request"]) == 0
    output = capsys.readouterr().out

    assert "# W6.D PDA Field Work Request" in output
    assert "This request package is not runtime evidence JSON and cannot close W6.D." in output
    assert "| Resource | Owner | Deliverable | Verification / variable |" in output
    assert "At least one real PDA" in output
    assert "`asset://.../pda/...`" in output
    assert "`WAVE_3_PDA_SERVICE_URL`" in output
    assert "50 sanitized barcode samples" in output
    assert "M2/M3 test data" in output
    assert "L7 runner" in output
    assert "Operator usability reviewer" in output
    assert "just wave-3-pda-service-precheck" in output
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in output
    assert "just wave-3-pda-runtime-readiness --from-env --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --check-only --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in output
    assert "writes_runtime_evidence=false" in output
    assert "closes_gate=false" in output
    assert "## 中文现场工单表" in output
    assert "| 资源 | 负责人 | 交付物 | 验证变量 / 命令 |" in output
    assert "| 至少一台真 PDA | 业务方 / 资产负责人 / 设备方 |" in output
    assert "| 实体扫码键 / 厂商扫码通道 | PDA 技术验证负责人 |" in output
    assert "| 人工易用性走查人 | 业务走查人 / 测试负责人 |" in output
    assert "| 追溯码 OpenAPI 合约 | 追溯码接口负责人 / 运维 |" in output
    assert "## 中文执行顺序" in output
    assert "1. 运维 / 部署负责人提供 WAVE_3_PDA_SERVICE_URL" in output
    assert "2. 追溯码接口负责人 / 运维提供 OpenAPI URL 和 API key" in output
    assert "3. 业务方 / 资产负责人提供至少一台真 PDA" in output
    assert "8. 后端 / 数据库操作人归档 H2 audit_event 查询证据" in output
    assert "9. 测试负责人选择 from-env 或 intake 路径执行 check-only、正式 record 和 validate" in output
    assert "Normal closeout must not use --force" in output
    assert "keep the original evidence ref before confirming any replacement" in output

def test_wave3_pda_field_work_request_exports_json_without_network(
    monkeypatch,
    capsys,
):
    """field-work-request --json 输出结构化资源项，便于自动分派。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"field work request must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--field-work-request", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-field-work-request"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    resources = {item["resource"]: item for item in payload["resources"]}
    assert "Trace-code OpenAPI contract" in resources
    trace_code = resources["Trace-code OpenAPI contract"]
    assert trace_code["resource_zh"] == "追溯码 OpenAPI 合约"
    assert trace_code["owner_zh"] == "追溯码接口负责人 / 运维"
    assert "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL" in trace_code["verification"]
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY" in trace_code["verification"]
    assert "不得把真实 key 写入仓库或 evidence JSON" in trace_code["verification_zh"]
    assert "At least one real PDA" in resources
    assert resources["At least one real PDA"]["owner"] == "Business / asset owner / device vendor"
    assert "WAVE_3_PDA_PDA_DEVICE_REF" in resources["At least one real PDA"]["verification"]
    assert resources["At least one real PDA"]["resource_zh"] == "至少一台真 PDA"
    assert resources["At least one real PDA"]["owner_zh"] == "业务方 / 资产负责人 / 设备方"
    assert "现场照片或资产登记" in resources["At least one real PDA"]["deliverable_zh"]
    assert "asset://.../pda/..." in resources["At least one real PDA"]["verification_zh"]
    assert "WebView/Capacitor native shell" in resources
    native_shell = resources["WebView/Capacitor native shell"]
    assert native_shell["resource_zh"] == "WebView/Capacitor Android native shell"
    assert native_shell["owner_zh"] == "PDA 技术验证负责人"
    assert "WAVE_3_PDA_NATIVE_SHELL_REF" in native_shell["verification_zh"]
    assert "WebView/Capacitor native scan plugin" in resources
    native_scan = resources["WebView/Capacitor native scan plugin"]
    assert native_scan["resource_zh"] == "WebView/Capacitor native scan plugin"
    assert native_scan["owner_zh"] == "PDA 技术验证负责人"
    assert "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF" in native_scan["verification_zh"]
    assert "Operator usability reviewer" in resources
    usability = resources["Operator usability reviewer"]
    assert usability["resource_zh"] == "人工易用性走查人"
    assert "设备握持" in usability["deliverable_zh"]
    assert "WAVE_3_PDA_USABILITY_REVIEW_REF" in usability["verification_zh"]
    assert payload["execution_order_zh"] == [
        "运维 / 部署负责人提供 WAVE_3_PDA_SERVICE_URL 并运行 service-precheck",
        "追溯码接口负责人 / 运维提供 OpenAPI URL 和 API key 并运行 trace-code OpenAPI precheck",
        "业务方 / 资产负责人提供至少一台真 PDA 并登记资产引用",
        "PDA 技术验证负责人确认实体扫码键或厂商扫码通道",
        "测试负责人准备 50 个脱敏条码样本和 M2/M3 测试数据",
        "测试执行人用真 PDA 采集 M2/M3 scan、offline replay 和 Idempotency-Key replay 日志",
        "PDA 技术验证负责人归档 SPIKE-005 / SPIKE-005B 真机实测结果",
        "后端 / 数据库操作人归档 H2 audit_event 查询证据",
        "测试负责人选择 from-env 或 intake 路径执行 check-only、正式 record 和 validate",
    ]
    assert "Normal closeout must not use --force" in "\n".join(payload["troubleshooting"])

def test_wave3_pda_preaudit_kit_exports_json_without_network(
    monkeypatch,
    capsys,
):
    """preaudit-kit 汇总无 PDA 阶段推进项和真机阻塞项，不联网不写 evidence。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"preaudit kit must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--preaudit-kit", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-preaudit-kit"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert payload["preaudit_stage"] == "before_real_pda_execution"
    assert payload["audiences"] == [
        "运维 / 部署负责人",
        "业务方 / 资产负责人 / 设备方",
        "测试负责人 / 业务数据负责人",
        "PDA 技术验证负责人",
        "后端 / 数据库操作人",
        "业务走查人 / 测试负责人",
    ]
    assert payload["now_actions"] == [
        {
            "owner": "运维 / 部署负责人",
            "action": "确认 dev/staging 环境和 WAVE_3_PDA_SERVICE_URL",
            "proof": "just wave-3-pda-service-precheck --from-env --json 输出",
        },
        {
            "owner": "追溯码接口负责人 / 运维",
            "action": "确认追溯码 OpenAPI URL 和 API key 已通过只读预检",
            "proof": "just wave-3-pda-trace-code-openapi-precheck --from-env --json 输出",
        },
        {
            "owner": "测试负责人 / 业务数据负责人",
            "action": "准备 50 个脱敏条码样本、M2/M3 测试数据和测试账号",
            "proof": "条码样本清单、可重建测试数据说明、m2.write / m3.write 账号引用",
        },
        {
            "owner": "测试负责人",
            "action": "导出 evidence package 模板并预建归档目录",
            "proof": "just wave-3-pda-evidence-package-template 输出归档引用",
        },
        {
            "owner": "业务方 / 资产负责人 / 设备方",
            "action": "借测或采购至少一台真 PDA",
            "proof": "PDA 到位后登记 asset://.../pda/... 设备资产引用",
        },
    ]
    blocked_names = {item["env_var"] for item in payload["blocked_until_real_pda"]}
    assert "WAVE_3_PDA_PDA_MODEL" in blocked_names
    assert "WAVE_3_PDA_SCAN_INPUT_METHOD" in blocked_names
    assert "WAVE_3_PDA_M2_SCAN_LOG_REF" in blocked_names
    assert "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF" in blocked_names
    assert "WAVE_3_PDA_L7_RUN_REF" in blocked_names
    assert "WAVE_3_PDA_USABILITY_REVIEW_REF" in blocked_names
    assert payload["must_not_do"] == [
        "不要创建或伪造 docs/retros/wave-3-pda-runtime-evidence.json",
        "不要用浏览器、模拟器、手机摄像头或本地脚本替代真 PDA 实体扫码键",
        "不要在对应真实证据引用缺失时把 WAVE_3_PDA_* 布尔变量设为 true",
        "不要把 readiness、preaudit-kit、field-work-request、field-execution-summary 或 field-precheck-summary 输出当作关闭 W6.D gate 的 evidence",
    ]
    assert payload["next_commands"][0] == "just wave-3-pda-preaudit-kit --json"
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in payload["next_commands"]

def test_wave3_pda_preaudit_kit_reports_current_env_status_without_values(
    monkeypatch,
    capsys,
):
    """preaudit-kit 应指出当前缺哪些前置变量，但不打印环境变量值。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in readiness.ENV_FIELDS.values():
        monkeypatch.delenv(env_name, raising=False)
    monkeypatch.setenv("WAVE_3_PDA_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE_3_PDA_SERVICE_URL", "https://wms-staging.internal/private")
    monkeypatch.setenv(
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "https://trace-code.internal/openapi/wms-openapi.yaml",
    )
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")

    assert readiness.main(["--preaudit-kit", "--json"]) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert "https://wms-staging.internal/private" not in output
    assert "https://trace-code.internal/openapi/wms-openapi.yaml" not in output
    assert "wms_secret_should_not_leak" not in output
    assert payload["current_env_status"] == {
        "required_now_env_vars": [
            "WAVE_3_PDA_ENVIRONMENT",
            "WAVE_3_PDA_SERVICE_URL",
            "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
            "WAVE_3_PDA_TRACE_CODE_API_KEY",
        ],
        "set_now_env_vars": [
            "WAVE_3_PDA_ENVIRONMENT",
            "WAVE_3_PDA_SERVICE_URL",
            "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
            "WAVE_3_PDA_TRACE_CODE_API_KEY",
        ],
        "missing_now_env_vars": [],
        "missing_now_env_var_owners": [],
    }
