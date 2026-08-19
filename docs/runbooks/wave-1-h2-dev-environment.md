# Wave 1 H2 Dev Environment Runbook

> 用途：准备 W6.A 所需的 dev PostgreSQL 边界，用于后续采集 Wave 1 H2 runtime evidence。本 runbook 只准备环境和输入，不会写入 runtime evidence，不能关闭 W6.A gate。

## 边界

- 目标 gate：W6.A。
- 环境口径：dev PostgreSQL。
- 部署形态：ADR-0016 的 docker-compose 单机路径，仅包含 PostgreSQL 和 one-shot migrator。
- 不能使用 staging，不能使用 local、localhost、127.0.0.1、0.0.0.0、prod、production、mock、fake、stub 或 example 证据关闭 W6.A。
- 正式 evidence 只能由 `just wave-1-h2-runtime-evidence` 在真实 dev 输入齐全后写入 `docs/retros/wave-1-h2-runtime-evidence.json`。

## 文件

- `deploy/docker-compose.dev-h2.yml`
- `deploy/env/dev-h2.env.example`
- `deploy/secrets.example.md`
- `backend/Dockerfile.wms-api`
- `backend/migrations/202606020001_audit_event.sql`

## 启动 dev PostgreSQL

在 dev H2 主机上准备 env 和 secret 文件：

```bash
mkdir -p deploy/secrets deploy/env
test -f deploy/env/dev-h2.env || cp deploy/env/dev-h2.env.example deploy/env/dev-h2.env
```

编辑 `deploy/env/dev-h2.env`，填入真实 dev H2 值；不要提交该文件。

```bash
set -a
. deploy/env/dev-h2.env
set +a

: "${WMS_DEV_H2_DB_PASSWORD:?set WMS_DEV_H2_DB_PASSWORD in deploy/env/dev-h2.env}"

printf '%s' "$WMS_DEV_H2_DB_PASSWORD" > deploy/secrets/wms_dev_h2_db_password.txt
test "$(cat deploy/secrets/wms_dev_h2_db_password.txt)" = "$WMS_DEV_H2_DB_PASSWORD" || \
  { echo "deploy/secrets/wms_dev_h2_db_password.txt does not match WMS_DEV_H2_DB_PASSWORD"; exit 1; }
```

启动 PostgreSQL 并执行 migration：

```bash
docker compose --env-file deploy/env/dev-h2.env -f deploy/docker-compose.dev-h2.yml up -d --build
```

当前 compose 路径使用 `wms-db-migrate-dev-h2` one-shot migrator，在 `postgres-dev-h2` healthy 后执行数据库迁移。迁移失败时不要采集 W6.A evidence，先修复 schema 或迁移脚本。

## W6.A 输入导出

`WAVE1_H2_DATABASE_URL` 的主机名或服务名必须包含 `dev` 标记，不能使用 staging 或 local 边界。现场可通过 dev DNS、容器网络别名、堡垒机代理或 CI secret 注入真实 dev URL。

```bash
export WAVE1_H2_RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
export WAVE1_H2_DATABASE_URL="postgres://wms_dev_h2:$WMS_DEV_H2_DB_PASSWORD@postgres-dev-h2:5432/wms_dev_h2"
export WAVE1_H2_WRK_OUTPUT="artifacts/dev/wave1/h2/wrk-$WAVE1_H2_RUN_ID.log"
export WAVE1_H2_BENCHMARK_LOG_REF="s3://wms-dev-evidence/wave1/h2/wrk-$WAVE1_H2_RUN_ID.log"
export WAVE1_H2_CRON_LOG_REF="s3://wms-dev-evidence/wave1/h2/audit-seal-cron-$WAVE1_H2_RUN_ID.log"
export WMS_DEV_DB_HOST_ALLOWLIST="pg-dev.wms.internal"
```

