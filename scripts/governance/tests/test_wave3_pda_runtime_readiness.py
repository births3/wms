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


def test_wave3_pda_runtime_readiness_rejects_missing_external_inputs(capsys):
    """readiness 必须列出缺失外部证据输入，且不能写 runtime evidence。"""
    import check_wave3_pda_runtime_readiness as readiness

    assert readiness.main(["--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "docs/retros/wave-3-pda-runtime-evidence.json" not in "\n".join(
        payload["issues"],
    )
    for expected in (
        "environment is required",
        "service_url is required",
        "pda_device_ref is required",
        "m2_scan_log_ref is required",
        "offline_replay_log_ref is required",
        "real_pda_used must be true",
        "barcode_samples_scanned is required",
    ):
        assert expected in payload["issues"]
    assert payload["next_commands"] == _expected_next_commands()


def test_wave3_pda_runtime_readiness_json_includes_w6d_materials_contract(capsys):
    """readiness JSON 应直接输出 W6.D 外部前置和最小证据清单。"""
    import check_wave3_pda_runtime_readiness as readiness
    import report_wave6_pre_release as wave6_report

    assert readiness.main(["--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["external_prerequisites"] == wave6_report.external_prerequisites_for_gate("W6.D")
    assert payload["minimum_evidence_refs"] == wave6_report.minimum_evidence_refs_for_gate("W6.D")
    assert "真 PDA" in payload["external_prerequisites"]
    assert "实体扫码键" in payload["external_prerequisites"]
    assert "PDA 资产引用" in payload["minimum_evidence_refs"]
    assert "L7 执行记录" in payload["minimum_evidence_refs"]


def test_wave3_pda_materials_checklist_exports_json_without_network(
    monkeypatch,
    capsys,
):
    """materials checklist 是现场分工清单，不联网、不写 evidence、不关闭 W6.D。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"materials checklist must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--materials-checklist", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-materials-checklist"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert payload["next_commands"] == _expected_next_commands()

    fields = {field["name"]: field for field in payload["fields"]}
    expected_env_fields = (
        set(readiness.ENV_STRING_FIELDS.values())
        | set(readiness.ENV_COUNT_FIELDS.values())
        | set(readiness.ENV_FLAG_FIELDS.values())
    )
    assert expected_env_fields - set(fields) == set()
    assert fields["WAVE_3_PDA_SERVICE_URL"]["no_pda_stage"] == "preparable"
    assert fields["WAVE_3_PDA_SERVICE_URL"]["requires_real_pda"] is False
    assert fields["WAVE_3_PDA_TRACE_CODE_OPENAPI_URL"]["no_pda_stage"] == "preparable"
    assert fields["WAVE_3_PDA_TRACE_CODE_OPENAPI_URL"]["requires_real_pda"] is False
    assert fields["WAVE_3_PDA_TRACE_CODE_OPENAPI_URL"]["source_owner"] == (
        "追溯码接口负责人 / 运维"
    )
    assert fields["WAVE_3_PDA_TRACE_CODE_API_KEY"]["no_pda_stage"] == "preparable"
    assert fields["WAVE_3_PDA_TRACE_CODE_API_KEY"]["requires_real_pda"] is False
    assert fields["WAVE_3_PDA_TRACE_CODE_API_KEY"]["evidence_requirement"] == (
        "追溯码 OpenAPI 合约"
    )
    assert fields["WAVE_3_PDA_PDA_MODEL"]["requires_real_pda"] is True
    assert fields["WAVE_3_PDA_PDA_MODEL"]["no_pda_stage"] == "blocked_until_device"
    assert fields["WAVE_3_PDA_M2_SCAN_LOG_REF"]["requires_real_pda"] is True
    assert fields["WAVE_3_PDA_AUDIT_EVENT_QUERY_REF"]["requires_real_pda"] is True
    assert fields["WAVE_3_PDA_L7_RUN_REF"]["evidence_requirement"] == "L7 执行记录"
    assert fields["WAVE_3_PDA_USABILITY_REVIEW_REF"]["evidence_requirement"] == "走查记录"


def test_wave3_pda_field_execution_summary_reports_env_gaps_without_value_leak(
    monkeypatch,
    capsys,
):
    """field execution summary 应汇总现场缺口，但不联网、不泄漏变量值。"""
    import check_wave3_pda_runtime_readiness as readiness

    secret = "wms_secret_should_not_leak"
    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    monkeypatch.setenv("WAVE_3_PDA_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", secret)

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"field execution summary must not call network: {url}")

    def fail_http_text_with_api_key(url, api_key, timeout_seconds=10):
        raise AssertionError(f"field execution summary must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)
    monkeypatch.setattr(readiness, "http_text_with_api_key", fail_http_text_with_api_key)

    assert readiness.main(["--field-execution-summary", "--json"]) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert secret not in output
    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-field-execution-summary"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert payload["current_env_status"]["set_now_env_vars"] == [
        "WAVE_3_PDA_ENVIRONMENT",
        "WAVE_3_PDA_TRACE_CODE_API_KEY",
    ]
    assert payload["current_env_status"]["missing_now_env_vars"] == [
        "WAVE_3_PDA_SERVICE_URL",
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
    ]
    assert payload["no_pda_precheck_commands"] == [
        "just wave-3-pda-service-precheck --from-env --json",
        "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
        "just wave-3-pda-field-precheck-summary --from-env --json",
    ]
    assert payload["field_package_commands"] == [
        "just wave-3-pda-preaudit-kit --json",
        "just wave-3-pda-materials-checklist --json",
        "just wave-3-pda-field-work-request",
        "just wave-3-pda-evidence-package-template",
        "just wave-3-pda-runtime-evidence-record --export-template",
    ]
    assert "WAVE_3_PDA_PDA_MODEL" in payload["real_pda_required_env_vars"]
    assert "WAVE_3_PDA_M2_SCAN_LOG_REF" in payload["real_pda_missing_env_vars"]
    real_pda_missing_owners = {
        item["env_var"]: item
        for item in payload["real_pda_missing_env_var_owners"]
    }
    assert real_pda_missing_owners["WAVE_3_PDA_PDA_MODEL"]["source_owner"] == (
        "设备借测 / 资产负责人"
    )
    assert real_pda_missing_owners["WAVE_3_PDA_M2_SCAN_LOG_REF"]["source_owner"] == (
        "测试执行人"
    )
    assert real_pda_missing_owners["WAVE_3_PDA_AUDIT_EVENT_QUERY_REF"][
        "evidence_requirement"
    ] == "audit_event 查询"
    assert "WAVE_3_PDA_REAL_PDA_USED" in payload["truth_flag_env_vars"]
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" in payload[
        "no_pda_precheck_truth_flag_env_vars"
    ]
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" not in payload[
        "truth_flags_must_remain_false_until_refs_present"
    ]
    assert "WAVE_3_PDA_REAL_PDA_USED" in payload[
        "truth_flags_must_remain_false_until_refs_present"
    ]
    assert "WAVE_3_PDA_REAL_PDA_USED" in payload["false_truth_flag_env_vars"]


def test_wave3_pda_field_execution_summary_accepts_sanitized_precheck_attachment(
    monkeypatch,
    tmp_path,
    capsys,
):
    """已归档脱敏前置附件可减少重复要 env，但不能关闭 W6.D。"""
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
                    "openapi_url": "http://43.128.77.47:9100/openapi/wms-openapi.yaml",
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
            "--field-execution-summary",
            "--field-precheck-attachment",
            str(attachment),
            "--json",
        ],
    ) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["ready_for_record_from_env_vars"] is False
    assert payload["current_env_status"]["set_now_env_vars"] == []
    assert payload["current_env_status"]["missing_now_env_vars"] == []
    assert payload["current_env_status"]["satisfied_by_precheck_attachment_env_vars"] == [
        "WAVE_3_PDA_ENVIRONMENT",
        "WAVE_3_PDA_SERVICE_URL",
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "WAVE_3_PDA_TRACE_CODE_API_KEY",
    ]
    assert payload["current_env_status"]["precheck_attachment"] == {
        "path": str(attachment),
        "kind": "wave3-pda-field-precheck-attachment",
        "service_precheck_ok": True,
        "trace_code_openapi_precheck_ok": True,
        "writes_runtime_evidence": False,
        "closes_gate": False,
    }
    assert payload["satisfied_by_precheck_attachment_truth_flag_env_vars"] == [
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
    ]
    assert "WAVE_3_PDA_PDA_MODEL" in payload["real_pda_missing_env_vars"]
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" not in payload[
        "false_truth_flag_env_vars"
    ]
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" not in payload[
        "false_no_pda_precheck_truth_flag_env_vars"
    ]
    assert "WAVE_3_PDA_REAL_PDA_USED" in payload[
        "false_real_evidence_truth_flag_env_vars"
    ]
    false_truth_flag_owners = {
        item["env_var"]: item
        for item in payload["false_truth_flag_env_var_owners"]
    }
    assert false_truth_flag_owners["WAVE_3_PDA_REAL_PDA_USED"]["source_owner"] == (
        "现场负责人"
    )
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" not in false_truth_flag_owners
    false_real_evidence_truth_flag_owners = {
        item["env_var"]: item
        for item in payload["false_real_evidence_truth_flag_env_var_owners"]
    }
    assert "WAVE_3_PDA_REAL_PDA_USED" in false_real_evidence_truth_flag_owners
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" not in (
        false_real_evidence_truth_flag_owners
    )
    assert payload["ready_for_record_from_env_vars"] is False
    assert payload["record_commands"] == [
        "just wave-3-pda-runtime-readiness --from-env --json",
        "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
        "just wave-3-pda-runtime-evidence-record --from-env --json",
        "just wave-3-pda-intake-check --json",
        "just wave-3-pda-intake-record --json",
        "just wave-3-pda-runtime-evidence-validate",
    ]
    assert payload["next_commands"] == _expected_next_commands()


def test_wave3_pda_field_precheck_attachment_rejects_incomplete_service_facts(
    monkeypatch,
    tmp_path,
    capsys,
):
    """附件缺服务鉴权事实时，不能被用于抵消 no-PDA env 缺口。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    attachment = tmp_path / "bad-service-precheck.json"
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
                },
                "trace_code_openapi_precheck": {
                    "ok": False,
                },
            },
        ),
        encoding="utf-8",
    )

    assert readiness.main(
        [
            "--field-execution-summary",
            "--field-precheck-attachment",
            str(attachment),
            "--json",
        ],
    ) == 2
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert "service_precheck.wave3_route_error_code must be AUTH-001" in payload[
        "error"
    ]


