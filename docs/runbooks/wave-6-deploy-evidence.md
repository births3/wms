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

- 环境为真实 `dev` 或 `staging`；没有稳定环境时只能补 runbook / validator，不能关闭 gate。
- 不把本机 docker、localhost、mock、stub、fake 或截图占位当作证据。
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

```json
{
  "environment": "staging",
  "deployment_mode": "kubernetes",
  "release_version": "wms-api-20260604.1",
  "release_plan_ref": "s3://wms-staging-evidence/wave6/deploy/release-plan-YYYYMMDD.md",
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
| `environment` | 只能是 `dev` 或 `staging` |
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

## 验证命令

```bash
just wave-6-deploy-evidence-validate
```

## 拒绝边界

- `environment` 是 `local` / `prod` / `production`。
- 任一证据引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`prod`、`production`、`mock`、`fake`、`stub`、`example`。
- `deployment_mode` 不是 ADR-0016 已确认的 `docker-compose` 或 `kubernetes`。
- 没有灰度配置，或 `full_release_blocked` 不是 `true`。
- 没有 smoke gate、dashboard、回滚演练、审批或 `audit_event` 查询证据。

## 完成判定

W6.H 的完成判定以 `just wave-6-deploy-evidence-validate` 通过为准。没有真实 dev/staging 灰度发布预演时，只能完成 runbook / validator，不能关闭 gate。
