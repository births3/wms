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