如果从宿主机连接 compose 暴露端口，不能把 `localhost` 写入 evidence 边界；请通过 dev DNS、SSH 隧道域名或 CI 注入一个包含 `dev` 标记的数据库 URL，再运行 W6.A readiness。

本机 dry-run 可临时把 `dev-h2.wms.internal` 指向宿主机回环地址，用于验证脚本链路和 migration 是否可用：

```bash
grep -q 'dev-h2.wms.internal' /etc/hosts || \
  echo '127.0.0.1 dev-h2.wms.internal' | sudo tee -a /etc/hosts

export WAVE1_H2_DATABASE_URL="postgres://wms_dev_h2:$WMS_DEV_H2_DB_PASSWORD@dev-h2.wms.internal:${WMS_DEV_H2_DB_PORT:-15432}/wms_dev_h2"
```

上述 alias 只用于本机 dry-run，不能作为正式 W6.A evidence；正式 evidence 仍必须来自真实 dev DNS、CI、日志归档和对象存储引用。

先运行只读检查：

```bash
just wave-1-runtime-prereq-h2
just wave-1-h2-runtime-readiness
```

这两条命令只验证环境变量、工具、dev 边界、`audit_event` 基线和 `audit_chain_seal` 最近 7 个自然日封档；不会写入 runtime evidence，不能关闭 W6.A gate。

本机 `dev-h2.wms.internal` alias 只用于 dry-run 状态报告，不能关闭 W6.A gate。需要查看当前 baseline / seal 计数时运行：

```bash
just wave-1-h2-runtime-readiness-dry-run
```

该命令输出 JSON，并明确 `writes_runtime_evidence=false`、`closes_gate=false`、`dry_run_only=true`、`formal_evidence_allowed=false`；即使 `audit_event` / `audit_chain_seal` 计数达标，也不能替代真实 dev DNS、1 小时 wrk 原始日志和外部归档引用。

## 准备 audit_event 基线材料

`audit_event` 基线可以使用 `wms-audit-baseline-load` 在真实 dev PostgreSQL 中补齐。该工具只准备 dev 基线材料，不写 `docs/retros/wave-1-h2-runtime-evidence.json`，不写 `audit_chain_seal`，不能关闭 W6.A gate。

先 dry-run，确认当前行数、计划补数、分区范围、计划批次数、每日行数分布、DB/schema/storage facts 和 summary 输出路径：

```bash
export DATABASE_URL="$DEV_WAVE1_H2_DATABASE_URL"

just wave-1-h2-baseline-dry-run \
  --target-total-rows 60000000 \
  --start-date "$(date -u -d '7 days ago' +%F)" \
  --days 7 \
  --batch-size 4000 \
  --run-id "$WAVE1_H2_RUN_ID" \
  --summary-output "artifacts/dev/wave1/h2/baseline-loader-$WAVE1_H2_RUN_ID.json"
```

如果宿主机无法解析 compose 服务名，优先使用容器网络 dry-run。该入口会先构建宿主机 release 版 `wms-audit-baseline-load` 并挂载到容器 `/tmp/wms-audit-baseline-load`，复用已构建的 `wms-api-dev-h2` 镜像和 `wms-dev-h2_default` 网络，不触发 compose build；它在容器内生成指向 `postgres-dev-h2:5432` 的 `WMS_DB_URL`，不把数据库密码写进命令行参数。它不加 `--execute`，不会插入 baseline 行，不会写入 `docs/retros/wave-1-h2-runtime-evidence.json`。入口会挂载宿主机 `artifacts/dev/wave1/h2` 到容器 `/tmp/artifacts/dev/wave1/h2`，summary 会保留在宿主机 `artifacts/dev/wave1/h2/`；summary 仍只能写入 `artifacts/dev/wave1/h2/`。

固定 60M 参数时，优先使用快捷入口，避免人工拼错 `target-total-rows`、日期窗口、batch size 和 summary 路径：

```bash
just wave-1-h2-baseline-plan-60m-container
```

