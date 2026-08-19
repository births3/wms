# Wave 6 Staging Deploy Dry Run - 2026-06-08

> 用途：记录 W6.H 灰度发布真实 staging 预演材料。本文件不是 `docs/retros/wave-6-deploy-evidence.json`，不能关闭 W6.H gate。

## 结论

本次预演推进了 W6.H 的 staging 服务、docker-compose 回滚命令链、smoke 前置和数据库 schema 事实，但仍不能写入最终 deploy evidence。

| 项 | 结果 | 说明 |
|----|------|------|
| staging 服务可达 | 通过 | `/healthz` 与 `/readyz` 返回 200，payload `status=ok` |
| 受保护 API 边界 | 通过 | `/api/v1/inventory/batches` 无 token 返回 401 / `AUTH-001` |
| docker-compose 回滚命令链 | 通过 | `wave1_rollback.sh --target docker-compose --environment staging --execute` 退出 0 |
| rollback 后服务健康 | 通过 | API 容器重建后为 healthy，smoke 仍通过 |
| 不同本地 image id 回滚切换 | 通过 | 补充演练中 `latest` 与 `previous-staging-sha` 为不同 image id，rollback 后 API 切到 `previous-staging-sha` |
| rollback 后恢复 latest | 通过 | 补充演练后已执行 compose 默认启动，API 恢复为 `wms-api-staging:latest` 且 healthy |
| deploy audit 工具入镜像 | 通过 | `wms-api-staging:latest` 已包含 `/app/wms-deploy-audit`，运行中 API 容器参数校验顺序符合预期 |
| audit schema | 通过 | `audit_event` 表存在，public schema 38 张表 |
| deploy audit event | 未通过 | `audit_event` 当前 0 条，不能声明 `audit_event_verified=true` |
| artifact 可追溯 | 未通过 | 补充演练只证明不同本地 image id 切换；仍无 registry / CI artifact digest |
| 灰度配置 | 未通过 | 未提供租户 / 比例 / 用户群体灰度配置引用 |
| smoke gate 归档 | 未通过 | 当前只有现场 curl 结果，不是 CI / 发布平台 smoke gate ref |
| dashboard | 未通过 | 未提供 Grafana / Prometheus / 日志查询归档 |
| 双人审批 | 未通过 | 未提供审批系统记录，不能声明 `dual_approval_recorded=true` |

## 执行窗口

| 字段 | 值 |
|------|----|
| 本地时间 | `2026-06-08T00:04:25+08:00` |
| UTC 时间 | `2026-06-07T16:04:25+00:00` |
| Git branch | `feature/horizontal-capabilities-hint-hfile-hapv-hsch` |
| Git commit | `243f87a12a59ef047f3e3e3a3970d837b03182cc` |
| deployment mode | `docker-compose` |
| service URL | `http://wms-staging.internal` |

## Compose 状态

命令：

```bash
sudo docker compose --env-file deploy/env/staging.env -f deploy/docker-compose.staging.yml ps --format json
```

关键结果：

| 服务 | 镜像 | 状态 |
|------|------|------|
| `postgres-staging` | `postgres:16` | `running` / `healthy` |
| `redis-staging` | `redis:7` | `running` / `healthy` |
| `wms-api-staging` | `wms-api-staging:previous-staging-sha` | `running` / `healthy` |

API 容器在 rollback drill 后重建：

```text
image=sha256:ae874275c8506b885f340c0e733806bc17cfc5046dba1a9f7478af9e33fbb4c2
config_image=wms-api-staging:previous-staging-sha
created=2026-06-07T16:03:38.936211256Z
started=2026-06-07T16:03:50.2251565Z
```

## Smoke 结果

命令：

```bash
curl --noproxy wms-staging.internal -fsS http://wms-staging.internal/healthz
curl --noproxy wms-staging.internal -fsS http://wms-staging.internal/readyz
curl --noproxy wms-staging.internal -i -sS http://wms-staging.internal/api/v1/inventory/batches
```

结果摘要：

```json
{"status":"ok","version":"0.0.1","generated_at":"2026-06-07T16:04:05.299764866Z"}
{"status":"ok","version":"0.0.1","generated_at":"2026-06-07T16:04:05.311682400Z"}
```

受保护接口返回：

```text
HTTP/1.1 401 Unauthorized
{"code":"AUTH-001","message":"缺少 Authorization 头","severity":"error","details":{},"trace_id":"unavailable","retry_hint":null}
```

## Rollback Drill

命令：

