"""Wave 2 config-center runtime evidence collector tests."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _base_args(output: Path) -> list[str]:
    return [
        "--environment",
        "staging",
        "--service-url",
        "https://wms-staging.internal",
        "--smoke-log-ref",
        "ci/staging/wave2-feature-flags-smoke/123",
        "--reconcile-log-ref",
        "ci/staging/wave2-feature-flags-reconcile/123",
        "--archive-ref",
        "s3://wms-staging-audit/feature-flags/feature_flags.toml",
        "--output",
        str(output),
    ]


def _success_responses(collector, archive_ref: str):
    return [
        collector.HttpJsonResult(200, {"active_source": "config_center"}),
        collector.HttpJsonResult(
            404,
            {
                "code": "M1_CONFIG_FLAG_MISSING",
                "message": "Feature Flag 不存在",
                "severity": "error",
                "details": {},
                "trace_id": "trace-staging",
            },
        ),
        collector.HttpJsonResult(200, {"migrated_count": 1}),
        collector.HttpJsonResult(
            200,
            {
                "matched": 1,
                "missing_in_config_center": [],
                "mismatched": [],
            },
        ),
        collector.HttpJsonResult(
            200,
            {
                "source": "config_center",
                "flags": [{"key": "m3_inventory_batches_config_center_smoke"}],
            },
        ),
        collector.HttpJsonResult(200, {"active_source": "config_center"}),
        collector.HttpJsonResult(200, {"data": [], "page": {"count": 0}}),
        collector.HttpJsonResult(
            200,
            {
                "archived_source": "deploy/feature_flags.toml",
                "archive_ref": archive_ref,
                "archived_at": "2026-06-07T00:00:00Z",
            },
        ),
    ]


def test_collect_wave2_runtime_evidence_runs_smoke_and_writes_valid_json(
    tmp_path,
    monkeypatch,
):
    """Collector 应调用真实链路顺序，并写出可被 Wave2 validator 接受的 evidence。"""
    import collect_wave2_runtime_evidence as collector
    from report_wave2_completion import validate_wave2_runtime_payload

    output = tmp_path / "wave-2-runtime-evidence.json"
    archive_ref = "s3://wms-staging-audit/feature-flags/feature_flags.toml"
    responses = _success_responses(collector, archive_ref)
    calls = []
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")

    def fake_http(method, url, token, body=None, timeout_seconds=30):
        calls.append((method, url, token, body, timeout_seconds))
        return responses.pop(0)

    monkeypatch.setattr(collector, "http_json", fake_http)

    assert collector.main(_base_args(output)) == 0
    assert responses == []
    assert [call[0] for call in calls] == [
        "POST",
        "GET",
        "POST",
        "GET",
        "GET",
        "POST",
        "GET",
        "POST",
    ]
    assert calls[0][3] == {"source": "config_center"}
    assert calls[-1][3] == {"archive_ref": archive_ref}

    payload = json.loads(output.read_text(encoding="utf-8"))
    ok, message = validate_wave2_runtime_payload(payload)
    assert ok is True
    assert "真实 dev/staging" in message
    assert payload["migrated_count"] == 1
    assert payload["reconcile"]["matched"] == 1
    assert payload["business_smoke"]["fail_closed_error_code"] == "M1_CONFIG_FLAG_MISSING"


def test_collect_wave2_runtime_evidence_uses_environment_defaults(
    tmp_path,
    monkeypatch,
):
    """现场可通过 WAVE_2_* env 运行 smoke，不必把引用写进命令行。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    archive_ref = "s3://wms-staging-audit/feature-flags/feature_flags.toml"
    responses = _success_responses(collector, archive_ref)
    calls = []
    monkeypatch.setenv("WAVE_2_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE_2_SERVICE_URL", "https://wms-staging.internal")
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setenv("WAVE_2_SMOKE_LOG_REF", "ci/staging/wave2-feature-flags-smoke/123")
    monkeypatch.setenv("WAVE_2_RECONCILE_LOG_REF", "ci/staging/wave2-feature-flags-reconcile/123")
    monkeypatch.setenv("WAVE_2_ARCHIVE_REF", archive_ref)
    monkeypatch.setenv("WAVE_2_CURL_TIMEOUT_SECONDS", "9")

    def fake_http(method, url, token, body=None, timeout_seconds=30):
        calls.append((method, url, token, body, timeout_seconds))
        return responses.pop(0)

    monkeypatch.setattr(collector, "http_json", fake_http)

    assert collector.main(["--output", str(output)]) == 0
    assert responses == []
    assert {call[4] for call in calls} == {9}


def test_collect_wave2_runtime_evidence_requires_access_token_before_http(
    tmp_path,
    monkeypatch,
):
    """没有真实 H1 token 时，collector 不应触发任何 HTTP 调用。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    monkeypatch.delenv("WAVE_2_H1_TOKEN", raising=False)
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("no http")),
    )

    assert collector.main(_base_args(output)) == 2
    assert not output.exists()


def test_collect_wave2_runtime_evidence_rejects_invalid_environment_before_http(
    tmp_path,
    monkeypatch,
):
    """非 dev/staging 环境必须在 HTTP 前失败。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    command = _base_args(output)
    command[command.index("staging")] = "qa"
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-qa")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("no http")),
    )

    assert collector.main(command) == 2
    assert not output.exists()