60M 大加载前置检查会确认目标日期范围内不能已有封档、不能混入非 baseline loader 行、不能已有 loader 进程在跑，然后执行固定 60M dry-run：

```bash
just wave-1-h2-baseline-preflight-60m-container
```

如需手动传参，也可以使用底层入口：

```bash
just wave-1-h2-baseline-dry-run-container \
  --target-total-rows 60000000 \
  --start-date "$(date -u -d '7 days ago' +%F)" \
  --days 7 \
  --batch-size 4000 \
  --run-id "$WAVE1_H2_RUN_ID" \
  --summary-output "artifacts/dev/wave1/h2/baseline-loader-$WAVE1_H2_RUN_ID.json"
```

`wave-1-h2-baseline-dry-run-container` 会拒绝 `--execute`。确认容器 dry-run summary 后，如需在 compose dev-h2 数据库内实际准备 baseline 材料，使用受保护的 container load 入口；该入口会在容器内设置 `WMS_DEV_DB_HOST_ALLOWLIST=postgres-dev-h2`，仍必须显式传入 `--execute` 和 `--i-understand-this-is-not-evidence`，且仍不会写入 runtime evidence。固定 60M 参数的加载入口会真实写入 dev-h2 数据库：

```bash
just wave-1-h2-baseline-load-60m-container
```

如需手动传参，也可以使用底层入口：

```bash
just wave-1-h2-baseline-load-container \
  --target-total-rows 60000000 \
  --start-date "$(date -u -d '7 days ago' +%F)" \
  --days 7 \
  --batch-size 4000 \
  --run-id "$WAVE1_H2_RUN_ID" \
  --summary-output "artifacts/dev/wave1/h2/baseline-loader-$WAVE1_H2_RUN_ID.json" \
  --execute \
  --i-understand-this-is-not-evidence
```

加载前、加载中、加载后都可以用同一条只读状态检查命令观察当前行数、按日分布、DB size、`audit_event` size、PostgreSQL 数据目录磁盘和 loader 进程。该命令不会写入 baseline 行，不会写入 runtime evidence：

```bash
just wave-1-h2-baseline-status-container
```

确认 dry-run summary 中的 `planned_batches`、`rows_per_day`、`database_facts_before`、dev DB 磁盘、IO 和运维窗口后再执行实际加载：

```bash
export DATABASE_URL="$DEV_WAVE1_H2_DATABASE_URL"

just wave-1-h2-baseline-load \
  --target-total-rows 60000000 \
  --start-date "$(date -u -d '7 days ago' +%F)" \
  --days 7 \
  --batch-size 4000 \
  --run-id "$WAVE1_H2_RUN_ID" \
  --summary-output "artifacts/dev/wave1/h2/baseline-loader-$WAVE1_H2_RUN_ID.json" \
  --execute \
  --i-understand-this-is-not-evidence
```

防误用规则：