```bash
sudo bash deploy/scripts/wave1_rollback.sh \
  --target docker-compose \
  --environment staging \
  --previous-version previous-staging-sha \
  --compose-file deploy/docker-compose.staging.yml \
  --compose-env-file deploy/env/staging.env \
  --execute
```

结果：

```text
wave1 rollback target=docker-compose environment=staging execute=true compose_file=/home/test1/workspace/wms/deploy/docker-compose.staging.yml compose_env_file=/home/test1/workspace/wms/deploy/env/staging.env previous_version=previous-staging-sha
Container wms-staging-wms-db-migrate-staging-1  Exited
Container wms-staging-wms-api-staging-1  Started
```

限制：

- `wms-api-staging:latest` 与 `wms-api-staging:previous-staging-sha` 当前都指向 `sha256:ae874275c8506b885f340c0e733806bc17cfc5046dba1a9f7478af9e33fbb4c2`。
- 因此本次只能证明 docker-compose rollback 命令链可执行，不能证明回到一个不同的上一稳定 artifact。

## Rollback Drill 补充：不同本地 Image ID

为消除“`latest` 与 `previous-staging-sha` 同 image id，回滚没有实际切换”的弱点，补充执行一次本地 staging 回滚机制演练。

先从当前 healthy 的 staging `latest` 容器生成演练用 previous 镜像。该镜像用于验证 compose 切换机制，不是 registry / CI artifact：

```bash
sudo docker commit \
  --change 'LABEL wms.wave6.rollback_drill=2026-06-08T00:13:05+08:00' \
  --change 'LABEL wms.wave6.source=staging-latest-container' \
  --change 'LABEL wms.wave6.git_head=243f87a12a59' \
  wms-staging-wms-api-staging-1 \
  wms-api-staging:previous-staging-sha
```

镜像状态：

```text
wms-api-staging:latest                sha256:ae874275c8506b885f340c0e733806bc17cfc5046dba1a9f7478af9e33fbb4c2
wms-api-staging:previous-staging-sha  sha256:16b003b735ddb9d754410d450f71596eefc0f97baabd0b7f8d8cefa72d5cee0b
```

再次执行 rollback drill：

```bash
sudo bash deploy/scripts/wave1_rollback.sh \
  --target docker-compose \
  --environment staging \
  --previous-version previous-staging-sha \
  --compose-file deploy/docker-compose.staging.yml \
  --compose-env-file deploy/env/staging.env \
  --execute
```

结果：

```text
wave1 rollback target=docker-compose environment=staging execute=true compose_file=/home/test1/workspace/wms/deploy/docker-compose.staging.yml compose_env_file=/home/test1/workspace/wms/deploy/env/staging.env previous_version=previous-staging-sha
Container wms-staging-wms-api-staging-1  Started
```

rollback 后 API 容器事实：

```text
image=wms-api-staging:previous-staging-sha
image_id=sha256:16b003b735ddb9d754410d450f71596eefc0f97baabd0b7f8d8cefa72d5cee0b
status=running
health=healthy
started=2026-06-07T16:14:06.305878834Z
```

rollback 后 smoke：

```text
/healthz HTTP 200 payload.status=ok
/readyz HTTP 200 payload.status=ok
/api/v1/inventory/batches without token HTTP 401 code=AUTH-001
```

演练完成后恢复 staging 默认 `latest`：

```bash
sudo docker compose --env-file deploy/env/staging.env -f deploy/docker-compose.staging.yml up -d --no-build
```

恢复后 API 容器事实：

```text
image=wms-api-staging:latest
image_id=sha256:ae874275c8506b885f340c0e733806bc17cfc5046dba1a9f7478af9e33fbb4c2
status=running
health=healthy
started=2026-06-07T16:14:39.204679754Z
```

恢复后 smoke：

```text
/healthz HTTP 200 payload.status=ok
/readyz HTTP 200 payload.status=ok
/api/v1/inventory/batches without token HTTP 401 code=AUTH-001
```

限制：

- 本补充演练证明 docker-compose 回滚脚本能让 API 容器切换到不同本地 image id，并能恢复到 `latest`。
- 本补充演练仍不能替代 `artifact_ref`：真实 W6.H evidence 仍需要 registry digest、CI artifact 或可追溯 tag / commit 归档。

## Deploy Audit CLI 镜像验证补充

为接通 W6.H 发布审计写入链路，补充构建并验证包含 `wms-deploy-audit` 的 staging 镜像。该步骤只证明工具链可用；没有真实外部 ref 和 H1 actor / owner 前，不写正式 `audit_event`。

