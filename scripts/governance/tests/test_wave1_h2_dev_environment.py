"""Wave 1 H2 dev runtime evidence environment scaffolding tests."""
from pathlib import Path


def test_wave1_h2_dev_compose_declares_real_dev_postgres_only():
    """W6.A H2 evidence needs a dev PostgreSQL boundary, not staging/local/mock."""
    compose = Path("deploy/docker-compose.dev-h2.yml").read_text(encoding="utf-8")

    assert "name: wms-dev-h2" in compose
    assert "postgres-dev-h2:" in compose
    assert "POSTGRES_DB: wms_dev_h2" in compose
    assert "POSTGRES_USER: wms_dev_h2" in compose
    assert "POSTGRES_PASSWORD_FILE: /run/secrets/wms_dev_h2_db_password" in compose
    assert "wms-db-migrate-dev-h2:" in compose
    assert "backend/Dockerfile.wms-api" in compose
    assert "image: wms-api-dev-h2:${WMS_VERSION:-latest}" in compose
    assert "pull_policy: build" in compose
    assert "entrypoint: [\"/app/wms-db-migrate\"]" in compose
    assert "WMS_DB_URL=postgres://wms_dev_h2" in compose
    assert "condition: service_healthy" in compose
    assert "${WMS_DEV_H2_DB_PORT:-15432}:5432" in compose
    assert "restart: \"no\"" in compose
    assert "postgres_dev_h2_data:" in compose
    assert "wms_dev_h2_db_password:" in compose
    assert "localhost" not in compose
    assert "127.0.0.1" not in compose
    assert "staging" not in compose
    assert "mock" not in compose.lower()
    assert "fake" not in compose.lower()


def test_wave1_h2_dev_env_example_declares_only_dev_values():
    """The committed env example must not contain real secrets or staging names."""
    env_example = Path("deploy/env/dev-h2.env.example").read_text(encoding="utf-8")
    secrets_doc = Path("deploy/secrets.example.md").read_text(encoding="utf-8")

    assert "WMS_DEV_H2_DB_PASSWORD=replace-with-dev-h2-db-password" in env_example
    assert "WMS_DEV_H2_DB_PORT=15432" in env_example
    assert "WMS_STAGING" not in env_example
    assert "localhost" not in env_example
    assert "127.0.0.1" not in env_example
    assert "deploy/env/*.env" in Path(".gitignore").read_text(encoding="utf-8")
    assert "deploy/secrets/" in secrets_doc
    assert "wms_dev_h2_db_password.txt" in secrets_doc


def test_wave1_h2_dev_runbook_exports_required_w6a_inputs():
    """Runbook must guide W6.A from dev DB bootstrap to readiness without writing evidence."""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )

    assert "W6.A" in runbook
    assert "dev PostgreSQL" in runbook
    assert "不能使用 staging" in runbook
    assert "不能使用 local" in runbook
    assert "docker compose --env-file deploy/env/dev-h2.env" in runbook
    assert "deploy/docker-compose.dev-h2.yml" in runbook
    assert "wms-db-migrate-dev-h2" in runbook
    assert "one-shot migrator" in runbook
    assert "export WAVE1_H2_DATABASE_URL=" in runbook
    assert "如果从宿主机连接 compose 暴露端口" in runbook
    assert "dev-h2.wms.internal" in runbook
    assert "只用于本机 dry-run" in runbook
    assert "不能作为正式 W6.A evidence" in runbook
    assert "export WAVE1_H2_WRK_OUTPUT=" in runbook
    assert "export WAVE1_H2_BENCHMARK_LOG_REF=" in runbook
    assert "export WAVE1_H2_CRON_LOG_REF=" in runbook
    assert "just wave-1-runtime-prereq-h2" in runbook
    assert "just wave-1-h2-runtime-readiness" in runbook
    assert "just wave-1-h2-runtime-evidence" in runbook
    assert "docs/retros/wave-1-h2-runtime-evidence.json" in runbook
    assert "不会写入 runtime evidence" in runbook
    assert "不能关闭 W6.A gate" in runbook
    assert "60,000,000" in runbook
    assert "7 个自然日" in runbook


