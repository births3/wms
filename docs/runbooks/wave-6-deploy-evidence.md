# Wave 6 Gray Release Evidence Runbook

> 用途：关闭 Wave 6 W6.H 首次试运行发布 evidence gate。覆盖 ADR-0016 灰度发布链路、smoke gate、可观测性、回滚演练、审批和审计证据。

## 目标

证明首次试运行发布不是全量直发，而是按 ADR-0016 的灰度发布思路执行：

- 使用 docker-compose 或 Kubernetes 部署形态。
- 有明确 release plan、构建产物和灰度配置。
- 至少执行一段灰度阶段，不直接全量发布。
- smoke gate 和可观测 dashboard 可追溯。
- 回滚链路已演练并可查询日志。
- 发布审批和 `audit_event` 证据存在。

## 前置条件

- 环境必须为真实 `staging`；W6.H 是首次试运行灰度发布 gate，`dev` 只能用于前序 smoke / 工具链验证，不能关闭 W6.H。
- 不把本机 docker、localhost、127.0.0.1、0.0.0.0、local、prod、production、mock、fake、stub、example 或截图占位当作证据。
- 发布审批记录不包含密钥、token 或 webhook key。
- 生产首次试运行前，应先用本 runbook 在 staging 预演；真实投产记录由运维系统或证据库归档，不把生产密钥写入仓库。

## 必需证据

1. 发布计划：
   - 发布版本。
   - 部署形态：`docker-compose` 或 `kubernetes`。
   - release plan 归档引用。
2. 构建产物：
   - 镜像、包或 artifact 引用。
   - 对应 commit / tag / CI job。
3. 灰度配置：
   - 按租户、比例或用户群体之一执行。
   - 不允许直接全量。
4. 验证与观测：
   - smoke gate 结果。
   - dashboard 或监控查询引用。
5. 回滚：
   - 至少一次回滚演练日志。
   - 确认可以回到上一稳定版本。
6. 审批与审计：
   - 双人审批记录。
   - `audit_event` 查询证据。

## Evidence JSON

真实证据写入 `docs/retros/wave-6-deploy-evidence.json`：
以下 JSON 仅为字段结构示例，不得复制为真实 evidence；真实 evidence 必须由 record 命令生成。

```json
{
  "environment": "staging",
  "deployment_mode": "kubernetes",
  "release_version": "wms-api-20260604.1",
  "release_plan_ref": "s3://wms-staging-evidence/wave6/deploy/release-plan-20260604.md",
  "artifact_ref": "registry://wms-staging/api@sha256:abcdef",
  "canary_config_ref": "gitlab/staging/wave6-canary-config/123",
  "smoke_gate_ref": "ci/staging/wave6-smoke-gate/123",
  "observability_dashboard_ref": "grafana/staging/wave6-release/123",
  "rollback_drill_log_ref": "ci/staging/wave6-rollback-drill/123",
  "approval_record_ref": "ticket://release-approval/WMS-20260604",
  "audit_event_query_ref": "ci/staging/wave6-deploy-audit/123",
  "canary_stages_exercised": 1,
  "smoke_checks_passed": 1,
  "rollback_drills_exercised": 1,
  "canary_used": true,
  "full_release_blocked": true,
  "rollback_verified": true,
  "audit_event_verified": true,
  "dual_approval_recorded": true
}
```

字段含义：

| 字段 | 要求 |
|------|------|
| `environment` | 只能是 `staging` |
| `deployment_mode` | `docker-compose` 或 `kubernetes` |
| `release_version` | 本次发布版本，不写密钥 |
| `*_ref` | 指向 release plan、artifact、灰度配置、smoke、监控、回滚、审批或审计证据 |
| `canary_stages_exercised` | 至少 1 |
| `smoke_checks_passed` | 至少 1 |
| `rollback_drills_exercised` | 至少 1 |
| `canary_used` | 确认使用灰度链路后为 `true` |
| `full_release_blocked` | 确认未全量直发后为 `true` |
| `rollback_verified` | 回滚链路验证后为 `true` |
| `audit_event_verified` | 查询到对应审计事件后为 `true` |
| `dual_approval_recorded` | 发布审批记录满足双人确认后为 `true` |