- `target-total-rows` 表示目标总行数，工具只补 `target - current` 的差额。
- dry-run summary 会输出 `planned_batches`、`rows_per_day` 和 `database_facts_before`，用于执行前容量 / 窗口评估；该 summary 仍不是 runtime evidence，不能关闭 W6.A。
- `database_facts_before` 只记录执行前事实，例如 database name、schema、PostgreSQL 版本、`audit_event` 分区数量、当前 database size、`target_partition_months_required`、`target_partition_months_existing`、`target_partition_months_missing` 和 `target_dates_without_partition_coverage`；这些字段按现有 `audit_event_YYYY_MM` 月分区模型判断哪些目标日期没有被月分区覆盖，不要把它当成 60M 插入后的容量证明。
- 优先通过 `DATABASE_URL` / `WMS_DB_URL` 传入数据库连接，避免把密码写进 shell 历史和进程参数。
- dry-run 可使用 `dev-h2.wms.internal` 验证链路；`--execute` 会拒绝该本机 alias。
- `--execute` 必须使用真实 dev PostgreSQL DNS，例如 `pg-dev.wms.internal`；不能使用本机 alias、raw IP、localhost、staging 或 production 边界。
- `WMS_DEV_DB_HOST_ALLOWLIST` 是 baseline `--execute` 的 dev DB DNS allowlist，逗号分隔。默认只允许 `pg-dev.wms.internal`；如果现场 DNS 不同，先把真实 dev DNS 加到该变量，不能把 `dev-h2.wms.internal` 加进去。
- baseline `--execute` 会解析 DB host，解析到 loopback 的 DNS 会被拒绝；不能把正式 dev DNS 临时写到 `/etc/hosts` 指向 `127.0.0.1` 来替代真实 dev DB。
- summary 只能写入 `artifacts/dev/wave1/h2/`，禁止写入 `docs/retros/`。
- summary 默认只允许新建文件，避免重跑覆盖材料；同一个 `run-id` 要重跑时，优先换新 `run-id`，确需覆盖才显式加 `--force-summary`。
- 加载窗口必须完全早于今天，且目标日期范围内不能已有 `audit_chain_seal`；一旦日期已封档，禁止再追加 baseline 事件。
- 加载窗口内如果已有非 baseline loader 写入的真实 `audit_event`，工具会拒绝，避免把 synthetic baseline 接到真实业务审计链后面。
- baseline loader 与 `seal_audit_chain` 使用相同的日维度 advisory lock，避免 seal cron 与 baseline 加载并发破坏封档语义。
- 7 天封档必须由 `audit_maintenance.sh` / H-SCH cron 真实执行产生，不能由 baseline loader 手工写 seal。

## 准备 7 天 seal cron 材料

60M `audit_event` 基线加载完成后，先查看最近 7 个自然日的封档状态：

```bash
just wave-1-h2-seal-status-container
```

再执行 7 天 seal cron dry-run。该命令只构建 `audit-maintenance`，展示目标日期并读取当前 `audit_chain_seal` 状态；它输出 `writes_audit_chain_seal=false`、`writes_runtime_evidence=false`，不会写入 runtime evidence，不能关闭 W6.A gate：

```bash
just wave-1-h2-seal-dry-run-7d-container
```

执行真实封档前必须先通过 60,000,000 行和 7 个目标日覆盖检查。该 preflight 只读检查 `audit_event` 总行数、最近 7 个自然日是否都有事件、目标日期是否尚未封档、是否有 baseline loader 并发运行；不会写入 runtime evidence，不能关闭 W6.A gate：

```bash
just wave-1-h2-seal-preflight-7d-container
```

确认 60M 行数、目标日期、无并发 baseline loader、无已封档冲突后，才执行真实 dev-h2 封档入口。该入口会先调用 `wave-1-h2-seal-preflight-7d-container`，再通过容器网络连接 `postgres-dev-h2:5432`，逐日调用 `deploy/scripts/audit_maintenance.sh` 和 `audit-maintenance`，真实写入 `audit_chain_seal`，但仍不会写入 `docs/retros/wave-1-h2-runtime-evidence.json`：

```bash
just wave-1-h2-seal-run-7d-container
```

执行后再次检查封档状态：

```bash
just wave-1-h2-seal-status-container
just wave-1-h2-runtime-readiness
```

## 基线要求

W6.A readiness 通过前，dev PostgreSQL 必须满足：

- `audit_event` 基线行数不少于 60,000,000。
- 最近 7 个自然日 `audit_chain_seal` 均有封档记录。
- 封档任务失败数为 0。
- `wrk` 原始日志来自 1 小时 dev API 压测。

压测完成后才运行正式采集：

```bash
just wave-1-h2-runtime-evidence
```

该命令会写入 `docs/retros/wave-1-h2-runtime-evidence.json`。没有 60,000,000 行基线、7 个自然日封档、1 小时 wrk 原始日志和 dev 边界引用时，不得运行正式采集命令。
