"""Wave 3 PDA runtime readiness 预检测试。"""
import json
import sys
from pathlib import Path


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