@pytest.mark.parametrize(
    ("field", "value", "message"),
    [
        (
            "healthz_payload_status",
            "down",
            "service_precheck.healthz_payload_status must be ok",
        ),
        (
            "wave3_route_status",
            403,
            "service_precheck.wave3_route_status must be 401",
        ),
    ],
)
def test_wave3_pda_field_precheck_attachment_rejects_service_fact_drift(
    monkeypatch,
    tmp_path,
    capsys,
    field,
    value,
    message,
):
    """附件服务事实必须和 service precheck 主路径同等严格。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    service_precheck = {
        "ok": True,
        "environment": "staging",
        "service_url": "http://wms-staging.internal:18080",
        "healthz_status": 200,
        "healthz_payload_status": "ok",
        "wave3_route_status": 401,
        "wave3_route_error_code": "AUTH-001",
    }
    service_precheck[field] = value
    attachment = tmp_path / "bad-service-precheck-drift.json"
    attachment.write_text(
        json.dumps(
            {
                "kind": "wave3-pda-field-precheck-attachment",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "runtime_evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
                "service_precheck": service_precheck,
                "trace_code_openapi_precheck": {
                    "ok": False,
                },
            },
        ),
        encoding="utf-8",
    )

    assert readiness.main(
        [
            "--field-execution-summary",
            "--field-precheck-attachment",
            str(attachment),
            "--json",
        ],
    ) == 2
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert message in payload["error"]


@pytest.mark.parametrize(
    ("attachment_payload", "secret", "message"),
    [
        (
            {
                "service_precheck": {
                    "ok": True,
                    "environment": "staging",
                    "service_url": "http://ops:secret-value@wms-staging.internal:18080",
                    "healthz_status": 200,
                    "healthz_payload_status": "ok",
                    "wave3_route_status": 401,
                    "wave3_route_error_code": "AUTH-001",
                },
                "trace_code_openapi_precheck": {"ok": False},
            },
            "secret-value",
            "service_precheck.service_url cannot contain userinfo credentials",
        ),
        (
            {
                "service_precheck": {"ok": False},
                "trace_code_openapi_precheck": {
                    "ok": True,
                    "openapi_url": (
                        "http://43.128.77.47:9100/openapi/wms-openapi.yaml"
                        "?token=secret-value"
                    ),
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
            "secret-value",
            "trace_code_openapi_precheck.openapi_url query cannot contain sensitive parameter: token",
        ),
    ],
)
def test_wave3_pda_field_precheck_attachment_rejects_url_secrets(
    monkeypatch,
    tmp_path,
    capsys,
    attachment_payload,
    secret,
    message,
):
    """field precheck 附件也不能携带 URL userinfo 或 query secret。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    attachment = tmp_path / "bad-url-secret-precheck.json"
    attachment_payload = {
        "kind": "wave3-pda-field-precheck-attachment",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "runtime_evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        **attachment_payload,
    }
    attachment.write_text(
        json.dumps(attachment_payload, ensure_ascii=False),
        encoding="utf-8",
    )

    assert readiness.main(
        [
            "--field-execution-summary",
            "--field-precheck-attachment",
            str(attachment),
            "--json",
        ],
    ) == 2
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert payload["ok"] is False
    assert message in payload["error"]
    assert secret not in output





































































