## Staging 预演 Worksheet

本 worksheet 用于准备真实 staging 灰度预演材料。它不是 runtime evidence 文件，不能复制为 `docs/retros/wave-6-deploy-evidence.json`；真实 evidence 仍必须由 `just wave-6-deploy-evidence-record` 生成，并由 validator 校验。

当前已有一份真实 staging dry-run 材料：`docs/runbooks/wave-6-staging-deploy-dry-run.md`。该材料证明 docker-compose staging、health/readiness、受保护 API、rollback 命令链和 audit schema 前置，但仍缺不同上一稳定 artifact、CI smoke gate、dashboard、双人审批和 deploy audit_event，因此不能关闭 W6.H。

### 可由当前 staging readiness 证明

| 项 | 采集方式 | 说明 |
|----|----------|------|
| staging 服务可达 | `just wave-6-deploy-readiness --from-env --json` | 只证明 `/healthz` 返回 200 且 payload `status=ok` |
| 部署形态参数 | `--deployment-mode docker-compose` 或 `--deployment-mode kubernetes` | 只证明本次预演声明的部署形态，不能替代 release plan |
| payload 合同完整性 | readiness 复用 deploy evidence validator | 只证明字段、计数、布尔和引用格式合规 |

### 必须由外部系统归档

| 字段 / 环境变量 | 真实来源 | 最低要求 |
|----------------|----------|----------|
| `WAVE_6_RELEASE_PLAN_REF` | 发布工单、运维系统或证据库 | 包含 `staging`，说明版本、部署形态、灰度阶段和回滚窗口 |
| `WAVE_6_ARTIFACT_REF` | 镜像仓库、CI artifact 或 tag/commit 归档 | 包含 `staging`，能追溯到构建产物；不能把本机 image id 当成 artifact ref |
| `WAVE_6_CANARY_CONFIG_REF` | Git / 配置中心 / 发布系统 | 包含 `staging`，能证明按租户、比例或用户群体之一灰度 |
| `WAVE_6_SMOKE_GATE_REF` | CI smoke job、发布平台 gate 或归档日志 | 包含 `staging`，至少 1 项通过；不能把 `/healthz` 200 当成 smoke gate |
| `WAVE_6_OBSERVABILITY_DASHBOARD_REF` | Grafana、Prometheus 查询或监控归档 | 包含 `staging`，能追溯到发布窗口 |
| `WAVE_6_ROLLBACK_DRILL_LOG_REF` | 发布系统、CI job 或回滚演练日志 | 包含 `staging`，至少 1 次回滚演练 |
| `WAVE_6_APPROVAL_RECORD_REF` | 审批工单或发布审批系统 | 包含 `staging`，满足双人审批，不包含密钥 |
| `WAVE_6_AUDIT_EVENT_QUERY_REF` | `audit_event` 查询归档或 CI 查询日志 | 包含 `staging`，能证明发布动作进入审计链路 |

## 发布审计写入

`audit_event_query_ref` 必须来自正式发布工具写入后的查询引用。不要手工 `INSERT INTO audit_event`，也不要把 `audit_event` 当前 count 当作发布审计证据。

staging docker-compose 形态使用运行中 API 容器内的 `wms-deploy-audit` 写入 append-only 审计表。先确认 staging 已用目标镜像 `up -d --no-build` 启动，避免 `deploy/docker-compose.staging.yml` 的 `pull_policy: build` 在审计命令前触发重建。`module` / `action` / `resource_type` / `resource_id` 由发布计划或工单明确给出，工具不隐式决定审计语义。

