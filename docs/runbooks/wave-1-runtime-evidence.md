# Wave 1 Pre-release Runtime Evidence Runbook

> 用途：在稳定 dev / staging 环境产出 Wave 1 预发布 gate 所需 runtime evidence。当前无稳定 dev/staging 时，runtime evidence 不阻塞 `just wave-1-complete-check` 的开发完成判定；预发布前必须补齐。禁止用 localhost、stub、短测、`example.*` 模板域名或生产边界代替。

## 1. H2 PostgreSQL 压测与封档证据

前置条件：

- `wrk`、`psql` 可用
- dev PostgreSQL 已跑最新 migration
- `audit_event` 基线行数不少于 60,000,000
- 最近 7 个自然日 `audit_chain_seal` 均有封档记录，且封档任务失败数为 0

先设置采集边界并执行前置检查。该检查不会写 evidence，只验证环境变量、工具、dev 边界和短测参数：

```bash
export WAVE1_H2_DATABASE_URL='postgres://USER:PASS@pg-dev.wms.internal:5432/wms_dev'
export WAVE1_H2_WRK_OUTPUT='/tmp/wave1-h2-wrk-dev.log'
export WAVE1_H2_BENCHMARK_LOG_REF='s3://wms-dev-evidence/wave1/h2/wrk-YYYYMMDD.log'
export WAVE1_H2_CRON_LOG_REF='s3://wms-dev-evidence/wave1/h2/audit-seal-cron-YYYYMMDD.log'
just wave-1-runtime-prereq-h2
just wave-1-h2-runtime-readiness
```

前置检查和 DB readiness 都通过后，在 dev API 边界执行 1 小时 wrk 压测，并保存原始日志：

```bash
wrk -t8 -c128 -d3600s --latency "https://wms-api.dev.wms.internal/api/v1/audit/events" \
  > "$WAVE1_H2_WRK_OUTPUT"
```

压测完成后运行采集器：

```bash
just wave-1-h2-runtime-evidence
```

采集器会写入 `docs/retros/wave-1-h2-runtime-evidence.json`，并校验：

- environment = `dev`
- baseline_rows >= 60,000,000
- observed_qps >= 1000
- duration_seconds >= 3600
- p99_ms < 200
- consecutive_success_days >= 7
- failure_count = 0
- DB / log 引用不能指向 localhost、127.0.0.1、0.0.0.0、prod、stub、mock、fake 或 example

## 2. W1.D 自动回滚证据

前置条件：

- dev 或 staging 有真实 smoke gate 或 Prometheus rollback 信号
- rollback 执行边界是 dev / staging，不能是 prod
- k8s 路径需要 `kubectl`、context、namespace
- docker-compose 路径需要上一版本 sha 与 dev/staging compose 文件
- 失败信号必须真实触发 rollback，且 rollback 退出码为 0

HTTP smoke 示例（k8s）：

```bash
export WAVE1_ROLLBACK_ENVIRONMENT='staging'
export WAVE1_K8S_CONTEXT='wms-staging'
export WAVE1_K8S_NAMESPACE='wms-staging'
export SMOKE_URL='https://wms-api.staging.wms.internal/healthz'
export WAVE1_ROLLBACK_LOG_REF='s3://wms-staging-evidence/wave1/rollback-YYYYMMDD.log'
export WAVE1_EXTERNAL_LOG_REF='s3://wms-staging-evidence/wave1/smoke-alert-YYYYMMDD.log'
just wave-1-runtime-prereq-rollback-k8s
just wave-1-rollback-runtime-readiness-k8s
just wave-1-rollback-runtime-evidence-k8s
```

Prometheus 示例（docker-compose）：

```bash
export WAVE1_ROLLBACK_ENVIRONMENT='dev'
export WAVE1_PREVIOUS_VERSION='previous-dev-sha'
export WAVE1_COMPOSE_FILE='/srv/wms-dev/docker-compose.yml'
export PROMETHEUS_URL='https://prometheus.dev.wms.internal'
export PROMETHEUS_QUERY='wms_wave1_rollback_signal{environment="dev"}'
export WAVE1_ROLLBACK_LOG_REF='s3://wms-dev-evidence/wave1/rollback-YYYYMMDD.log'
export WAVE1_EXTERNAL_LOG_REF='s3://wms-dev-evidence/wave1/prometheus-alert-YYYYMMDD.log'
just wave-1-runtime-prereq-rollback-compose
just wave-1-rollback-runtime-readiness-compose
just wave-1-rollback-runtime-evidence-compose
```

probe 会写入 `docs/retros/wave-1-runtime-evidence.json`，并校验：

- environment = `dev` 或 `staging`
- signal_type = `http` 或 `prometheus`
- rollback_triggered = true
- rollback_exit_code = 0
- signal / log 引用不能指向 localhost、127.0.0.1、0.0.0.0、prod、stub、mock、fake 或 example

## 3. 预发布验证

Wave 1 开发完成判定使用：

```bash
just wave-1-complete-check
```

两份 runtime evidence 写入后，预发布前执行：

```bash
just wave-1-runtime-evidence-validate
just gov-t1
python3 scripts/governance/task_check.py --tier T2 --strict
```

如需只验证 `.example.json` 模板格式，必须显式使用：

```bash
python3 scripts/governance/validate_wave1_runtime_evidence.py --kind all \
  --h2-file docs/retros/wave-1-h2-runtime-evidence.example.json \
  --w1d-file docs/retros/wave-1-runtime-evidence.example.json \
  --allow-example-refs
```

只有 `just wave-1-runtime-evidence-validate` 对真实 evidence 退出 0，才能视为预发布 runtime gate 通过。
