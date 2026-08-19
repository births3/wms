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

- 环境为 `dev` 或 `staging`，不得使用 `local` / `prod` / `production`。
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
以下 JSON 仅为字段结构示例，不得复制为真实 evidence；真实 evidence 必须由 record 命令生成。

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

所有 TMS 系统、推送、回调、失败重试、Vault 和审计证据引用必须包含当前 `environment` 标记（`dev` 或 `staging`），并且不能指向 local / prod / production / mock / fake / stub / example。

先导出现场材料变量模板。该命令只输出变量清单和 check-only 命令，不调用 TMS，不写 `docs/retros/wave-5-tms-evidence.json`，不能关闭 W6.G gate：

```bash
just wave-5-tms-materials --export-template
```

```bash
just wave-5-tms-materials --from-env --json
just wave-5-tms-readiness --from-env --json
just wave-5-tms-evidence-record --from-env --check-only --json
just wave-5-tms-evidence-record --from-env --json
just wave-5-tms-evidence-validate
```

`just wave-5-tms-materials --from-env --json`、`just wave-5-tms-readiness --from-env --json` 和 `just wave-5-tms-evidence-record --from-env --check-only --json` 只校验字段、证据引用和 dev/staging 边界；不调用 TMS，不写 `docs/retros/wave-5-tms-evidence.json`，不能关闭 W6.G gate。外部联调必须先在真实 dev/staging 环境完成；`record --from-env --json` 写入真实 evidence 后，`validate` 只验证 evidence JSON 的完整性和边界。

### 现场执行包完成标准

W6.G 现场执行包完成，不等于真实 TMS evidence 完成。现场执行包完成标准是：

1. `just wave-5-tms-materials --export-template` 能输出完整 `WAVE_5_TMS_*` 变量清单和后续命令。
2. TMS 对接方和测试负责人只需要填入真实 TMS 系统引用、推送日志、回调日志、失败重试日志、Vault 凭证引用和 `audit_event` 查询引用，不需要拼长参数。
3. `just wave-5-tms-materials --from-env --json` 和 `just wave-5-tms-readiness --from-env --json` 能定位缺失变量及负责人。
4. `just wave-5-tms-evidence-record --from-env --check-only --json` 通过后，现场同事执行一条正式命令 `just wave-5-tms-evidence-record --from-env --json` 生成 `docs/retros/wave-5-tms-evidence.json`。
5. 正式 record 后必须立即执行 `just wave-5-tms-evidence-validate`；只有 validator 通过才关闭 W6.G。

## 拒绝边界

- `environment` 是 `local` / `prod` / `production`。
- 任一证据引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`prod`、`production`、`mock`、`fake`、`stub`、`example`。
- 任一证据引用保留模板占位，如 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认`。
- `credential_ref` 不是 `vault://` 引用。
- 只有 WMS 内部 handler 测试，没有真实 TMS 推送或回调记录。
- 计数为 0。
- 失败重试未实际触发或最终未成功。
- 查不到对应 `audit_event`。

## 完成判定

W6.G 的完成判定以 `just wave-5-tms-evidence-validate` 通过为准。没有真实 TMS dev/staging 和回调日志时，只能完成 runbook / validator，不能关闭 gate。
