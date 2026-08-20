# H8 ERP 消息 10M 性能验收

本运行手册用于关闭 US-H8-003 AC12。只能在独立的生产等价 dev/staging 数据库执行；不得在生产库、本机 localhost 或共享业务库灌入基线数据。
仓库现有 `deploy/docker-compose.staging.yml` 属于本机 staging 预演，按
[Wave 6 staging 预演记录](wave-6-staging-deploy-dry-run.md)不能作为本条正式证据。

## 本机 Docker 预检

可以复用现有 Compose 文件建立隔离的 PostgreSQL 18 / Redis / API 环境，用于检查 migration、
分区表、索引和应用健康状态。必须使用独立 Compose project 和未占用端口，不得复用共享
staging 的卷：

```bash
WMS_VERSION="$(git rev-parse --short HEAD)" \
WMS_STAGING_API_PORT=18084 \
docker compose \
  -p wms-h8-perf \
  --env-file deploy/env/staging.env \
  -f deploy/docker-compose.staging.yml \
  up -d --build

curl -fsS http://127.0.0.1:18084/healthz
docker compose \
  -p wms-h8-perf \
  --env-file deploy/env/staging.env \
  -f deploy/docker-compose.staging.yml \
  exec postgres-staging \
  psql -U wms_staging -d wms_staging -c \
  "SELECT relname, relkind FROM pg_class WHERE relname IN ('h8_erp_messages', 'h8_erp_message_attempts');"
```

若宿主机的 Docker 代理无法拉取构建镜像，可先只启动 `postgres-staging` 和
`redis-staging`，在宿主机编译当前 commit 的 `wms-db-migrate` / `wms-api`，再将二进制文件
只读挂载到现有 WMS 运行时镜像。该方式只是本机连通性预检，不得将旧镜像内的应用
当作当前版本，也不得写入正式验收证据。

本机 Docker 不得执行下文的 10M/10M 数据灌入。AC12 仍必须使用独立、生产等价的
dev/staging 主机、真实网络入口和真实 JWT；运维确认容量、入口和证据归档后，
部署在该主机上的独立 Docker Compose 环境可作为正式验收环境。

## 验收基线

- PostgreSQL 18，记录 CPU、内存、磁盘类型、数据库参数和应用实例规格。
- 单货主单自然月 10,000,000 条消息，每条至少 1 条已完成尝试。
- 关闭 dev-mock，使用真实 API、真实 JWT 和真实网络入口。
- 16 并发；预热 5 分钟，分别连续测量列表和统计 15 分钟。
- 列表使用 30 天窗口和 `limit=200`，P95 ≤ 500ms；统计使用 30 天窗口，P95 ≤ 1s；HTTP 错误为 0。
- 标准 `wrk --latency` 输出 P99；P99 通过同一阈值可作为更严格的 P95 证明。

## 准备独立基线

先对独立数据库应用当前 migration，再设置专用货主和月份：

```bash
export DATABASE_URL='postgres://<user>:<password>@<dev-or-staging-host>:5432/<dedicated-h8-perf-db>'
export H8_PERF_OWNER_ID='00000000-0000-0000-0000-000000008812'
export H8_PERF_MONTH="$(date -u +%Y-%m-01)"
```

确认连接目标不是生产或 localhost 后执行。下面的删除只允许作用于上述专用货主：