| 字段 | 值 |
|------|----|
| 本地时间 | `2026-06-08T11:40:06+08:00` |
| UTC 时间 | `2026-06-08T03:40:06+00:00` |
| Git commit | `243f87a12a59` |
| image tag | `wms-api-staging:latest` |
| image id | `sha256:b236c9f4da1dde1c14a7525d51ea9cb2ac21beb5a47246f67a47c15bbd1f0e09` |
| image created | `2026-06-08T11:38:32.230634039+08:00` |

镜像内 CLI 参数校验顺序：

```bash
sudo docker run --rm --entrypoint /app/wms-deploy-audit wms-api-staging:latest --module W6.H
```

结果：

```text
Error: Custom { kind: InvalidInput, error: "--action is required" }
```

完整参数但无 DB URL 时才要求数据库连接：

```text
Error: Custom { kind: InvalidInput, error: "DATABASE_URL or WMS_DB_URL is required" }
```

用新镜像恢复 staging：

```bash
sudo docker compose --env-file deploy/env/staging.env -f deploy/docker-compose.staging.yml up -d --no-build
```

恢复后 API 容器事实：

```text
image=wms-api-staging:latest
image_id=sha256:b236c9f4da1dde1c14a7525d51ea9cb2ac21beb5a47246f67a47c15bbd1f0e09
status=running
health=healthy
created=2026-06-08T03:39:12.021255518Z
started=2026-06-08T03:39:23.426121154Z
```

镜像内文件验证：

```text
-rwxr-xr-x 1 root root 6153000 Jun  8 03:38 /app/wms-deploy-audit
```

恢复后 smoke：

```text
/healthz HTTP 200 payload.status=ok
/readyz HTTP 200 payload.status=ok
/api/v1/inventory/batches without token HTTP 401 code=AUTH-001
```

运行中 API 容器内的 CLI 参数校验：

```bash
sudo docker compose --env-file deploy/env/staging.env -f deploy/docker-compose.staging.yml exec -T wms-api-staging /app/wms-deploy-audit --module W6.H
```

结果：

```text
Error: Custom { kind: InvalidInput, error: "--action is required" }
```

审计表计数仍为 0：

```bash
sudo docker compose --env-file deploy/env/staging.env -f deploy/docker-compose.staging.yml exec -T postgres-staging \
  psql -U wms_staging -d wms_staging -Atc "SELECT count(*) FROM audit_event;"
```

结果：

```text
0
```

含义：

- `wms-deploy-audit` 已进入 staging 镜像和运行中 API 容器。
- 工具先校验参数，再要求 DB URL / DB 连接。
- 本次没有写入正式 deploy `audit_event`，因此仍不能设置 `audit_event_verified=true`。

## DB / Audit 查询

命令：

```bash
sudo docker compose --env-file deploy/env/staging.env -f deploy/docker-compose.staging.yml exec -T postgres-staging \
  psql -U wms_staging -d wms_staging -Atc "SELECT to_regclass('public.audit_event'); SELECT count(*) FROM information_schema.tables WHERE table_schema='public'; SELECT count(*) FROM audit_event;"
```

结果：

```text
audit_event
38
0
```

含义：

- `audit_event` schema 已存在。
- staging 数据库 migration 已跑到包含 38 张 public 表的状态。
- 当前没有 deploy / release 审计事件，不能设置 `audit_event_verified=true`。

## Materials 阶段化阻塞复核

2026-06-11 复核当前 materials 链路，按最新工具入口使用 `just wave-6-deploy-materials --from-env --json`；仅填入当前可验证的 staging 事实：

```bash
WAVE_6_SERVICE_URL=http://wms-staging.internal \
WAVE_6_ENVIRONMENT=staging \
WAVE_6_DEPLOYMENT_MODE=docker-compose \
just wave-6-deploy-materials --from-env --json
```

结果仍为阻塞状态：

```text
ok = false
next_blocking_stage = `pre_audit`
writes_runtime_evidence = false
closes_gate = false
```

阶段化缺口：

| 阶段 | 当前结论 | 处理边界 |
|------|----------|----------|
| `pre_audit` | `pre_audit` 当前仍缺发布版本、外部 ref、审计语义和 H1 actor / owner | 必须先补齐真实 `WAVE_6_RELEASE_VERSION`、发布计划、artifact、灰度配置、smoke gate、dashboard、rollback、审批、`WAVE_6_DEPLOY_*` |
| `deploy_audit_record` | 不能执行 | 缺 H1 actor / owner 和外部 ref 前，不写 `audit_event` |
| `post_audit` | `post_audit` 必须等待正式 deploy audit 输出 `WAVE_6_AUDIT_EVENT_QUERY_REF` | 不能提前手工伪造 `WAVE_6_AUDIT_EVENT_QUERY_REF` |
| `evidence_record` | 不能执行 | 只有 deploy audit、readiness、record check-only 都通过后，才能写正式 evidence JSON |