def test_collect_wave2_runtime_evidence_rejects_local_refs_before_http(
    tmp_path,
    monkeypatch,
):
    """Collector 不能连接 local/mock/prod 边界，也不能写入对应 evidence。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("no http")),
    )

    command = _base_args(output)
    command[command.index("https://wms-staging.internal")] = "http://localhost:8080"

    assert collector.main(command) == 2
    assert not output.exists()

    command = _base_args(output)
    command[command.index("https://wms-staging.internal")] = "https://wms-staging.your-internal-domain"

    assert collector.main([*command, "--check-only"]) == 2
    assert not output.exists()

    command = _base_args(output)
    command[command.index("https://wms-staging.internal")] = "https://wms-staging.examplesite.internal"

    assert collector.main([*command, "--check-only"]) == 0
    assert not output.exists()


def test_collect_wave2_runtime_evidence_requires_force_before_http(
    tmp_path,
    monkeypatch,
):
    """已有 evidence 时必须显式 --force，且默认不重复触发真实 smoke。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    output.write_text("{}", encoding="utf-8")
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("no http")),
    )

    assert collector.main(_base_args(output)) == 2
    assert output.read_text(encoding="utf-8") == "{}"


def test_collect_wave2_runtime_evidence_check_only_ignores_existing_output(
    tmp_path,
    monkeypatch,
):
    """readiness 不写 evidence，所以已有 output 不应要求 --force。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    output.write_text("{}", encoding="utf-8")
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("no http")),
    )

    assert collector.main([*_base_args(output), "--check-only"]) == 0
    assert output.read_text(encoding="utf-8") == "{}"


def test_collect_wave2_runtime_evidence_rejects_missing_fail_closed_response(
    tmp_path,
    monkeypatch,
):
    """业务 smoke 未证明 M1_CONFIG_FLAG_MISSING fail-closed 时不能写 evidence。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    archive_ref = "s3://wms-staging-audit/feature-flags/feature_flags.toml"
    responses = _success_responses(collector, archive_ref)
    responses[1] = collector.HttpJsonResult(200, {"data": [], "page": {"count": 0}})
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: responses.pop(0),
    )

    assert collector.main(_base_args(output)) == 2
    assert not output.exists()


def test_collect_wave2_runtime_evidence_rejects_export_without_smoke_flag(
    tmp_path,
    monkeypatch,
):
    """导出结果必须证明真实 rollout flag 已进入 config-center。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    archive_ref = "s3://wms-staging-audit/feature-flags/feature_flags.toml"
    responses = _success_responses(collector, archive_ref)
    responses[4] = collector.HttpJsonResult(
        200,
        {"source": "config_center", "flags": [{"key": "other_flag"}]},
    )
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: responses.pop(0),
    )

    assert collector.main(_base_args(output)) == 2
    assert not output.exists()


def test_collect_wave2_runtime_evidence_rejects_archive_ref_mismatch(
    tmp_path,
    monkeypatch,
):
    """归档接口必须回显同一个 archive_ref，避免记录了未归档引用。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    archive_ref = "s3://wms-staging-audit/feature-flags/feature_flags.toml"
    responses = _success_responses(collector, archive_ref)
    responses[-1] = collector.HttpJsonResult(
        200,
        {
            "archived_source": "deploy/feature_flags.toml",
            "archive_ref": "s3://wms-staging-audit/feature-flags/other.toml",
            "archived_at": "2026-06-07T00:00:00Z",
        },
    )
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: responses.pop(0),
    )

    assert collector.main(_base_args(output)) == 2
    assert not output.exists()


def test_collect_wave2_runtime_evidence_check_only_validates_inputs_without_http(
    tmp_path,
    monkeypatch,
):
    """readiness 模式只校验 env / 边界 / token，不发请求也不写 evidence。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("no http")),
    )

    assert collector.main([*_base_args(output), "--check-only"]) == 0
    assert not output.exists()


def test_collect_wave2_runtime_evidence_rejects_bad_timeout_env_before_http(
    tmp_path,
    monkeypatch,
):
    """timeout env 非正整数时应可控失败，不能 traceback 或发 HTTP。"""
    import collect_wave2_runtime_evidence as collector

    output = tmp_path / "wave-2-runtime-evidence.json"
    monkeypatch.setenv("WAVE_2_H1_TOKEN", "token-staging")
    monkeypatch.setenv("WAVE_2_CURL_TIMEOUT_SECONDS", "0")
    monkeypatch.setattr(
        collector,
        "http_json",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("no http")),
    )

    assert collector.main(_base_args(output)) == 2
    assert not output.exists()