```bash
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -v owner_id="$H8_PERF_OWNER_ID" -v perf_month="$H8_PERF_MONTH" <<'SQL'
SELECT current_database(), version();
SELECT h8_erp_messages_ensure_month_partition(:'perf_month'::date);

INSERT INTO auth_owners (id, owner_code, owner_name)
VALUES (:'owner_id'::uuid, 'H8_PERF', 'H8 性能验收专用货主')
ON CONFLICT (id) DO UPDATE SET owner_name = EXCLUDED.owner_name;

DELETE FROM h8_erp_message_attempts WHERE owner_id = :'owner_id'::uuid;
DELETE FROM h8_erp_messages WHERE owner_id = :'owner_id'::uuid;

WITH bounds AS (
  SELECT
    date_trunc('month', CURRENT_TIMESTAMP AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' AS start_ts,
    CURRENT_TIMESTAMP AS end_ts
)
INSERT INTO h8_erp_messages (
  id, owner_id, connector_code, direction, message_type, schema_version, channel,
  external_ref, idempotency_key, correlation_id, sync_status, retry_count,
  payload_digest, created_at, updated_at, completed_at
)
SELECT
  gen_random_uuid(), :'owner_id'::uuid, 'H8-PERF', 'inbound', 'asn', '1',
  'interface_table', 'PERF-' || n, 'perf-idem-' || n, 'perf-corr-' || n,
  'succeeded', (n % 3)::int, md5(n::text),
  bounds.start_ts
    + ((n - 1) % GREATEST(EXTRACT(EPOCH FROM bounds.end_ts - bounds.start_ts)::bigint, 1))
      * interval '1 second',
  bounds.start_ts
    + ((n - 1) % GREATEST(EXTRACT(EPOCH FROM bounds.end_ts - bounds.start_ts)::bigint, 1))
      * interval '1 second',
  bounds.start_ts
    + ((n - 1) % GREATEST(EXTRACT(EPOCH FROM bounds.end_ts - bounds.start_ts)::bigint, 1))
      * interval '1 second'
FROM generate_series(1, 10000000) AS n
CROSS JOIN bounds;

INSERT INTO h8_erp_message_attempts (
  id, message_id, owner_id, attempt_no, channel, started_at, finished_at, result, actor
)
SELECT
  gen_random_uuid(), id, owner_id, 1, channel, created_at,
  created_at + ((1 + (hashtextextended(id::text, 0) & 2047) % 2000)::text
    || ' milliseconds')::interval,
  'succeeded', 'h8-perf-loader'
FROM h8_erp_messages
WHERE owner_id = :'owner_id'::uuid;

ANALYZE h8_erp_messages;
ANALYZE h8_erp_message_attempts;
ANALYZE h8_erp_message_stats_daily;

SELECT count(*) AS messages FROM h8_erp_messages WHERE owner_id = :'owner_id'::uuid;
SELECT count(*) AS attempts FROM h8_erp_message_attempts WHERE owner_id = :'owner_id'::uuid;
SELECT tableoid::regclass, count(*) FROM h8_erp_messages
WHERE owner_id = :'owner_id'::uuid GROUP BY tableoid ORDER BY tableoid::regclass::text;

EXPLAIN (ANALYZE, BUFFERS)
SELECT id, created_at
FROM h8_erp_messages
WHERE owner_id = :'owner_id'::uuid
  AND created_at >= CURRENT_TIMESTAMP - interval '30 days'
  AND created_at < CURRENT_TIMESTAMP
ORDER BY created_at DESC, id DESC
LIMIT 201;
SQL
```

## API 压测

为专用货主准备带 `h8.erp_connector.write` 的系统管理员真实 JWT，然后设置入口和证据目录：

```bash
export H8_API_BASE='https://<real-dev-or-staging-api>'
export H8_PERF_TOKEN='<short-lived-jwt>'
export H8_PERF_FROM="$(date -u -d '30 days ago' +%Y-%m-%dT%H:%M:%SZ | sed 's/:/%3A/g')"
export H8_PERF_TO="$(date -u +%Y-%m-%dT%H:%M:%SZ | sed 's/:/%3A/g')"
mkdir -p artifacts/dev/h8/performance
```

列表先预热再测量：

```bash
wrk -t4 -c16 -d300s --latency \
  "$H8_API_BASE/api/v1/integration/erp-messages?limit=200&created_from=$H8_PERF_FROM&created_to=$H8_PERF_TO" \
  -H "Authorization: Bearer $H8_PERF_TOKEN" >/dev/null
wrk -t4 -c16 -d900s --latency \
  "$H8_API_BASE/api/v1/integration/erp-messages?limit=200&created_from=$H8_PERF_FROM&created_to=$H8_PERF_TO" \
  -H "Authorization: Bearer $H8_PERF_TOKEN" \
  | tee artifacts/dev/h8/performance/list-wrk.log
```

统计先预热再测量：

```bash
wrk -t4 -c16 -d300s --latency \
  "$H8_API_BASE/api/v1/integration/erp-messages/stats?connector_code=H8-PERF&channel=interface_table&message_type=asn" \
  -H "Authorization: Bearer $H8_PERF_TOKEN" >/dev/null
wrk -t4 -c16 -d900s --latency \
  "$H8_API_BASE/api/v1/integration/erp-messages/stats?connector_code=H8-PERF&channel=interface_table&message_type=asn" \
  -H "Authorization: Bearer $H8_PERF_TOKEN" \
  | tee artifacts/dev/h8/performance/stats-wrk.log
```

## 证据与退出条件

证据包必须同时保存：环境规格、PostgreSQL 版本和参数、10M/10M 行数、分区分布、列表查询 `EXPLAIN (ANALYZE, BUFFERS)`、两份原始 `wrk` 日志及外部归档引用。任一日志的 P99 超过对应阈值、出现 socket/HTTP 错误，或证据来自 localhost/Mock/缩量数据时，AC12 保持 `PARTIAL`。