def test_wave1_h2_dev_runbook_documents_baseline_loader_guardrails():
    """基线材料 loader 必须和正式 runtime evidence 明确隔离。"""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )

    assert "wms-audit-baseline-load" in runbook
    assert "just wave-1-h2-baseline-dry-run" in runbook
    assert "just wave-1-h2-baseline-load" in runbook
    assert "export DATABASE_URL=\"$DEV_WAVE1_H2_DATABASE_URL\"" in runbook
    assert "--target-total-rows 60000000" in runbook
    assert "planned_batches" in runbook
    assert "rows_per_day" in runbook
    assert "database_facts_before" in runbook
    assert "DB/schema/storage facts" in runbook
    assert "database size" in runbook
    assert "target_partition_months_required" in runbook
    assert "target_partition_months_existing" in runbook
    assert "target_partition_months_missing" in runbook
    assert "target_dates_without_partition_coverage" in runbook
    assert "audit_event_YYYY_MM" in runbook
    assert "哪些目标日期没有被月分区覆盖" in runbook
    assert "执行前容量 / 窗口评估" in runbook
    assert "--i-understand-this-is-not-evidence" in runbook
    assert "不写 `docs/retros/wave-1-h2-runtime-evidence.json`" in runbook
    assert "不写 `audit_chain_seal`" in runbook
    assert "dry-run 可使用 `dev-h2.wms.internal`" in runbook
    assert "`--execute` 会拒绝该本机 alias" in runbook
    assert "summary 只能写入 `artifacts/dev/wave1/h2/`" in runbook


