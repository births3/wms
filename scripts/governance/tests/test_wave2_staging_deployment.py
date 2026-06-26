"""Wave 2 staging deployment scaffolding governance tests."""
from pathlib import Path


def test_wave2_staging_compose_declares_real_staging_services():
    """W6.C runtime evidence needs a real staging service chain, not local/mock/test."""
    compose = Path("deploy/docker-compose.staging.yml").read_text(encoding="utf-8")

    assert "services:" in compose
    assert "postgres-staging:" in compose
    assert "redis-staging:" in compose
    assert "wms-db-migrate-staging:" in compose
    assert "wms-api-staging:" in compose
    assert "backend/Dockerfile.wms-api" in compose
    assert "image: wms-api-staging:${WMS_VERSION:-latest}" in compose
    assert "image: wms-api-staging:latest" not in compose
    assert "pull_policy: build" in compose
    assert "entrypoint: [\"/app/wms-db-migrate\"]" in compose
    assert "command: [\"/app/wms-db-migrate\"]" not in compose
    assert "condition: service_completed_successfully" in compose
    assert "WMS_DB_URL=postgres://wms_staging" in compose
    assert "WMS_REDIS_URL=redis://redis-staging:6379/0" in compose
    assert "WMS_FEATURE_FLAGS_FILE=/app/deploy/feature_flags.toml" in compose
    assert "WMS_JWT_SECRET" in compose
    assert "/docker-entrypoint-initdb.d" not in compose
    assert "env_file:" not in compose
    assert "localhost" not in compose
    assert "127.0.0.1" not in compose
    assert "mock" not in compose.lower()
    assert "fake" not in compose.lower()


def test_wave2_staging_runbook_uses_staging_evidence_boundary():
    """Runbook must preserve the validator boundary: test purpose, staging environment."""
    runbook = Path("docs/runbooks/wave-2-staging-environment.md").read_text(
        encoding="utf-8",
    )

    assert "WAVE_2_ENVIRONMENT=staging" in runbook
    assert "测试用途的 staging 环境" in runbook
    assert "不能写成 `test`" in runbook
    assert "just wave-2-runtime-evidence-readiness" in runbook
    assert "just wave-2-runtime-evidence-smoke" in runbook
    assert "just wave-2-runtime-evidence-validate" in runbook
    assert "just wave-2-h1-token" in runbook
    assert "docker compose --env-file deploy/env/staging.env" in runbook
    assert "http://wms-staging.internal" in runbook
    assert "既有 nginx / 网关 / LB" in runbook
    assert "127.0.0.1:${WMS_STAGING_API_PORT:-18080}" in runbook
    assert "不纳入本仓库 compose" in runbook
    assert "your-internal-domain" not in runbook
    assert "test -f deploy/env/staging.env || cp" in runbook
    assert ". deploy/env/staging.env" in runbook
    assert "WMS_STAGING_DB_PASSWORD:?" in runbook
    assert "does not match WMS_STAGING_DB_PASSWORD" in runbook
    assert "wms-db-migrate-staging" in runbook
    assert "one-shot migrator" in runbook
    assert "PostgreSQL initdb" not in runbook
    assert "docs/retros/wave-2-runtime-evidence.json" in runbook
    assert "environment=test" not in runbook


def test_wave2_staging_runbook_records_confirmed_runtime_scope_choices():
    """Confirmed W6.C scope choices must stay explicit in the runbook."""
    staging_runbook = Path("docs/runbooks/wave-2-staging-environment.md").read_text(
        encoding="utf-8",
    )
    runtime_runbook = Path("docs/runbooks/wave-2-runtime-evidence.md").read_text(
        encoding="utf-8",
    )

    assert "2A" in staging_runbook
    assert "外部 staging 反代" in staging_runbook
    assert "不在 compose 内新增 Caddy/Nginx/Traefik" in staging_runbook
    assert "3A" in runtime_runbook
    assert "接受当前内存态 config-center" in runtime_runbook
    assert "4A" in runtime_runbook
    assert "H2 审计不纳入本次 W6.C evidence" in runtime_runbook
    assert "本次 W6.C evidence payload 不校验 H2 审计字段" in runtime_runbook
    assert "H2 审计追踪可记录" not in runtime_runbook
    assert "5A" in runtime_runbook
    assert "Wave2 静态完成项 + runtime evidence" in runtime_runbook

    source_step = runtime_runbook.index("1. 调用 `POST /api/v1/config-center/feature-flags/source`")
    fail_closed_step = runtime_runbook.index("2. 在迁移前调用 `GET /api/v1/inventory/batches`")
    migrate_step = runtime_runbook.index("3. 调用 `POST /api/v1/config-center/feature-flags/migrate`")
    assert source_step < fail_closed_step < migrate_step
    assert (
        "切源到 `config_center` → fail-closed → migrate → reconcile → export "
        "→ 切源到 `config_center` → 业务 200 smoke → 旧文件归档"
    ) in runtime_runbook


def test_wms_api_dockerfile_builds_release_binary_without_secrets():
    """Runtime image must copy the wms-api binary and no secret material."""
    dockerfile = Path("backend/Dockerfile.wms-api").read_text(encoding="utf-8")

    assert "cargo build --release --bin wms-api" in dockerfile
    assert "cargo build --release --bin wms-db-migrate" in dockerfile
    assert "cargo build --release --bin wms-deploy-audit" in dockerfile
    assert "COPY --from=builder /tmp/wms-api /app/wms-api" in dockerfile
    assert "COPY --from=builder /tmp/wms-db-migrate /app/wms-db-migrate" in dockerfile
    assert "COPY --from=builder /tmp/wms-deploy-audit /app/wms-deploy-audit" in dockerfile
    assert "ENTRYPOINT [\"/app/wms-api\"]" in dockerfile
    assert "WMS_JWT_SECRET" not in dockerfile
    assert "DATABASE_URL" not in dockerfile
    assert "PASSWORD" not in dockerfile


def test_wave2_staging_real_env_file_is_gitignored():
    """Real staging env files must stay outside version control."""
    gitignore = Path(".gitignore").read_text(encoding="utf-8")
    secrets_doc = Path("deploy/secrets.example.md").read_text(encoding="utf-8")

    assert "deploy/env/*.env" in gitignore
    assert "deploy/secrets/" in secrets_doc
    assert "Do not commit real secret files" in secrets_doc