正式写入前先运行 env-driven `--check-only`。该模式只校验 W6.H staging-only 边界、外部 ref、H1 actor / owner 和计数字段，输出 `writes_audit_event=false`、`writes_runtime_evidence=false`、`closes_gate=false`；不要求 `DATABASE_URL`，不连接数据库，不写 `audit_event`，也不写 `docs/retros/wave-6-deploy-evidence.json`：

```bash
just wave-6-deploy-audit --from-env --check-only
```

`--from-env` 会从 `WAVE_6_DEPLOY_ACTOR_ID`、`WAVE_6_DEPLOY_ACTOR_NAME`、`WAVE_6_DEPLOY_OWNER_ID` 和 `WAVE_6_DEPLOY_JTI` 填充底层 `--actor-id`、`--actor-name`、`--owner-id` 和 `--jti` 参数；这些值仍必须来自 H1 发布人员、灰度 owner 和发布工单。

`--check-only` 通过后，且确认本次发布窗口、审批、artifact 和灰度配置均真实归档，再移除 `--check-only` 执行正式审计写入：

```bash
: "${WAVE_6_DEPLOY_ACTOR_ID:?set H1 release actor UUID}"
: "${WAVE_6_DEPLOY_ACTOR_NAME:?set H1 release actor name}"
: "${WAVE_6_DEPLOY_OWNER_ID:?set canary owner UUID or confirmed system owner UUID}"
: "${WAVE_6_DEPLOY_JTI:?set unique deploy run id}"
: "${WAVE_6_DEPLOY_MODULE:?set audit module from release plan}"
: "${WAVE_6_DEPLOY_ACTION:?set audit action from release plan}"
: "${WAVE_6_DEPLOY_RESOURCE_TYPE:?set audit resource type from release plan}"
: "${WAVE_6_DEPLOY_RESOURCE_ID:?set audit resource id from release plan}"

sudo docker compose \
  --env-file deploy/env/staging.env \
  -f deploy/docker-compose.staging.yml \
  exec -T wms-api-staging /app/wms-deploy-audit --from-env
```

命令成功后会输出 JSON，其中 `audit_event_query_ref` 才能写入 `WAVE_6_AUDIT_EVENT_QUERY_REF`。如果缺少 H1 发布 actor、灰度 owner、registry artifact、审批或其他外部 ref，只能记录 blocker，不能写正式 W6.H evidence。

本地运维也可以通过 just 入口调用同一工具；该入口只负责写 `audit_event` 并输出 `audit_event_query_ref`，不写 `docs/retros/wave-6-deploy-evidence.json`：

```bash
just wave-6-deploy-audit --from-env
```

### 预演命令顺序

`just wave-6-deploy-materials --from-env --json` 会输出 `execution_plan`，按真实依赖展示下一步：materials → deploy_audit_check_only → deploy_audit_record → readiness → evidence_record_check_only → evidence_record → validate。先执行 deploy audit 正式写入，取得 `audit_event_query_ref` 后填入 `WAVE_6_AUDIT_EVENT_QUERY_REF`，再运行 readiness；否则 readiness 只能报告审计查询引用缺失，不能进入 evidence record。

`just wave-6-deploy-materials --from-env --json` 还会输出 `missing_env_by_stage` 和 `next_blocking_stage`：先补 `pre_audit` 缺口，再运行 deploy audit；deploy audit 输出 `audit_event_query_ref` 后再补 `post_audit` 缺口。不要在 deploy audit 之前手工伪造 `WAVE_6_AUDIT_EVENT_QUERY_REF`。

先生成一份不包含密钥的变量模板，再按真实 staging 发布材料填入引用：

```bash
just wave-6-deploy-materials --export-template
```

