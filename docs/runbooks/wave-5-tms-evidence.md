# Wave 5 TMS+ Evidence Runbook

> 用途：关闭 Wave 6 W6.G 中从 Wave 5 后移的 M10 TMS+ 真实 dev/staging evidence gate。覆盖调度推送、TMS 回调、失败重试和审计查询。

## 目标

证明 M10 TMS+ 在真实 dev/staging TMS 边界下可以完成以下链路：

- WMS 向 TMS 推送至少 1 条调度。
- TMS 回调至少 1 条调度状态。
- 可控失败回调或失败推送能触发重试。
- 重试最终成功。
- 推送、回调和重试均能查询到对应 `audit_event`。

## 前置条件

- 环境为 `dev` 或 `staging`，不得使用 `local` / `prod`。
- 外部 TMS 测试环境、回调地址和鉴权方式已确认。
- M10 TMS+ API、PostgreSQL migration、H2 audit_event 已部署到同一环境。
- TMS 凭证通过 ADR-0013 约定的 secrets 机制注入，不写入仓库。
- 推送日志、回调日志、重试日志和审计查询结果已归档到证据库。

## 必需证据

1. 外部系统证据：
   - TMS 测试环境或租户引用。
   - Vault 中的凭证引用，不能写明文 token。
2. 调度推送证据：
   - 至少 1 条 WMS → TMS 推送日志。
   - TMS 接收成功或可查询的调度流水。
3. 回调证据：
   - 至少 1 条 TMS → WMS 回调日志。
   - 回调能更新 WMS 调度状态。
4. 失败重试证据：
   - 至少 1 条可控失败。
   - 重试最终成功。
5. 审计证据：
   - `audit_event` 查询结果引用。
   - 审计事件能关联调度、回调或重试资源。

## Evidence JSON

真实证据写入 `docs/retros/wave-5-tms-evidence.json`：

```json
{
  "environment": "staging",
  "tms_system_ref": "partner://wms-staging/tms/vendor-a",
  "dispatch_push_log_ref": "ci/staging/wave5-tms-dispatch-push/123",
  "callback_log_ref": "ci/staging/wave5-tms-callback/123",
  "failure_retry_log_ref": "ci/staging/wave5-tms-failure-retry/123",
  "audit_event_query_ref": "ci/staging/wave5-tms-audit/123",
  "credential_ref": "vault://wms/staging/tms/vendor-a",
  "dispatches_received": 1,
  "callbacks_received": 1,
  "failed_callbacks_exercised": 1,
  "retry_succeeded": true,
  "audit_event_verified": true
}
```

字段含义：

| 字段 | 要求 |
|------|------|
| `environment` | 只能是 `dev` 或 `staging` |
| `tms_system_ref` | 真实 TMS dev/staging 租户、系统或工单引用 |
| `credential_ref` | 必须以 `vault://` 开头，不能写明文凭证 |
| `dispatch_push_log_ref` | WMS 推送调度到 TMS 的真实日志引用 |
| `callback_log_ref` | TMS 回调 WMS 的真实日志引用 |
| `failure_retry_log_ref` | 失败与重试链路的真实日志引用 |
| `audit_event_query_ref` | 对应 `audit_event` 查询证据 |
| `dispatches_received` | 至少 1 |
| `callbacks_received` | 至少 1 |
| `failed_callbacks_exercised` | 至少 1 |
| `retry_succeeded` | 重试成功后为 `true` |
| `audit_event_verified` | 查询到对应审计事件后为 `true` |

## 验证命令

```bash
just wave-5-tms-evidence-validate
```

该命令只验证 evidence JSON 的完整性和边界，不负责调用 TMS。外部联调必须先在真实 dev/staging 环境完成。

## 拒绝边界

- `environment` 是 `local` / `prod` / `production`。
- 任一证据引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`prod`、`production`、`mock`、`fake`、`stub`、`example`。
- `credential_ref` 不是 `vault://` 引用。
- 只有 WMS 内部 handler 测试，没有真实 TMS 推送或回调记录。
- 计数为 0。
- 失败重试未实际触发或最终未成功。
- 查不到对应 `audit_event`。

## 完成判定

W6.G 的完成判定以 `just wave-5-tms-evidence-validate` 通过为准。没有真实 TMS dev/staging 和回调日志时，只能完成 runbook / validator，不能关闭 gate。
