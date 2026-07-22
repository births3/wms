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
