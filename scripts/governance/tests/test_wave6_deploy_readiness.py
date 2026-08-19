"""Wave 6 gray release readiness 预检测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _valid_args() -> list[str]:
    return [
        "--environment",
        "staging",
        "--deployment-mode",
        "docker-compose",
        "--release-version",
        "wms-api-20260607.1",
        "--service-url",
        "http://wms-staging.internal",
        "--release-plan-ref",
        "ticket://wms-staging-release-plan/W6H-20260607",
        "--artifact-ref",
        "registry://wms-staging/wms-api@sha256:abcdef1234567890",
        "--canary-config-ref",
        "git://wms-staging/deploy/canary/W6H-20260607",
        "--smoke-gate-ref",
        "ci/staging/wave6-deploy-smoke/123",
        "--observability-dashboard-ref",
        "grafana/staging/wms/wave6-deploy/123",
        "--rollback-drill-log-ref",
        "ci/staging/wave6-deploy-rollback/123",
        "--approval-record-ref",
        "ticket://wms-staging-release-approval/W6H-20260607",
        "--audit-event-query-ref",
        "postgres://wms-staging/audit_event/deploy/W6H-20260607",
        "--canary-stages-exercised",
        "1",
        "--smoke-checks-passed",
        "3",
        "--rollback-drills-exercised",
        "1",
        "--canary-used",
        "--full-release-blocked",
        "--rollback-verified",
        "--audit-event-verified",
        "--dual-approval-recorded",
    ]


def _valid_env() -> dict[str, str]:
    return {
        "WAVE_6_ENVIRONMENT": "staging",
        "WAVE_6_DEPLOYMENT_MODE": "docker-compose",
        "WAVE_6_RELEASE_VERSION": "wms-api-20260607.1",
        "WAVE_6_SERVICE_URL": "http://wms-staging.internal",
        "WAVE_6_RELEASE_PLAN_REF": "ticket://wms-staging-release-plan/W6H-20260607",
        "WAVE_6_ARTIFACT_REF": "registry://wms-staging/wms-api@sha256:abcdef1234567890",
        "WAVE_6_CANARY_CONFIG_REF": "git://wms-staging/deploy/canary/W6H-20260607",
        "WAVE_6_SMOKE_GATE_REF": "ci/staging/wave6-deploy-smoke/123",
        "WAVE_6_OBSERVABILITY_DASHBOARD_REF": "grafana/staging/wms/wave6-deploy/123",
        "WAVE_6_ROLLBACK_DRILL_LOG_REF": "ci/staging/wave6-deploy-rollback/123",
        "WAVE_6_APPROVAL_RECORD_REF": "ticket://wms-staging-release-approval/W6H-20260607",
        "WAVE_6_AUDIT_EVENT_QUERY_REF": "postgres://wms-staging/audit_event/deploy/W6H-20260607",
        "WAVE_6_CANARY_STAGES_EXERCISED": "1",
        "WAVE_6_SMOKE_CHECKS_PASSED": "3",
        "WAVE_6_ROLLBACK_DRILLS_EXERCISED": "1",
        "WAVE_6_CANARY_USED": "true",
        "WAVE_6_FULL_RELEASE_BLOCKED": "true",
        "WAVE_6_ROLLBACK_VERIFIED": "true",
        "WAVE_6_AUDIT_EVENT_VERIFIED": "true",
        "WAVE_6_DUAL_APPROVAL_RECORDED": "true",
    }


def test_wave6_deploy_readiness_rejects_missing_external_inputs(capsys):
    """readiness 必须列出缺失灰度发布证据输入，且不能写 runtime evidence。"""
    import check_wave6_deploy_readiness as readiness

    assert readiness.main(["--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "docs/retros/wave-6-deploy-evidence.json" not in "\n".join(
        payload["issues"],
    )
    for expected in (
        "environment is required",
        "service_url is required",
        "deployment_mode is required",
        "release_plan_ref is required",
        "artifact_ref is required",
        "canary_stages_exercised is required",
        "canary_used must be true",
        "dual_approval_recorded must be true",
    ):
        assert expected in payload["issues"]


def test_wave6_deploy_readiness_checks_staging_health_but_does_not_write_evidence(
    monkeypatch,
    capsys,
):
    """readiness 应探测 staging healthz，并明确不写 deploy evidence。"""
    import check_wave6_deploy_readiness as readiness

    calls = []

    def fake_http_json(url, timeout_seconds=10):
        calls.append((url, timeout_seconds))
        return readiness.HttpJsonResult(200, {"status": "ok"})

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    assert readiness.main([*_valid_args(), "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-6-deploy-evidence.json"
    assert payload["facts"]["healthz_status"] == 200
    assert payload["facts"]["healthz_payload_status"] == "ok"
    assert calls == [("http://wms-staging.internal/healthz", 10)]


def test_wave6_deploy_readiness_still_probes_service_when_refs_missing(
    monkeypatch,
    capsys,
):
    """缺外部引用时，readiness 仍应报告可独立验证的 staging 服务事实。"""
    import check_wave6_deploy_readiness as readiness

    monkeypatch.setattr(
        readiness,
        "http_json",
        lambda url, timeout_seconds=10: readiness.HttpJsonResult(200, {"status": "ok"}),
    )

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
    assert "release_plan_ref is required" in payload["issues"]


def test_wave6_deploy_readiness_from_env_checks_staging_health(
    monkeypatch,
    capsys,
):
    """W6.H readiness 应能直接从 WAVE_6_* 现场变量读取材料。"""
    import check_wave6_deploy_readiness as readiness

    calls = []

    def fake_http_json(url, timeout_seconds=10):
        calls.append((url, timeout_seconds))
        return readiness.HttpJsonResult(200, {"status": "ok"})

    monkeypatch.setattr(readiness, "http_json", fake_http_json)
    for key, value in _valid_env().items():
        monkeypatch.setenv(key, value)

    assert readiness.main(["--from-env", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["facts"]["environment"] == "staging"
    assert payload["facts"]["healthz_status"] == 200
    assert calls == [("http://wms-staging.internal/healthz", 10)]


def test_wave6_deploy_readiness_from_env_reports_missing_vars(
    monkeypatch,
    capsys,
):
    """W6.H readiness from-env 缺材料时输出缺失变量，不写 evidence。"""
    import check_wave6_deploy_readiness as readiness

    monkeypatch.setenv("WAVE_6_ENVIRONMENT", "staging")

    assert readiness.main(["--from-env", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "WAVE_6_SERVICE_URL" in payload["missing_env_vars"]
    assert "WAVE_6_AUDIT_EVENT_QUERY_REF" in payload["missing_env_vars"]
    assert "WAVE_6_RELEASE_PLAN_REF" in payload["missing_env_vars"]


def test_wave6_deploy_readiness_rejects_non_staging_service_url_without_http(
    monkeypatch,
):
    """readiness 必须先拒绝非 staging service URL，不能向 local/dev/prod 发 HTTP。"""
    import check_wave6_deploy_readiness as readiness

    calls = []

    def fake_http_json(url, timeout_seconds=10):
        calls.append((url, timeout_seconds))
        return readiness.HttpJsonResult(200, {"status": "ok"})

    monkeypatch.setattr(readiness, "http_json", fake_http_json)

    args = readiness.parse_args([
        *[
            value.replace("staging", "dev").replace("wms-staging", "wms-dev")
            if value == "http://wms-staging.internal"
            else value
            for value in _valid_args()
        ],
    ])
    ok, facts, issues = readiness.check_readiness(args)

    assert ok is False
    assert calls == []
    assert "healthz_status" not in facts
    assert any("service_url" in issue for issue in issues)


def test_wave6_deploy_readiness_rejects_unhealthy_staging_service(monkeypatch):
    """staging healthz 不健康时，不能进入 deploy evidence 采集。"""
    import check_wave6_deploy_readiness as readiness

    monkeypatch.setattr(
        readiness,
        "http_json",
        lambda url, timeout_seconds=10: readiness.HttpJsonResult(503, {"status": "down"}),
    )

    ok, facts, issues = readiness.check_readiness(
        readiness.parse_args(_valid_args()),
    )

    assert ok is False
    assert facts["healthz_status"] == 503
    assert any("healthz expected HTTP 200" in issue for issue in issues)


def test_wave6_deploy_readiness_rejects_dev_environment_even_with_valid_refs(monkeypatch):
    """W6.H readiness 只能检查 staging 灰度发布材料，不能接受 dev 关闭路径。"""
    import check_wave6_deploy_readiness as readiness

    monkeypatch.setattr(
        readiness,
        "http_json",
        lambda url, timeout_seconds=10: readiness.HttpJsonResult(200, {"status": "ok"}),
    )
    args = [
        *[
            value.replace("staging", "dev").replace("wms-staging", "wms-dev")
            for value in _valid_args()
        ],
        "--json",
    ]

    assert readiness.main(args) == 1
