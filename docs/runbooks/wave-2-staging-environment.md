# Wave 2 Staging Environment Runbook

> 用途：搭建测试用途的 staging 环境，用于采集 W6.C Wave 2 config-center Feature Flag runtime evidence。该环境可以是测试用途，但 evidence 的 `environment` 不能写成 `test`，必须写 `staging`。

## 边界

- 部署形态：ADR-0016 的 docker-compose 单机路径。
- 环境口径：`staging`。
- 服务链路：真实 `wms-api` + PostgreSQL + Redis + `deploy/feature_flags.toml`。
- 已确认 2A：服务地址由外部 staging 反代、内网网关或既有负载均衡暴露；本 compose 只提供后端服务链路，不在 compose 内新增 Caddy/Nginx/Traefik。
- 禁止用 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`prod`、`production`、`mock`、`fake`、`stub`、`example` 证据关闭 W6.C。

## 文件

- `backend/Dockerfile.wms-api`
- `deploy/docker-compose.staging.yml`
- `deploy/env/staging.env.example`
- `deploy/secrets.example.md`

## 启动

在 staging 主机上准备 secrets：

```bash
mkdir -p deploy/secrets deploy/env
test -f deploy/env/staging.env || cp deploy/env/staging.env.example deploy/env/staging.env
```

编辑 `deploy/env/staging.env`，填入真实 staging 值；不要提交该文件。

加载 staging env，并用同一份 env 写入 PostgreSQL secret file：

```bash
set -a
. deploy/env/staging.env
set +a

: "${WMS_STAGING_DB_PASSWORD:?set WMS_STAGING_DB_PASSWORD in deploy/env/staging.env}"
: "${WMS_JWT_SECRET:?set WMS_JWT_SECRET in deploy/env/staging.env}"

printf '%s' "$WMS_STAGING_DB_PASSWORD" > deploy/secrets/wms_staging_db_password.txt
test "$(cat deploy/secrets/wms_staging_db_password.txt)" = "$WMS_STAGING_DB_PASSWORD" || \
  { echo "deploy/secrets/wms_staging_db_password.txt does not match WMS_STAGING_DB_PASSWORD"; exit 1; }
```

当前 compose 路径使用 `wms-db-migrate-staging` one-shot migrator 在 `wms-api-staging` 启动前执行数据库迁移。迁移容器成功退出后，API 才会启动；如果迁移失败，不要采集 W6.C evidence，先修复 schema 或迁移脚本。

启动服务：

```bash
docker compose --env-file deploy/env/staging.env -f deploy/docker-compose.staging.yml up -d --build
```

服务地址必须通过外部 staging 反代、内网网关或既有负载均衡暴露，例如：

```bash
export WAVE_2_ENVIRONMENT=staging
export WAVE_2_SERVICE_URL="http://wms-staging.internal"
```

若复用既有 nginx / 网关 / LB，单独为 `wms-staging.internal` 配置 server block，将请求转发到 compose 暴露的 `127.0.0.1:${WMS_STAGING_API_PORT:-18080}`；该反代配置属于现场基础设施，不纳入本仓库 compose。

## H1 Token

W6.C smoke 需要一个真实 H1 token，至少包含：

- `m1.config.write`
- `m3.read`

token 必须由 staging 的 `WMS_JWT_SECRET` 签发，并通过现场 shell 注入：

```bash
eval "$(just wave-2-h1-token \
  --user-name wave2-staging-operator \
  --jwt-secret "$WMS_JWT_SECRET")"
```

该命令只向 stdout 输出 `export WAVE_2_H1_TOKEN=...`，不会写入 token 文件。生成的 token 有效期为 1 小时，权限固定为 `m1.config.write` 和 `m3.read`。

## Evidence 采集

现场证据引用必须包含 `staging` 标记。`WMS_STAGING_RUN_ID` 必须来自现场 CI、日志平台或 artifact 系统的真实运行编号：

```bash
: "${WMS_STAGING_RUN_ID:?set WMS_STAGING_RUN_ID to the real staging CI or artifact run id}"
export WAVE_2_SMOKE_LOG_REF="ci/staging/wave2-feature-flags-smoke/${WMS_STAGING_RUN_ID}"
export WAVE_2_RECONCILE_LOG_REF="ci/staging/wave2-feature-flags-reconcile/${WMS_STAGING_RUN_ID}"
export WAVE_2_ARCHIVE_REF="artifact/staging/feature-flags/archive/${WMS_STAGING_RUN_ID}"

just wave-2-runtime-evidence-readiness
just wave-2-runtime-evidence-smoke
just wave-2-runtime-evidence-validate
```

成功后会写入 `docs/retros/wave-2-runtime-evidence.json`。如果 evidence 环境值写成“test”而不是 `staging`，validator 会拒绝该 evidence。