_MOVED_TESTS = {
    'test_wave3_pda_field_precheck_attachment_rejects_incomplete_openapi_facts': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_precheck_attachment_rejects_trace_code_openapi_version_drift': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_precheck_attachment_rejects_incomplete_openapi_operations': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_execution_summary_treats_native_refs_as_webview_only': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_owner_gap_actions_groups_missing_work_by_owner_without_network': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_owner_gap_actions_reuses_sanitized_precheck_attachment': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_owner_gap_actions_exports_markdown_for_field_handoff': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_handoff_bundle_exports_json_without_network_or_key_leak': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_handoff_bundle_from_env_includes_precheck_without_key_leak': 'test_wave3_pda_runtime_readiness_part_1',
    'test_wave3_pda_field_handoff_bundle_reuses_precheck_attachment_in_preaudit': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_field_handoff_bundle_can_write_sanitized_handoff_file': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_field_handoff_bundle_rejects_overwrite_without_force': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_field_handoff_bundle_force_overwrites_handoff_file': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_field_handoff_bundle_exports_markdown': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_from_env_validates_contract_without_key_leak': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_rejects_wrong_method': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_rejects_version_drift': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_reports_proxy_troubleshooting_on_502': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_text_reports_troubleshooting_on_failure': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_reports_missing_env_without_network': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_rejects_url_secrets_without_network': 'test_wave3_pda_runtime_readiness_part_2',
    'test_wave3_pda_trace_code_openapi_precheck_rejects_incomplete_contract': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_field_precheck_summary_combines_service_trace_and_field_status_without_key_leak': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_field_precheck_summary_exports_markdown_for_field_handoff': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_field_precheck_summary_markdown_lists_missing_now_owners_without_network': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_field_precheck_summary_returns_nonzero_with_structured_failures': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_field_work_request_exports_markdown_without_network': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_field_work_request_exports_json_without_network': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_preaudit_kit_exports_json_without_network': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_preaudit_kit_reports_current_env_status_without_values': 'test_wave3_pda_runtime_readiness_part_3',
    'test_wave3_pda_preaudit_kit_reports_missing_now_env_owner_details': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_preaudit_kit_exports_markdown_without_network': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_checks_staging_health_and_wave3_route': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_checks_service_and_payload': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_ignores_blank_native_refs_for_rn': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_rejects_missing_service_url': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_rejects_local_service_url_without_network': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_service_url_boundary_rejects_url_secrets_without_network': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_rejects_local_service_url_without_network': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_rejects_invalid_boolean': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_cli_json_reports_env_errors_as_json': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_strips_boolean_whitespace': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_strips_string_and_integer_whitespace': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_rejects_non_positive_counts_before_network': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_supports_webview_native_refs': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_from_env_rejects_blank_webview_native_refs': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_service_precheck_only_skips_pda_evidence_inputs': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_service_precheck_only_from_env_ignores_pda_flags': 'test_wave3_pda_runtime_readiness_part_4',
    'test_wave3_pda_runtime_readiness_rejects_missing_wave3_route': 'test_wave3_pda_runtime_readiness_part_5',
    'test_wave3_pda_runtime_readiness_still_probes_service_when_pda_inputs_missing': 'test_wave3_pda_runtime_readiness_part_5',
}

def __getattr__(name: str):
    module = _MOVED_TESTS.get(name)
    if module is None:
        raise AttributeError(name)
    return getattr(__import__(module), name)