当前不能预填为 `true` 的布尔字段：

- 不能把 `WAVE_6_AUDIT_EVENT_VERIFIED` 预填为 `true`：当前 `audit_event` 为 0 条，尚未写入发布审计事件。
- 不能把 `WAVE_6_DUAL_APPROVAL_RECORDED` 预填为 `true`：当前没有双人审批记录引用。

可安全带入下一轮采集的事实只有：

| 环境变量 | 当前值 | 依据 |
|----------|--------|------|
| `WAVE_6_SERVICE_URL` | `http://wms-staging.internal` | `/healthz` 与 `/readyz` HTTP 200 |
| `WAVE_6_ENVIRONMENT` | `staging` | W6.H 只能由 staging evidence 关闭 |
| `WAVE_6_DEPLOYMENT_MODE` | `docker-compose` | 当前 staging compose 形态 |

需要外部确认或归档后才能填入的字段：

| 字段 | 需要来源 |
|------|----------|
| `WAVE_6_RELEASE_VERSION` | 发布计划或 CI release tag |
| `WAVE_6_RELEASE_PLAN_REF` | 发布工单 / 运维系统 / 证据库 |
| `WAVE_6_ARTIFACT_REF` | registry digest / CI artifact；不能使用本机 image id |
| `WAVE_6_CANARY_CONFIG_REF` | Git / 配置中心 / 发布系统灰度配置 |
| `WAVE_6_SMOKE_GATE_REF` | CI / 发布平台 smoke gate；不能把 `/healthz` 200 当成 smoke gate |
| `WAVE_6_OBSERVABILITY_DASHBOARD_REF` | Grafana / Prometheus / 日志查询归档 |
| `WAVE_6_ROLLBACK_DRILL_LOG_REF` | 发布系统或 CI 回滚演练归档 |
| `WAVE_6_APPROVAL_RECORD_REF` | 双人审批记录 |
| `WAVE_6_DEPLOY_ACTOR_ID` | H1 发布 actor UUID |
| `WAVE_6_DEPLOY_ACTOR_NAME` | H1 发布 actor 名称 |
| `WAVE_6_DEPLOY_OWNER_ID` | 灰度 owner UUID 或确认后的系统 owner UUID |
| `WAVE_6_DEPLOY_JTI` | 本次发布唯一 run id |
| `WAVE_6_AUDIT_EVENT_QUERY_REF` | 正式 deploy audit 输出的查询引用 |

## 最终 Evidence 缺口

要写入 `docs/retros/wave-6-deploy-evidence.json`，仍需外部提供以下真实 ref：

| 字段 | 当前状态 |
|------|----------|
| `release_plan_ref` | 缺真实发布计划归档 |
| `artifact_ref` | 缺 registry / CI artifact digest；本地不同 image id 演练不够 |
| `canary_config_ref` | 缺灰度配置归档 |
| `smoke_gate_ref` | 缺 CI / 发布平台 smoke gate 归档 |
| `observability_dashboard_ref` | 缺 dashboard / query 归档 |
| `rollback_drill_log_ref` | 有本次命令链和不同本地 image id 切换材料，但还缺外部回滚演练归档引用 |
| `approval_record_ref` | 缺双人审批记录 |
| `audit_event_query_ref` | 缺 deploy 审计事件查询结果 |

## 下一步

1. 通过 registry / CI artifact 生成可追溯 `artifact_ref`，不要使用本机 image id。
2. 通过发布系统或 CI 归档 rollback drill log ref。
3. 通过 CI / 发布平台生成 smoke gate ref，而不是只用现场 curl。
4. 接入或归档 dashboard / Prometheus / 日志查询。
5. 补双人审批记录。
6. 用 `wms-deploy-audit` 在真实外部 ref、H1 actor 和 owner 到位后写入发布 `audit_event`，并归档输出的 `audit_event_query_ref`。
7. 上述完成后运行 `just wave-6-deploy-materials --from-env --json`，再按顺序运行 `just wave-6-deploy-audit --from-env --check-only`、`just wave-6-deploy-audit --from-env`、`just wave-6-deploy-readiness --from-env --json`、`just wave-6-deploy-evidence-record --from-env --check-only --json` 和 `just wave-6-deploy-evidence-record --from-env --json`。