def test_wave1_h2_baseline_container_dry_run_entry_is_documented_and_guarded():
    """容器网络 dry-run 必须固化入口，避免宿主机误连 compose 服务名。"""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-1-h2-baseline-dry-run-container" in justfile
    assert "--env-file deploy/env/dev-h2.env" in justfile
    assert "--workdir /tmp" in justfile
    assert "mkdir -p artifacts/dev/wave1/h2" in justfile
    assert (
        "cargo build --manifest-path backend/Cargo.toml -p wms-api --release "
        "--bin wms-audit-baseline-load"
        in justfile
    )
    assert "exec /tmp/wms-audit-baseline-load" in justfile
    assert "wms-api-dev-h2:${WMS_VERSION:-latest}" in justfile
    container_recipe = justfile.split(
        "wave-1-h2-baseline-dry-run-container", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 基线材料加载", maxsplit=1)[0]
    assert "sudo -n docker run" in container_recipe
    assert "--pull=never" in container_recipe
    assert "--network wms-dev-h2_default" in container_recipe
    assert "--user \"$(id -u):$(id -g)\"" in container_recipe
    assert "--entrypoint /bin/sh" in container_recipe
    assert "WMS_DB_URL=" in container_recipe
    assert (
        "$PWD/backend/target/release/wms-audit-baseline-load:"
        "/tmp/wms-audit-baseline-load:ro"
        in container_recipe
    )
    assert "/tmp/wms-audit-baseline-load" in container_recipe
    assert "/app/wms-audit-baseline-load" not in container_recipe
    assert "$@" not in container_recipe
    assert "docker compose" not in container_recipe
    assert (
        '$PWD/artifacts/dev/wave1/h2:/tmp/artifacts/dev/wave1/h2'
        in container_recipe
    )
    assert 'case " {{args}} " in *" --execute "*)' in container_recipe
    assert "dry-run-container refuses --execute" in container_recipe
    assert "--database-url" not in container_recipe

    assert "wave-1-h2-baseline-load-container" in justfile
    load_recipe = justfile.split(
        "wave-1-h2-baseline-load-container", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 基线材料加载：需要显式", maxsplit=1)[0]
    assert "sudo -n docker run" in load_recipe
    assert "--pull=never" in load_recipe
    assert "--network wms-dev-h2_default" in load_recipe
    assert "--user \"$(id -u):$(id -g)\"" in load_recipe
    assert "--entrypoint /bin/sh" in load_recipe
    assert "WMS_DB_URL=" in load_recipe
    assert (
        "$PWD/backend/target/release/wms-audit-baseline-load:"
        "/tmp/wms-audit-baseline-load:ro"
        in load_recipe
    )
    assert "/tmp/wms-audit-baseline-load" in load_recipe
    assert "/app/wms-audit-baseline-load" not in load_recipe
    assert "$@" not in load_recipe
    assert "docker compose" not in load_recipe
    assert "WMS_DEV_DB_HOST_ALLOWLIST=postgres-dev-h2" in load_recipe
    assert "--workdir /tmp" in load_recipe
    assert (
        '$PWD/artifacts/dev/wave1/h2:/tmp/artifacts/dev/wave1/h2' in load_recipe
    )
    assert "-e WMS_DEV_DB_HOST_ALLOWLIST=postgres-dev-h2" in load_recipe
    assert "--database-url" not in load_recipe

    assert "容器网络 dry-run" in runbook
    assert "postgres-dev-h2:5432" in runbook
    assert "复用已构建的 `wms-api-dev-h2` 镜像" in runbook
    assert "不触发 compose build" in runbook
    assert "挂载宿主机 `artifacts/dev/wave1/h2`" in runbook
    assert "summary 会保留在宿主机 `artifacts/dev/wave1/h2/`" in runbook
    assert "just wave-1-h2-baseline-dry-run-container" in runbook
    assert "不加 `--execute`" in runbook
    assert "不会插入 baseline 行" in runbook
    assert "不会写入 `docs/retros/wave-1-h2-runtime-evidence.json`" in runbook
    assert "summary 仍只能写入 `artifacts/dev/wave1/h2/`" in runbook
    assert "wave-1-h2-baseline-load-container" in runbook
    assert "WMS_DEV_DB_HOST_ALLOWLIST=postgres-dev-h2" in runbook
    assert "--execute" in runbook
    assert "--i-understand-this-is-not-evidence" in runbook


def test_wave1_h2_baseline_status_container_is_read_only_and_documented():
    """大加载前/中/后必须有稳定只读监控入口。"""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-1-h2-baseline-status-container:" in justfile
    status_recipe = justfile.split(
        "wave-1-h2-baseline-status-container:", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 60M 基线材料规划", maxsplit=1)[0]
    assert "sudo -n docker exec wms-dev-h2-postgres-dev-h2-1" in status_recipe
    assert "select count(*) as audit_event_rows from audit_event" in status_recipe
    assert "select count(*) as audit_chain_seal_rows from audit_chain_seal" in (
        status_recipe
    )
    assert "occurred_at::date as day" in status_recipe
    assert "pg_database_size(current_database())" in status_recipe
    assert "pg_inherits" in status_recipe
    assert "audit_event_total_size" in status_recipe
    assert "sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 df -h" in status_recipe
    assert "ps -eo" in status_recipe
    assert "[w]ms-audit-baseline-load" in status_recipe
    assert "--execute" not in status_recipe
    assert "INSERT INTO" not in status_recipe
    assert "docs/retros" not in status_recipe

    assert "just wave-1-h2-baseline-status-container" in runbook
    assert "只读状态检查" in runbook
    assert "不会写入 baseline 行" in runbook
    assert "不会写入 runtime evidence" in runbook


def test_wave1_h2_baseline_60m_container_shortcuts_are_guarded_and_documented():
    """固定 60M 快捷入口必须避免人工拼错参数。"""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-1-h2-baseline-plan-60m-container:" in justfile
    plan_recipe = justfile.split(
        "wave-1-h2-baseline-plan-60m-container:", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 60M 基线材料加载", maxsplit=1)[0]
    assert "wave-1-h2-baseline-dry-run-container" in plan_recipe
    assert "--target-total-rows 60000000" in plan_recipe
    assert "--days 7" in plan_recipe
    assert "--batch-size 4000" in plan_recipe
    assert "PLAN60M-" in plan_recipe
    assert "--execute" not in plan_recipe
    assert "docs/retros" not in plan_recipe

    assert "wave-1-h2-baseline-load-60m-container:" in justfile
    load_recipe = justfile.split(
        "wave-1-h2-baseline-load-60m-container:", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 基线材料加载", maxsplit=1)[0]
    assert "wave-1-h2-baseline-load-container" in load_recipe
    assert "--target-total-rows 60000000" in load_recipe
    assert "--days 7" in load_recipe
    assert "--batch-size 4000" in load_recipe
    assert "BASELINE60M-" in load_recipe
    assert "--execute" in load_recipe
    assert "--i-understand-this-is-not-evidence" in load_recipe
    assert "docs/retros" not in load_recipe

    assert "just wave-1-h2-baseline-plan-60m-container" in runbook
    assert "just wave-1-h2-baseline-load-60m-container" in runbook
    assert "固定 60M 参数" in runbook
    assert "真实写入 dev-h2 数据库" in runbook


def test_wave1_h2_baseline_preflight_60m_container_blocks_risky_state():
    """60M 大加载前必须有只读 preflight，避免封档/混入/并发加载风险。"""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-1-h2-baseline-preflight-60m-container:" in justfile
    recipe = justfile.split(
        "wave-1-h2-baseline-preflight-60m-container:", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 60M 基线材料规划", maxsplit=1)[0]
    assert "audit_chain_seal" in recipe
    assert "SEALED_COUNT=" in recipe
    assert "MIXED_COUNT=" in recipe
    assert "actor_name <> 'wave1-h2-baseline-loader'" in recipe
    assert "action <> 'baseline.synthetic_event.prepared'" in recipe
    assert "[w]ms-audit-baseline-load" in recipe
    assert "wave-1-h2-baseline-plan-60m-container" in recipe
    assert "-Atc" in recipe
    assert "do $$" not in recipe
    assert "--execute" not in recipe
    assert "INSERT INTO" not in recipe
    assert "docs/retros" not in recipe

    assert "just wave-1-h2-baseline-preflight-60m-container" in runbook
    assert "60M 大加载前置检查" in runbook
    assert "目标日期范围内不能已有封档" in runbook
    assert "不能混入非 baseline loader 行" in runbook


def test_wave1_h2_dev_runbook_documents_dry_run_readiness_status_report():
    """dev-h2 readiness dry-run 报告必须明确不能关闭 W6.A gate。"""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-1-h2-runtime-readiness-dry-run:" in justfile
    assert "--dry-run-alias-ok" in justfile
    assert "--json" in justfile
    assert "just wave-1-h2-runtime-readiness-dry-run" in runbook
    assert "writes_runtime_evidence=false" in runbook
    assert "closes_gate=false" in runbook
    assert "dry_run_only=true" in runbook
    assert "formal_evidence_allowed=false" in runbook
    assert "避免把密码写进 shell 历史和进程参数" in runbook
    assert "加载窗口必须完全早于今天" in runbook
    assert "目标日期范围内不能已有 `audit_chain_seal`" in runbook
    assert "一旦日期已封档，禁止再追加 baseline 事件" in runbook
    assert "已有非 baseline loader 写入的真实 `audit_event`" in runbook
    assert "日维度 advisory lock" in runbook


def test_wave1_h2_baseline_loader_and_seal_share_day_lock():
    """baseline loader 与 seal 必须共享日维度锁，避免封档并发追加。"""
    loader = Path("backend/crates/api/src/bin/wms_audit_baseline_load.rs").read_text(
        encoding="utf-8",
    )
    audit = Path("backend/crates/api/src/audit.rs").read_text(encoding="utf-8")

    assert "audit_event:{day}" in loader
    assert "audit_event:{}" in audit
    assert "pg_advisory_lock(hashtext($1))" in loader
    assert "pg_advisory_xact_lock(hashtext($1))" in audit
    assert "ensure_day_unsealed_in_tx" in loader
    assert "ensure_day_contains_only_baseline_events" in loader


def test_wave1_h2_seal_cron_container_entries_are_guarded_and_documented():
    """60M 基线后必须有 dev-h2 容器网络 seal cron 入口。"""
    runbook = Path("docs/runbooks/wave-1-h2-dev-environment.md").read_text(
        encoding="utf-8",
    )
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-1-h2-seal-status-container:" in justfile
    status_recipe = justfile.split(
        "wave-1-h2-seal-status-container:", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 7 天封档 dry-run", maxsplit=1)[0]
    assert "select seal_date, last_id, sealed_at from audit_chain_seal" in status_recipe
    assert "sudo -n docker exec wms-dev-h2-postgres-dev-h2-1" in status_recipe
    assert "INSERT INTO" not in status_recipe
    assert "docs/retros" not in status_recipe

    assert "wave-1-h2-seal-dry-run-7d-container:" in justfile
    dry_run_recipe = justfile.split(
        "wave-1-h2-seal-dry-run-7d-container:", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 7 天封档执行", maxsplit=1)[0]
    assert "wave-1-h2-seal-status-container" in dry_run_recipe
    assert "writes_audit_chain_seal=false" in dry_run_recipe
    assert "writes_runtime_evidence=false" in dry_run_recipe
    assert "cargo build --manifest-path backend/Cargo.toml -p wms-api --release --bin audit-maintenance" in dry_run_recipe
    assert "--execute" not in dry_run_recipe
    assert "audit_maintenance.sh" not in dry_run_recipe

    assert "wave-1-h2-seal-preflight-7d-container:" in justfile
    preflight_recipe = justfile.split(
        "wave-1-h2-seal-preflight-7d-container:", maxsplit=1
    )[1].split("\n# Wave 1 H2 dev 7 天封档执行", maxsplit=1)[0]
    assert "TOTAL_ROWS=" in preflight_recipe
    assert "60000000" in preflight_recipe
    assert "DAYS_WITH_EVENTS=" in preflight_recipe
    assert "EVENT_DAYS_WITHOUT_SEAL=" in preflight_recipe
    assert "SEALED_COUNT=" in preflight_recipe
    assert "audit_event" in preflight_recipe
    assert "audit_chain_seal" in preflight_recipe
    assert "[w]ms-audit-baseline-load" in preflight_recipe
    assert "writes_runtime_evidence=false" in preflight_recipe
    assert "INSERT INTO" not in preflight_recipe
    assert "docs/retros" not in preflight_recipe

    assert "wave-1-h2-seal-run-7d-container:" in justfile
    run_recipe = justfile.split(
        "wave-1-h2-seal-run-7d-container:", maxsplit=1
    )[1].split("\n# Wave 1 runtime evidence", maxsplit=1)[0]
    assert "just wave-1-h2-seal-preflight-7d-container" in run_recipe
    assert "cargo build --manifest-path backend/Cargo.toml -p wms-api --release --bin audit-maintenance" in run_recipe
    assert "--network wms-dev-h2_default" in run_recipe
    assert "--env-file deploy/env/dev-h2.env" in run_recipe
    assert "$PWD/backend/target/release/audit-maintenance:/tmp/audit-maintenance:ro" in run_recipe
    assert "AUDIT_MAINTENANCE_BIN=/tmp/audit-maintenance" in run_recipe
    assert "deploy/scripts/audit_maintenance.sh" in run_recipe
    assert "postgres-dev-h2:5432" in run_recipe
    assert "for offset in 7 6 5 4 3 2 1" in run_recipe
    assert "docs/retros" not in run_recipe

    assert "just wave-1-h2-seal-preflight-7d-container" in runbook
    assert "just wave-1-h2-seal-dry-run-7d-container" in runbook
    assert "just wave-1-h2-seal-run-7d-container" in runbook
    assert "just wave-1-h2-seal-status-container" in runbook
    assert "7 天 seal cron 材料" in runbook
    assert "先通过 60,000,000 行和 7 个目标日覆盖检查" in runbook
    assert "不会写入 runtime evidence" in runbook
    assert "不能关闭 W6.A gate" in runbook
