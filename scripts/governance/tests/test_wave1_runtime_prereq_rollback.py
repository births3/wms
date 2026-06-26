"""Wave 1 rollback runtime evidence 前置检查测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_runtime_test_helpers import clear_wave1_prereq_env, wave1_prereq_module


def test_wave1_runtime_prereq_rollback_rejects_missing_signal(monkeypatch, capsys):
    """自动回滚前置检查必须有真实 smoke 或 Prometheus 信号。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_K8S_CONTEXT", "wms-staging")
    monkeypatch.setenv("WAVE1_K8S_NAMESPACE", "wms-staging")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-staging-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")

    exit_code = prereq.main(["--mode", "rollback-k8s"])

    assert exit_code == 2
    assert "missing runtime signal" in capsys.readouterr().err


def test_wave1_runtime_prereq_rollback_rejects_local_prod_or_stub_boundaries(
    monkeypatch,
    capsys,
):
    """自动回滚前置检查不能接受本机、生产或 stub 信号/日志。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_K8S_CONTEXT", "wms-staging")
    monkeypatch.setenv("WAVE1_K8S_NAMESPACE", "wms-staging")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-prod-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")
    monkeypatch.setenv("SMOKE_URL", "http://127.0.0.1/staging-stub/healthz")

    exit_code = prereq.main(["--mode", "rollback-k8s"])

    assert exit_code == 2
    err = capsys.readouterr().err
    assert "WAVE1_ROLLBACK_LOG_REF" in err
    assert "SMOKE_URL" in err


def test_wave1_runtime_prereq_rollback_rejects_local_named_signal(
    monkeypatch,
    capsys,
):
    """自动回滚前置检查不能接受 local 命名的 smoke 信号。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_K8S_CONTEXT", "wms-staging")
    monkeypatch.setenv("WAVE1_K8S_NAMESPACE", "wms-staging")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-staging-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")
    monkeypatch.setenv("SMOKE_URL", "https://smoke.local.wms.internal/staging/healthz")

    exit_code = prereq.main(["--mode", "rollback-k8s"])

    assert exit_code == 2
    assert "local/prod/production" in capsys.readouterr().err


def test_wave1_runtime_prereq_rollback_compose_accepts_valid_prometheus_signal(
    tmp_path,
    monkeypatch,
):
    """docker-compose 前置检查接受真实 dev Prometheus 信号配置。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    compose_dir = tmp_path / "wms-dev"
    compose_dir.mkdir()
    compose_file = compose_dir / "docker-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")
    compose_env_file = compose_dir / "staging.env"
    compose_env_file.write_text("WMS_STAGING_API_PORT=18080\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "dev")
    monkeypatch.setenv("WAVE1_PREVIOUS_VERSION", "previous-dev-sha")
    monkeypatch.setenv("WAVE1_COMPOSE_FILE", str(compose_file))
    monkeypatch.setenv("WAVE1_COMPOSE_ENV_FILE", str(compose_env_file))
    monkeypatch.setenv("PROMETHEUS_URL", "https://prometheus.dev.wms.internal")
    monkeypatch.setenv("PROMETHEUS_QUERY", 'wms_wave1_rollback_signal{environment="dev"}')
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-dev-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-dev-evidence/wave1/prometheus.log")

    exit_code = prereq.main(["--mode", "rollback-compose"])

    assert exit_code == 0


def test_wave1_runtime_prereq_rollback_compose_rejects_missing_env_file(
    tmp_path,
    monkeypatch,
    capsys,
):
    """docker-compose 回滚 env file 若配置则必须存在，避免 rollback 启动空 env。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    compose_dir = tmp_path / "wms-staging"
    compose_dir.mkdir()
    compose_file = compose_dir / "docker-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_PREVIOUS_VERSION", "previous-staging-sha")
    monkeypatch.setenv("WAVE1_COMPOSE_FILE", str(compose_file))
    monkeypatch.setenv("WAVE1_COMPOSE_ENV_FILE", str(compose_dir / "missing.env"))
    monkeypatch.setenv("SMOKE_URL", "https://smoke.staging.wms.internal/wms/healthz")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-staging-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")

    exit_code = prereq.main(["--mode", "rollback-compose"])

    assert exit_code == 2
    assert "WAVE1_COMPOSE_ENV_FILE" in capsys.readouterr().err


