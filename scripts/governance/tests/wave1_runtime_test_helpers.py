"""Shared fixtures for Wave 1 runtime evidence governance tests."""
import json
from pathlib import Path


WAVE1_PREREQ_ENV_VARS = [
    "WAVE1_H2_DATABASE_URL",
    "WAVE1_H2_WRK_OUTPUT",
    "WAVE1_H2_BENCHMARK_LOG_REF",
    "WAVE1_H2_CRON_LOG_REF",
    "WAVE1_H2_DURATION_SECONDS",
    "WAVE1_H2_TARGET_QPS",
    "WAVE1_H2_SEAL_FAILURE_COUNT",
    "WAVE1_ROLLBACK_ENVIRONMENT",
    "WAVE1_K8S_CONTEXT",
    "WAVE1_K8S_NAMESPACE",
    "WAVE1_PREVIOUS_VERSION",
    "WAVE1_COMPOSE_FILE",
    "WAVE1_COMPOSE_ENV_FILE",
    "WAVE1_ROLLBACK_LOG_REF",
    "WAVE1_EXTERNAL_LOG_REF",
    "SMOKE_URL",
    "PROMETHEUS_URL",
    "PROMETHEUS_QUERY",
]


def wave1_prereq_module(monkeypatch):
    import check_wave1_runtime_evidence_prereqs as prereq

    monkeypatch.setattr(prereq.shutil, "which", lambda command: f"/usr/bin/{command}")
    return prereq


def clear_wave1_prereq_env(monkeypatch) -> None:
    for name in WAVE1_PREREQ_ENV_VARS:
        monkeypatch.delenv(name, raising=False)


def valid_h2_runtime_evidence() -> dict[str, object]:
    return {
        "environment": "dev",
        "captured_at": "2026-06-03T12:00:00+08:00",
        "database": {
            "host": "pg-dev.wms.internal",
            "resolved_ips": ["10.0.0.8"],
        },
        "performance": {
            "tool": "wrk",
            "baseline_rows": 60_000_000,
            "target_qps": 1000,
            "observed_qps": 1001.5,
            "duration_seconds": 3600,
            "p99_ms": 199.5,
            "benchmark_log_ref": "s3://wms-dev-evidence/wave1/h2/wrk.log",
        },
        "seal_cron": {
            "consecutive_success_days": 7,
            "failure_count": 0,
            "last_seal_verified": True,
            "cron_log_ref": "s3://wms-dev-evidence/wave1/h2/seal-cron.log",
        },
    }


def valid_w1d_runtime_evidence() -> dict[str, object]:
    return {
        "environment": "staging",
        "captured_at": "2026-06-03T12:00:00+08:00",
        "signal_type": "http",
        "signal_url": "https://smoke.staging.wms.internal/wms/healthz",
        "rollback_triggered": True,
        "rollback_exit_code": 0,
        "rollback_log_ref": "s3://wms-staging-evidence/wave1/rollback.log",
        "external_log_ref": "s3://wms-staging-evidence/wave1/smoke-alert.log",
    }


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


def write_runtime_evidence(repo_root: Path, filename: str, payload: dict[str, object]) -> Path:
    evidence = repo_root / "docs" / "retros" / filename
    evidence.parent.mkdir(parents=True, exist_ok=True)
    write_json(evidence, payload)
    return evidence
