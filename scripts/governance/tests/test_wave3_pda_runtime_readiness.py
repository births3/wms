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


def test_wave3_pda_field_precheck_attachment_rejects_incomplete_openapi_facts(
    monkeypatch,
    tmp_path,
    capsys,
):
    """附件缺 OpenAPI 鉴权或 paths 事实时，不能被用于抵消 trace-code env 缺口。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    attachment = tmp_path / "bad-trace-code-precheck.json"
    attachment.write_text(
        json.dumps(
            {
                "kind": "wave3-pda-field-precheck-attachment",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "runtime_evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
                "service_precheck": {
                    "ok": False,
                },
                "trace_code_openapi_precheck": {
                    "ok": True,
                    "status": 200,
                    "openapi": "3.0.3",
                    "api_key_header_name": "Authorization",
                    "required_paths_present": [
                        "/api/codes/{code}",
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
    ) == 2
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert "trace_code_openapi_precheck.api_key_header_name must be X-API-Key" in (
        payload["error"]
    )


def test_wave3_pda_field_precheck_attachment_rejects_trace_code_openapi_version_drift(
    monkeypatch,
    tmp_path,
    capsys,
):
    """附件 OpenAPI 版本必须是当前 WMS 对接合同要求的 3.0.3。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    attachment = tmp_path / "bad-trace-code-version.json"
    attachment.write_text(
        json.dumps(
            {
                "kind": "wave3-pda-field-precheck-attachment",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "runtime_evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
                "service_precheck": {
                    "ok": False,
                },
                "trace_code_openapi_precheck": {
                    "ok": True,
                    "status": 200,
                    "openapi": "3.1.0",
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
    ) == 2
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert "trace_code_openapi_precheck.openapi must be 3.0.3" in payload["error"]


def test_wave3_pda_field_precheck_attachment_rejects_incomplete_openapi_operations(
    monkeypatch,
    tmp_path,
    capsys,
):
    """附件声明 operation 摘要时，必须覆盖全部 required GET/POST。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    attachment = tmp_path / "bad-trace-code-operations.json"
    attachment.write_text(
        json.dumps(
            {
                "kind": "wave3-pda-field-precheck-attachment",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "runtime_evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
                "service_precheck": {
                    "ok": False,
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
            "--field-execution-summary",
            "--field-precheck-attachment",
            str(attachment),
            "--json",
        ],
    ) == 2
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert "missing required operations: POST /api/codes/batch" in payload["error"]


def test_wave3_pda_field_execution_summary_treats_native_refs_as_webview_only(
    monkeypatch,
    capsys,
):
    """React Native 候选不应把 WebView/Capacitor native refs 当作必填。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in (
        set(readiness.ENV_FIELDS.values())
        | set(readiness.TRACE_CODE_ENV_FIELDS.values())
    ):
        monkeypatch.delenv(env_name, raising=False)
    for env_name, value in _valid_wave3_pda_env().items():
        monkeypatch.setenv(env_name, value)
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_OPENAPI_URL", "http://trace-code.internal/openapi.yaml")
    monkeypatch.setenv("WAVE_3_PDA_TRACE_CODE_API_KEY", "wms_secret_should_not_leak")
    monkeypatch.delenv("WAVE_3_PDA_NATIVE_SHELL_REF", raising=False)
    monkeypatch.delenv("WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF", raising=False)

    assert readiness.main(["--field-execution-summary", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert "WAVE_3_PDA_NATIVE_SHELL_REF" not in payload["real_pda_required_env_vars"]
    assert "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF" not in payload["real_pda_required_env_vars"]
    assert "WAVE_3_PDA_NATIVE_SHELL_REF" not in payload["real_pda_missing_env_vars"]
    assert "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF" not in payload["real_pda_missing_env_vars"]
    assert payload["false_truth_flag_env_vars"] == []
    assert payload["false_no_pda_precheck_truth_flag_env_vars"] == []
    assert payload["false_real_evidence_truth_flag_env_vars"] == []
    assert payload["ready_for_record_from_env_vars"] is True


def test_wave3_pda_field_owner_gap_actions_groups_missing_work_by_owner_without_network(
    monkeypatch,
    capsys,
):
    """owner gap actions 应把现场缺口按负责人分组，便于直接派单。"""
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
        raise AssertionError(f"owner gap actions must not call network: {url}")

    def fail_http_text_with_api_key(url, api_key, timeout_seconds=10):
        raise AssertionError(f"owner gap actions must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)
    monkeypatch.setattr(readiness, "http_text_with_api_key", fail_http_text_with_api_key)

    assert readiness.main(["--field-owner-gap-actions", "--json"]) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert secret not in output
    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-field-owner-gap-actions"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert payload["field_execution_summary"]["mode"] == "wave3-pda-field-execution-summary"
    assert payload["ready_for_record_from_env_vars"] is False
    assert payload["gap_action_count"] > 0

    actions = {
        item["source_owner"]: item
        for item in payload["field_owner_gap_actions"]
    }
    asset_owner = actions["设备借测 / 资产负责人"]
    assert "WAVE_3_PDA_PDA_MODEL" in asset_owner["missing_env_vars"]
    assert "WAVE_3_PDA_ANDROID_VERSION" in asset_owner["missing_env_vars"]
    assert asset_owner["requires_real_pda"] is True
    assert asset_owner["evidence_requirements"] == ["PDA 资产引用"]
    assert asset_owner["next_action"] == "补齐缺失环境变量或真实 evidence 引用"

    ops_owner = actions["运维 / 部署负责人"]
    assert "WAVE_3_PDA_SERVICE_URL" in ops_owner["missing_now_env_vars"]
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" in (
        ops_owner["false_truth_flag_env_vars"]
    )
    assert ops_owner["requires_real_pda"] is False

    field_owner = actions["现场负责人"]
    assert "WAVE_3_PDA_REAL_PDA_USED" in field_owner["missing_env_vars"]
    assert "WAVE_3_PDA_REAL_PDA_USED" in field_owner["false_truth_flag_env_vars"]
    assert payload["next_commands"] == _expected_next_commands()


def test_wave3_pda_field_owner_gap_actions_reuses_sanitized_precheck_attachment(
    monkeypatch,
    tmp_path,
    capsys,
):
    """owner gap actions 可复用脱敏附件，但仍保留真 PDA 缺口。"""
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
            "--field-owner-gap-actions",
            "--field-precheck-attachment",
            str(attachment),
            "--json",
        ],
    ) == 0
    payload = json.loads(capsys.readouterr().out)

    actions = {
        item["source_owner"]: item
        for item in payload["field_owner_gap_actions"]
    }
    assert "追溯码接口负责人 / 运维" not in actions
    assert "运维 / 部署负责人" not in actions
    assert "WAVE_3_PDA_PDA_MODEL" in actions["设备借测 / 资产负责人"][
        "missing_env_vars"
    ]
    assert payload["ready_for_record_from_env_vars"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False


def test_wave3_pda_field_owner_gap_actions_exports_markdown_for_field_handoff(
    monkeypatch,
    capsys,
):
    """owner gap actions 默认输出 Markdown，便于直接转发给现场负责人。"""
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
        raise AssertionError(f"owner gap actions must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--field-owner-gap-actions"]) == 0
    output = capsys.readouterr().out

    assert secret not in output
    assert "# W6.D PDA Owner Gap Actions" in output
    assert "This handoff is read-only and cannot close W6.D." in output
    assert "writes_runtime_evidence=false" in output
    assert "closes_gate=false" in output
    assert "## Summary" in output
    assert "ready_for_record_from_env_vars=false" in output
    assert "gap_action_count=" in output
    assert "real_pda_missing_env_vars_count=" in output
    assert "false_truth_flag_env_vars_count=" in output
    assert "evidence_file=docs/retros/wave-3-pda-runtime-evidence.json" in output
    assert (
        "| Owner | Action | Missing now | Real evidence vars | False flags | "
        "Evidence requirements | Stage | Real PDA? |"
    ) in output
    assert "设备借测 / 资产负责人" in output
    assert "`WAVE_3_PDA_PDA_MODEL`" in output
    assert "`PDA 资产引用`" in output
    assert "运维 / 部署负责人" in output
    assert "`WAVE_3_PDA_SERVICE_URL`" in output
    assert "`WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED`" in output
    assert "## Commands" in output
    assert "\njust wave-3-pda-field-owner-gap-actions\n" not in output
    assert "just wave-3-pda-field-owner-gap-actions --json" in output
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in output


def test_wave3_pda_field_handoff_bundle_exports_json_without_network_or_key_leak(
    monkeypatch,
    capsys,
):
    """field-handoff-bundle 默认聚合现场材料，不联网、不写 evidence、不泄露 key。"""
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
        raise AssertionError(f"field handoff bundle must not call network: {url}")

    def fail_http_text_with_api_key(url, api_key, timeout_seconds=10):
        raise AssertionError(f"field handoff bundle must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)
    monkeypatch.setattr(readiness, "http_text_with_api_key", fail_http_text_with_api_key)

    assert readiness.main(["--field-handoff-bundle", "--json"]) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert secret not in output
    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-field-handoff-bundle"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["include_precheck"] is False
    assert payload["field_precheck_summary"] is None
    assert "field_precheck_summary_not_run" in payload["bundle_scope"]
    assert payload["preaudit_kit"]["mode"] == "wave3-pda-preaudit-kit"
    assert payload["materials_checklist"]["mode"] == "wave3-pda-materials-checklist"
    assert payload["field_work_request"]["mode"] == "wave3-pda-field-work-request"
    assert payload["field_execution_summary"]["mode"] == "wave3-pda-field-execution-summary"
    assert payload["field_owner_gap_actions"]["mode"] == "wave3-pda-field-owner-gap-actions"
    assert payload["evidence_package_template"]["mode"] == "wave3-pda-evidence-package-template"
    assert payload["evidence_package_template"]["writes_runtime_evidence"] is False
    assert payload["evidence_package_template"]["closes_gate"] is False
    assert payload["intake_template"]["mode"] == (
        "wave3-pda-runtime-evidence-intake-template"
    )
    assert payload["intake_template"]["kind"] == "wave3-pda-runtime-evidence-intake"
    assert payload["intake_template"]["writes_runtime_evidence"] is False
    assert payload["intake_template"]["closes_gate"] is False
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY" not in json.dumps(
        payload["intake_template"],
    )
    assert payload["ready_for_record_from_env_vars"] is False
    assert payload["gap_action_count"] > 0
    assert payload["real_pda_missing_env_vars_count"] > 0
    assert payload["false_truth_flag_env_vars_count"] > 0
    assert payload["next_commands"] == _expected_next_commands()

    owner_action = payload["field_owner_gap_actions"]["field_owner_gap_actions"][0]
    assert {
        "source_owner",
        "env_vars",
        "missing_env_vars",
        "missing_now_env_vars",
        "false_truth_flag_env_vars",
        "evidence_requirements",
        "no_pda_stages",
        "requires_real_pda",
    }.issubset(owner_action)

    package_owner_action = payload["evidence_package_template"]["owner_actions"][0]
    assert package_owner_action["can_write_runtime_evidence"] is False
    assert "required_env_vars" in package_owner_action


def test_wave3_pda_field_handoff_bundle_from_env_includes_precheck_without_key_leak(
    monkeypatch,
    capsys,
):
    """field-handoff-bundle --from-env 可纳入只读预检结果，但不能泄露 key。"""
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

    assert readiness.main(["--field-handoff-bundle", "--from-env", "--json"]) == 0
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert secret not in output
    assert payload["ok"] is True
    assert payload["include_precheck"] is True
    assert "field_precheck_summary_from_env" in payload["bundle_scope"]
    assert payload["field_precheck_summary"]["ok"] is True
    assert payload["field_precheck_summary"]["service_precheck"]["ok"] is True
    assert payload["field_precheck_summary"]["trace_code_openapi_precheck"]["ok"] is True
    assert payload["field_precheck_summary"]["trace_code_openapi_precheck"]["facts"][
        "api_key_header_name"
    ] == "X-API-Key"
    assert calls == [
        ("json", "http://wms-staging.internal/healthz", 10),
        ("json", "http://wms-staging.internal/api/v1/inventory/batches", 10),
        (
            "text",
            "http://trace-code.internal/openapi/wms-openapi.yaml",
            secret,
            10,
        ),
    ]


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


def test_wave3_pda_preaudit_kit_reports_missing_now_env_owner_details(
    monkeypatch,
    capsys,
):
    """preaudit-kit 缺当前阶段变量时，应直接给出负责人。"""
    import check_wave3_pda_runtime_readiness as readiness

    for env_name in readiness.ENV_FIELDS.values():
        monkeypatch.delenv(env_name, raising=False)
    for env_name in readiness.TRACE_CODE_ENV_FIELDS.values():
        monkeypatch.delenv(env_name, raising=False)

    assert readiness.main(["--preaudit-kit", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["current_env_status"]["set_now_env_vars"] == []
    assert payload["current_env_status"]["missing_now_env_vars"] == [
        "WAVE_3_PDA_ENVIRONMENT",
        "WAVE_3_PDA_SERVICE_URL",
        "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "WAVE_3_PDA_TRACE_CODE_API_KEY",
    ]
    assert payload["current_env_status"]["missing_now_env_var_owners"] == [
        {
            "env_var": "WAVE_3_PDA_ENVIRONMENT",
            "source_owner": "运维 / 部署负责人",
            "no_pda_stage": "preparable",
            "requires_real_pda": False,
            "evidence_requirement": "dev/staging M2/M3 API",
        },
        {
            "env_var": "WAVE_3_PDA_SERVICE_URL",
            "source_owner": "运维 / 部署负责人",
            "no_pda_stage": "preparable",
            "requires_real_pda": False,
            "evidence_requirement": "dev/staging M2/M3 API",
        },
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


def test_wave3_pda_preaudit_kit_exports_markdown_without_network(
    monkeypatch,
    capsys,
):
    """preaudit-kit Markdown 应能直接转发给项目负责人和现场负责人。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"preaudit kit must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--preaudit-kit"]) == 0
    output = capsys.readouterr().out

    assert "# W6.D PDA Pre-Audit Kit" in output
    assert "不是 runtime evidence JSON，不能关闭 W6.D gate" in output
    assert "## 现在就能推进" in output
    assert "确认 dev/staging 环境和 WAVE_3_PDA_SERVICE_URL" in output
    assert "准备 50 个脱敏条码样本、M2/M3 测试数据和测试账号" in output
    assert "确认追溯码 OpenAPI URL 和 API key 已通过只读预检" in output
    assert "导出 evidence package 模板并预建归档目录" in output
    assert "借测或采购至少一台真 PDA" in output
    assert "## 必须等真 PDA 实扫后才能填写" in output
    assert "WAVE_3_PDA_PDA_MODEL" in output
    assert "WAVE_3_PDA_SCAN_INPUT_METHOD" in output
    assert "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF" in output
    assert "WAVE_3_PDA_USABILITY_REVIEW_REF" in output
    assert "## 禁止事项" in output
    assert "不要创建或伪造 docs/retros/wave-3-pda-runtime-evidence.json" in output
    assert "不要用浏览器、模拟器、手机摄像头或本地脚本替代真 PDA 实体扫码键" in output
    assert "just wave-3-pda-preaudit-kit --json" in output


def test_wave3_pda_runtime_readiness_checks_staging_health_and_wave3_route(
    monkeypatch,
    capsys,
):
    """readiness 应探测 staging healthz 与 Wave3 路由鉴权，但不写 evidence。"""
    import check_wave3_pda_runtime_readiness as readiness

    calls = []

    def fake_http_json(url, timeout_seconds=10):
        calls.append((url, timeout_seconds))
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(
                401,
                {"code": "AUTH-001", "message": "缺少 Authorization 头"},
            )
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main([*_valid_args(), "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["facts"]["healthz_status"] == 200
    assert payload["facts"]["wave3_route_status"] == 401
    assert payload["facts"]["wave3_route_error_code"] == "AUTH-001"
    assert calls == [
        ("http://wms-staging.internal/healthz", 10),
        ("http://wms-staging.internal/api/v1/inventory/batches", 10),
    ]


def test_wave3_pda_runtime_readiness_from_env_checks_service_and_payload(
    monkeypatch,
    capsys,
):
    """from-env 应复用 WAVE_3_PDA_* 变量，避免现场粘贴长 readiness 参数。"""
    import check_wave3_pda_runtime_readiness as readiness

    for key, value in _valid_wave3_pda_env().items():
        monkeypatch.setenv(key, value)

    calls = []

    def fake_http_json(url, timeout_seconds=10):
        calls.append((url, timeout_seconds))
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main(["--from-env", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-runtime-readiness"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["facts"]["service_url"] == "http://wms-staging.internal"
    assert payload["facts"]["healthz_status"] == 200
    assert payload["facts"]["wave3_route_error_code"] == "AUTH-001"
    assert calls == [
        ("http://wms-staging.internal/healthz", 10),
        ("http://wms-staging.internal/api/v1/inventory/batches", 10),
    ]


def test_wave3_pda_runtime_readiness_from_env_ignores_blank_native_refs_for_rn(
    monkeypatch,
    capsys,
):
    """RN 候选下 export-template 的空 native refs 不应污染 readiness payload。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_NATIVE_SHELL_REF"] = ""
    env["WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF"] = ""
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main(["--from-env", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-runtime-readiness"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False


def test_wave3_pda_runtime_readiness_from_env_rejects_missing_service_url(
    monkeypatch,
    capsys,
):
    """from-env 缺服务地址时必须失败，不能静默跳过 dev/staging 前置验证。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env.pop("WAVE_3_PDA_SERVICE_URL")
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"missing service url must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--from-env", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["mode"] == "wave3-pda-runtime-readiness"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "service_url is required" in payload["issues"]
    assert payload["missing_env_vars"] == ["WAVE_3_PDA_SERVICE_URL"]
    assert payload["missing_env_var_owners"] == [
        {
            "env_var": "WAVE_3_PDA_SERVICE_URL",
            "source_owner": "运维 / 部署负责人",
            "no_pda_stage": "preparable",
            "requires_real_pda": False,
            "evidence_requirement": "dev/staging M2/M3 API",
        },
    ]


def test_wave3_pda_runtime_readiness_rejects_local_service_url_without_network(
    monkeypatch,
    capsys,
):
    """service-precheck 不能把本机服务当作 dev/staging 服务前置。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"local service url must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main([
        "--environment",
        "staging",
        "--service-url",
        "http://localhost:18080",
        "--service-precheck-only",
        "--json",
    ]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["mode"] == "wave3-pda-service-precheck"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "service_url cannot point to local/prod/production/mock/fake/stub/example" in payload["issues"]
    assert "healthz_status" not in payload["facts"]


@pytest.mark.parametrize(
    ("service_url", "secret", "message"),
    [
        (
            "http://ops:secret-value@wms-staging.internal:18080",
            "secret-value",
            "service_url cannot contain userinfo credentials",
        ),
        (
            "http://wms-staging.internal:18080/health?api_key=secret-value",
            "secret-value",
            "service_url query cannot contain sensitive parameter: api_key",
        ),
    ],
)
def test_wave3_pda_service_url_boundary_rejects_url_secrets_without_network(
    monkeypatch,
    capsys,
    service_url,
    secret,
    message,
):
    """service_url 不能把账号、密码或 token 放进 URL 并回显到 JSON。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"secret-bearing service url must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main([
        "--environment",
        "staging",
        "--service-url",
        service_url,
        "--service-precheck-only",
        "--json",
    ]) == 1
    output = capsys.readouterr().out
    payload = json.loads(output)

    assert payload["ok"] is False
    assert message in payload["issues"]
    assert secret not in output
    assert "healthz_status" not in payload["facts"]


def test_wave3_pda_runtime_readiness_from_env_rejects_local_service_url_without_network(
    monkeypatch,
    capsys,
):
    """from-env 也必须拒绝本机 service URL，避免误标服务前置已验证。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_SERVICE_URL"] = "http://127.0.0.1:18080"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"local service url must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--from-env", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["mode"] == "wave3-pda-runtime-readiness"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "service_url cannot point to local/prod/production/mock/fake/stub/example" in payload["issues"]
    assert "healthz_status" not in payload["facts"]


def test_wave3_pda_runtime_readiness_from_env_rejects_invalid_boolean(monkeypatch):
    """from-env 布尔变量拼错时必须失败，避免误当 false。"""
    import check_wave3_pda_runtime_readiness as readiness

    for key, value in _valid_wave3_pda_env().items():
        monkeypatch.setenv(key, value)
    monkeypatch.setenv("WAVE_3_PDA_REAL_PDA_USED", "TRUEE")

    assert readiness.main(["--from-env", "--json"]) == 2


def test_wave3_pda_runtime_readiness_cli_json_reports_env_errors_as_json():
    """真实 CLI argv=None 路径下 --json 异常也必须输出 JSON。"""
    env = os.environ.copy()
    env.update(_valid_wave3_pda_env())
    env["WAVE_3_PDA_REAL_PDA_USED"] = "TRUEE"

    result = subprocess.run(
        [
            sys.executable,
            "scripts/governance/check_wave3_pda_runtime_readiness.py",
            "--from-env",
            "--json",
        ],
        cwd=Path(__file__).resolve().parents[3],
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )

    payload = json.loads(result.stdout)
    assert result.returncode == 2
    assert result.stderr == ""
    assert payload["ok"] is False
    assert payload["mode"] == "wave3-pda-runtime-readiness"
    assert "WAVE_3_PDA_REAL_PDA_USED must be true or false" in payload["error"]
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False


def test_wave3_pda_runtime_readiness_from_env_strips_boolean_whitespace(
    monkeypatch,
    capsys,
):
    """from-env 布尔变量允许 shell 拷贝时的首尾空白。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_REAL_PDA_USED"] = " true "
    env["WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED"] = "\ttrue\n"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main(["--from-env", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-runtime-readiness"


def test_wave3_pda_runtime_readiness_from_env_strips_string_and_integer_whitespace(
    monkeypatch,
    capsys,
):
    """readiness from-env 允许现场复制变量值时带首尾空白。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_ENVIRONMENT"] = " staging "
    env["WAVE_3_PDA_SERVICE_URL"] = "\thttp://wms-staging.internal\n"
    env["WAVE_3_PDA_PDA_MODEL"] = " Honeywell EDA52 "
    env["WAVE_3_PDA_BARCODE_SAMPLES_SCANNED"] = " 50 "
    env["WAVE_3_PDA_M2_OPERATIONS_EXERCISED"] = "\t1\n"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    calls = []

    def fake_http_json(url, timeout_seconds=10):
        calls.append((url, timeout_seconds))
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main(["--from-env", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-runtime-readiness"
    assert payload["facts"]["environment"] == "staging"
    assert payload["facts"]["service_url"] == "http://wms-staging.internal"
    assert calls == [
        ("http://wms-staging.internal/healthz", 10),
        ("http://wms-staging.internal/api/v1/inventory/batches", 10),
    ]


def test_wave3_pda_runtime_readiness_from_env_rejects_non_positive_counts_before_network(
    monkeypatch,
    capsys,
):
    """readiness from-env 计数字段非正时应作为输入错误处理，不联网。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_BARCODE_SAMPLES_SCANNED"] = "0"
    env["WAVE_3_PDA_M2_OPERATIONS_EXERCISED"] = "-1"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    def fail_http_json(url, timeout_seconds=10):
        raise AssertionError(f"invalid env counts must not call network: {url}")

    monkeypatch.setattr(readiness, "http_json", fail_http_json)

    assert readiness.main(["--from-env", "--json"]) == 2
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED must be > 0" in payload["error"]
    assert "WAVE_3_PDA_M2_OPERATIONS_EXERCISED must be > 0" in payload["error"]


def test_wave3_pda_runtime_readiness_from_env_supports_webview_native_refs(
    monkeypatch,
    capsys,
):
    """from-env 在 WebView/Capacitor 候选下必须读取 native refs。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env.update({
        "WAVE_3_PDA_STACK_CANDIDATE": "webview-capacitor",
        "WAVE_3_PDA_SPIKE_RESULT_REF": (
            "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260604.md"
        ),
        "WAVE_3_PDA_NATIVE_SHELL_REF": "ci/staging/wave3-pda-native-shell/123",
        "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF": (
            "ci/staging/wave3-pda-native-scan-plugin/123"
        ),
    })
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main(["--from-env", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True


def test_wave3_pda_runtime_readiness_from_env_rejects_blank_webview_native_refs(
    monkeypatch,
    capsys,
):
    """WebView/Capacitor 候选下 native refs 为空必须失败。"""
    import check_wave3_pda_runtime_readiness as readiness

    env = _valid_wave3_pda_env()
    env.update({
        "WAVE_3_PDA_STACK_CANDIDATE": "webview-capacitor",
        "WAVE_3_PDA_SPIKE_RESULT_REF": (
            "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260604.md"
        ),
        "WAVE_3_PDA_NATIVE_SHELL_REF": "",
        "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF": "",
    })
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main(["--from-env", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert "native_shell_ref is required" in payload["issues"]
    assert "native_scan_plugin_ref is required" in payload["issues"]


def test_wave3_pda_runtime_readiness_service_precheck_only_skips_pda_evidence_inputs(
    monkeypatch,
    capsys,
):
    """无 PDA 阶段可只读验证服务前置，但不能关闭 W6.D gate。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main([
        "--environment",
        "staging",
        "--service-url",
        "http://wms-staging.internal",
        "--service-precheck-only",
        "--json",
    ]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-service-precheck"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["facts"]["healthz_status"] == 200
    assert payload["facts"]["wave3_route_status"] == 401
    assert "pda_device_ref is required" not in payload["issues"]


def test_wave3_pda_runtime_readiness_service_precheck_only_from_env_ignores_pda_flags(
    monkeypatch,
    capsys,
):
    """service-precheck-only from-env 只取服务前置变量，不被 PDA 证据变量拼写影响。"""
    import check_wave3_pda_runtime_readiness as readiness

    monkeypatch.setenv("WAVE_3_PDA_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE_3_PDA_SERVICE_URL", "http://wms-staging.internal")
    monkeypatch.setenv("WAVE_3_PDA_REAL_PDA_USED", "TRUEE")

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        if url.endswith("/api/v1/inventory/batches"):
            return readiness.HttpJsonResult(401, {"code": "AUTH-001"})
        raise AssertionError(url)

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main(["--from-env", "--service-precheck-only", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-service-precheck"
    assert payload["facts"]["healthz_status"] == 200


def test_wave3_pda_runtime_readiness_rejects_missing_wave3_route(monkeypatch):
    """staging 可达但 Wave3 路由未挂载时，不能进入 PDA runtime evidence 采集。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        return readiness.HttpJsonResult(404, {"code": "NOT_FOUND"})

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    ok, facts, issues = readiness.check_readiness(
        readiness.parse_args(_valid_args()),
    )

    assert ok is False
    assert facts["healthz_status"] == 200
    assert any("Wave3 route expected 401 AUTH-001" in issue for issue in issues)


def test_wave3_pda_runtime_readiness_still_probes_service_when_pda_inputs_missing(
    monkeypatch,
    capsys,
):
    """缺 PDA 外部证据时，readiness 仍应报告可独立验证的 staging 服务事实。"""
    import check_wave3_pda_runtime_readiness as readiness

    def fake_http_json(url, timeout_seconds=10):
        if url.endswith("/healthz"):
            return readiness.HttpJsonResult(200, {"status": "ok"})
        return readiness.HttpJsonResult(401, {"code": "AUTH-001"})

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main([
        "--environment",
        "staging",
        "--service-url",
        "http://wms-staging.internal",
        "--json",
    ]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["facts"]["healthz_status"] == 200
    assert payload["facts"]["wave3_route_status"] == 401
    assert "pda_device_ref is required" in payload["issues"]
