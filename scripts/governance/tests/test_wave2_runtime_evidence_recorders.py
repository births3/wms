"""Wave 2 runtime evidence record 脚本输出与边界校验测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _valid_wave2_record_args(output: Path, **overrides: str) -> list[str]:
    values = {
        "service_url": "https://wms-staging.internal",
        "smoke_log_ref": "ci/staging/wave2-feature-flags-smoke/123",
        "reconcile_log_ref": "ci/staging/wave2-feature-flags-reconcile/123",
        "archive_ref": "s3://wms-staging-audit/feature-flags/feature_flags.toml",
    }
    values.update(overrides)
    return [
        "--output", str(output),
        "--environment", "staging",
        "--service-url", values["service_url"],
        "--migrated-count", "1",
        "--reconcile-matched", "1",
        "--smoke-log-ref", values["smoke_log_ref"],
        "--reconcile-log-ref", values["reconcile_log_ref"],
        "--archive-ref", values["archive_ref"],
    ]


def test_record_wave2_runtime_evidence_writes_valid_evidence(tmp_path):
    """Wave 2 记录脚本生成的 evidence 必须能被同一规则接受。"""
    import record_wave2_runtime_evidence as recorder
    from report_wave2_completion import validate_wave2_runtime_payload

    output = tmp_path / "wave-2-runtime-evidence.json"

    assert recorder.main(_valid_wave2_record_args(output)) == 0

    ok, message = validate_wave2_runtime_payload(json.loads(output.read_text(encoding="utf-8")))

    assert ok is True
    assert "真实 dev/staging" in message
    evidence = json.loads(output.read_text(encoding="utf-8"))
    assert evidence["business_smoke"] == {
        "path": "/api/v1/inventory/batches",
        "enabled_flag": "m3_inventory_batches_config_center_smoke",
        "success_status": 200,
        "fail_closed_error_code": "M1_CONFIG_FLAG_MISSING",
    }


def test_validate_wave2_runtime_evidence_rejects_placeholder_values():
    """Wave 2 runtime evidence 不能保留 YYYY / <...> / 待填等模板占位。"""
    from report_wave2_completion import validate_wave2_runtime_payload

    evidence = {
        "environment": "staging",
        "service_url": "https://wms-staging.internal",
        "source_switched_to": "config_center",
        "migrated_count": 1,
        "reconcile": {
            "matched": 1,
            "missing_in_config_center": [],
            "mismatched": [],
        },
        "smoke_log_ref": "ci/staging/wave2-feature-flags-smoke/123-YYYYMMDD",
        "reconcile_log_ref": "ci/staging/wave2-feature-flags-reconcile/123",
        "archive_ref": "s3://wms-staging-audit/feature-flags/feature_flags.toml",
    }

    ok, message = validate_wave2_runtime_payload(evidence)

    assert ok is False
    assert "占位" in message
    assert "smoke_log_ref" in message


def test_validate_wave2_runtime_evidence_requires_service_url():
    """Wave 2 runtime evidence 必须记录真实 dev/staging service_url。"""
    from report_wave2_completion import validate_wave2_runtime_payload

    evidence = {
        "environment": "staging",
        "service_url": "",
        "source_switched_to": "config_center",
        "migrated_count": 1,
        "reconcile": {
            "matched": 1,
            "missing_in_config_center": [],
            "mismatched": [],
        },
        "business_smoke": {
            "path": "/api/v1/inventory/batches",
            "enabled_flag": "m3_inventory_batches_config_center_smoke",
            "success_status": 200,
            "fail_closed_error_code": "M1_CONFIG_FLAG_MISSING",
        },
        "smoke_log_ref": "ci/staging/wave2-feature-flags-smoke/123",
        "reconcile_log_ref": "ci/staging/wave2-feature-flags-reconcile/123",
        "archive_ref": "s3://wms-staging-audit/feature-flags/feature_flags.toml",
    }

    ok, message = validate_wave2_runtime_payload(evidence)

    assert ok is False
    assert "service_url" in message


def test_validate_wave2_runtime_evidence_rejects_empty_migration():
    """Wave 2 runtime evidence 不能用 0 条迁移 / 0 条对账冒充真实灰度。"""
    from report_wave2_completion import validate_wave2_runtime_payload

    evidence = {
        "environment": "staging",
        "service_url": "https://wms-staging.internal",
        "source_switched_to": "config_center",
        "migrated_count": 0,
        "reconcile": {
            "matched": 0,
            "missing_in_config_center": [],
            "mismatched": [],
        },
        "smoke_log_ref": "ci/staging/wave2-feature-flags-smoke/123",
        "reconcile_log_ref": "ci/staging/wave2-feature-flags-reconcile/123",
        "archive_ref": "s3://wms-staging-audit/feature-flags/feature_flags.toml",
    }

    ok, message = validate_wave2_runtime_payload(evidence)

    assert ok is False
    assert "migrated_count" in message

    evidence["migrated_count"] = 1
    ok, message = validate_wave2_runtime_payload(evidence)

    assert ok is False
    assert "reconcile.matched" in message


def test_validate_wave2_runtime_evidence_requires_business_smoke_contract():
    """Wave 2 runtime evidence 必须证明真实业务接口成功和缺失 flag fail-closed。"""
    from report_wave2_completion import validate_wave2_runtime_payload

    evidence = {
        "environment": "staging",
        "service_url": "https://wms-staging.internal",
        "source_switched_to": "config_center",
        "migrated_count": 1,
        "reconcile": {
            "matched": 1,
            "missing_in_config_center": [],
            "mismatched": [],
        },
        "smoke_log_ref": "ci/staging/wave2-feature-flags-smoke/123",
        "reconcile_log_ref": "ci/staging/wave2-feature-flags-reconcile/123",
        "archive_ref": "s3://wms-staging-audit/feature-flags/feature_flags.toml",
    }

    ok, message = validate_wave2_runtime_payload(evidence)

    assert ok is False
    assert "business_smoke" in message

    evidence["business_smoke"] = {
        "path": "/api/v1/inventory/batches",
        "enabled_flag": "m3_inventory_batches_config_center_smoke",
        "success_status": 200,
        "fail_closed_error_code": "W3-404",
    }

    ok, message = validate_wave2_runtime_payload(evidence)

    assert ok is False
    assert "M1_CONFIG_FLAG_MISSING" in message


def test_record_wave2_runtime_evidence_rejects_invalid_refs_before_write(tmp_path):
    """Wave 2 记录脚本不能写入 local/prod/production/mock/fake/stub/example 边界。"""
    import record_wave2_runtime_evidence as recorder

    output = tmp_path / "wave-2-runtime-evidence.json"

    assert recorder.main(_valid_wave2_record_args(
        output,
        service_url="http://localhost:8080",
    )) == 1

    assert not output.exists()

    assert recorder.main(_valid_wave2_record_args(
        output,
        service_url="https://wms-local.internal",
    )) == 1

    assert not output.exists()

    assert recorder.main(_valid_wave2_record_args(
        output,
        service_url="https://wms-staging.example.internal",
    )) == 1

    assert not output.exists()

    assert recorder.main(_valid_wave2_record_args(
        output,
        service_url="https://wms-staging.your-internal-domain",
    )) == 1

    assert not output.exists()

    assert recorder.main(_valid_wave2_record_args(
        output,
        service_url="https://wms-staging.examplesite.internal",
    )) == 0
    output.unlink()

    assert recorder.main(_valid_wave2_record_args(
        output,
        service_url="http://0.0.0.0:8080",
    )) == 1

    assert not output.exists()

    assert recorder.main(_valid_wave2_record_args(
        output,
        smoke_log_ref="ci/staging/wave2-feature-flags-smoke/mock-run",
    )) == 1

    assert not output.exists()

    assert recorder.main(_valid_wave2_record_args(
        output,
        service_url="https://wms.internal",
    )) == 1

    assert not output.exists()