def test_wave1_runtime_prereq_rollback_compose_requires_unresolved_compose_env(
    tmp_path,
    monkeypatch,
    capsys,
):
    """compose 中无默认值的变量必须由 shell 环境或 compose env-file 提供。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    compose_dir = tmp_path / "wms-staging"
    compose_dir.mkdir()
    compose_file = compose_dir / "docker-compose.yml"
    compose_file.write_text(
        "\n".join(
            [
                "services:",
                "  wms-api-staging:",
                "    image: wms-api-staging:${WMS_VERSION:-latest}",
                "    environment:",
                "      - WMS_DB_URL=postgres://wms:${WMS_STAGING_DB_PASSWORD}@postgres:5432/wms",
                "      - WMS_JWT_SECRET=${WMS_JWT_SECRET}",
                "    ports:",
                "      - ${WMS_STAGING_API_PORT:-18080}:8080",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_PREVIOUS_VERSION", "previous-staging-sha")
    monkeypatch.setenv("WAVE1_COMPOSE_FILE", str(compose_file))
    monkeypatch.setenv("SMOKE_URL", "https://smoke.staging.wms.internal/wms/healthz")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-staging-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")

    exit_code = prereq.main(["--mode", "rollback-compose"])

    assert exit_code == 2
    err = capsys.readouterr().err
    assert "compose env is missing required values" in err
    assert "WMS_STAGING_DB_PASSWORD" in err
    assert "WMS_JWT_SECRET" in err
    assert "WMS_VERSION" not in err
    assert "WMS_STAGING_API_PORT" not in err


def test_wave1_runtime_prereq_rollback_compose_reads_required_values_from_env_file(
    tmp_path,
    monkeypatch,
):
    """compose env-file 中的必填变量可满足 docker-compose 回滚前置检查。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    compose_dir = tmp_path / "wms-staging"
    compose_dir.mkdir()
    compose_file = compose_dir / "docker-compose.yml"
    compose_file.write_text(
        "\n".join(
            [
                "services:",
                "  wms-api-staging:",
                "    image: wms-api-staging:${WMS_VERSION:-latest}",
                "    environment:",
                "      - WMS_DB_URL=postgres://wms:${WMS_STAGING_DB_PASSWORD}@postgres:5432/wms",
                "      - WMS_JWT_SECRET=${WMS_JWT_SECRET}",
                "    ports:",
                "      - ${WMS_STAGING_API_PORT:-18080}:8080",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    compose_env_file = compose_dir / "staging.env"
    compose_env_file.write_text(
        "WMS_STAGING_DB_PASSWORD=secret\nWMS_JWT_SECRET=jwt-secret\n",
        encoding="utf-8",
    )
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_PREVIOUS_VERSION", "previous-staging-sha")
    monkeypatch.setenv("WAVE1_COMPOSE_FILE", str(compose_file))
    monkeypatch.setenv("WAVE1_COMPOSE_ENV_FILE", str(compose_env_file))
    monkeypatch.setenv("SMOKE_URL", "https://smoke.staging.wms.internal/wms/healthz")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-staging-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")

    exit_code = prereq.main(["--mode", "rollback-compose"])

    assert exit_code == 0


def test_wave1_runtime_prereq_rollback_rejects_prometheus_without_env_url(
    tmp_path,
    monkeypatch,
    capsys,
):
    """Prometheus evidence 最终只记录 query endpoint，所以 URL 本身也必须带环境标记。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    compose_dir = tmp_path / "wms-dev"
    compose_dir.mkdir()
    compose_file = compose_dir / "docker-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "dev")
    monkeypatch.setenv("WAVE1_PREVIOUS_VERSION", "previous-dev-sha")
    monkeypatch.setenv("WAVE1_COMPOSE_FILE", str(compose_file))
    monkeypatch.setenv("PROMETHEUS_URL", "https://prometheus.wms.internal")
    monkeypatch.setenv("PROMETHEUS_QUERY", 'wms_wave1_rollback_signal{environment="dev"}')
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-dev-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-dev-evidence/wave1/prometheus.log")

    exit_code = prereq.main(["--mode", "rollback-compose"])

    assert exit_code == 2
    assert "Prometheus URL" in capsys.readouterr().err