```bash
: "${WAVE_6_SERVICE_URL:?set staging service URL}"
: "${WAVE_6_RELEASE_VERSION:?set release version}"
: "${WAVE_6_RELEASE_PLAN_REF:?set release plan evidence ref}"
: "${WAVE_6_ARTIFACT_REF:?set artifact evidence ref}"
: "${WAVE_6_CANARY_CONFIG_REF:?set canary config evidence ref}"
: "${WAVE_6_SMOKE_GATE_REF:?set smoke gate evidence ref}"
: "${WAVE_6_OBSERVABILITY_DASHBOARD_REF:?set observability evidence ref}"
: "${WAVE_6_ROLLBACK_DRILL_LOG_REF:?set rollback drill evidence ref}"
: "${WAVE_6_APPROVAL_RECORD_REF:?set approval evidence ref}"
: "${WAVE_6_DEPLOY_ACTOR_ID:?set H1 release actor UUID}"
: "${WAVE_6_DEPLOY_ACTOR_NAME:?set H1 release actor name}"
: "${WAVE_6_DEPLOY_OWNER_ID:?set canary owner UUID or confirmed system owner UUID}"
: "${WAVE_6_DEPLOY_JTI:?set unique deploy run id}"
: "${WAVE_6_DEPLOY_MODULE:?set audit module from release plan}"
: "${WAVE_6_DEPLOY_ACTION:?set audit action from release plan}"
: "${WAVE_6_DEPLOY_RESOURCE_TYPE:?set audit resource type from release plan}"
: "${WAVE_6_DEPLOY_RESOURCE_ID:?set audit resource id from release plan}"

just wave-6-deploy-materials --from-env --json
just wave-6-deploy-audit --from-env --check-only
just wave-6-deploy-audit --from-env

: "${WAVE_6_AUDIT_EVENT_QUERY_REF:?set audit event evidence ref from wave-6-deploy-audit output}"

just wave-6-deploy-readiness --from-env --json
just wave-6-deploy-evidence-record --from-env --check-only --json
just wave-6-deploy-evidence-record --from-env --json
just wave-6-deploy-evidence-validate
```

readiness 通过后，才能进入下一段 record。若 readiness 失败，只修外部材料或 staging 服务，不手工编辑 runtime evidence JSON。

## 验证命令

所有 release plan、构建产物、灰度配置、smoke gate、dashboard、回滚演练、审批和审计证据引用必须包含 `staging` 环境标记，并且不能指向 local / dev / prod / production / mock / fake / stub / example。

先运行只读 materials，再执行 deploy audit check-only 和正式 deploy audit 写入，取得 `audit_event_query_ref` 后再运行 readiness，确认外部材料变量齐备、staging 服务可达、payload 合同完整。materials、deploy audit check-only、readiness 和 evidence record check-only 都不会写入 `docs/retros/wave-6-deploy-evidence.json`，也不能关闭 W6.H gate。正式 evidence record 前，用同一组参数追加 `--check-only` 做 recorder 级预检；该模式只复用正式 validator 校验证据字段和引用边界，不写 deploy evidence JSON，不能关闭 W6.H gate。

```bash
just wave-6-deploy-materials --from-env --json
just wave-6-deploy-audit --from-env --check-only
just wave-6-deploy-audit --from-env
just wave-6-deploy-readiness --from-env --json
just wave-6-deploy-evidence-record --from-env --check-only --json
just wave-6-deploy-evidence-record --from-env --json
just wave-6-deploy-evidence-validate
```

## 拒绝边界

- `environment` 不是 `staging`。
- 任一证据引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`dev`、`prod`、`production`、`mock`、`fake`、`stub`、`example`。
- 任一证据引用保留模板占位，如 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认`。
- `deployment_mode` 不是 ADR-0016 已确认的 `docker-compose` 或 `kubernetes`。
- 没有灰度配置，或 `full_release_blocked` 不是 `true`。
- 没有 smoke gate、dashboard、回滚演练、审批或 `audit_event` 查询证据。

## 完成判定

W6.H 的完成判定以 `just wave-6-deploy-evidence-validate` 通过为准。没有真实 dev/staging 灰度发布预演时，只能完成 runbook / validator，不能关闭 gate。
