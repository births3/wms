# Wave 4 External Dependency Evidence Runbook

> 用途：关闭 Wave 6 W6.E 中从 Wave 4 后移的外部依赖 evidence gate。当前主要覆盖 W4.D "码上放心"平台上报；内部三元组契约不能替代正式平台适配证据。

## 目标

证明 M-TC 追溯码核销事件可以按正式"码上放心"接口规范上报到真实测试环境，并满足 W6.E 预发布证据标准：

- 账号 / 租户 / 测试环境已开通
- 正式接口文档已归档，包含字段名、鉴权方式、错误码、频率限制
- WMS 上报内容遵守已确认边界：追溯码 + 状态变更类型 + 时间戳
- 平台不可用时，本地待补报队列可保留事件，不阻塞 WMS 业务
- 上报成功、失败、重试、待补报均有审计记录

## 前置条件

- 已完成 W4.A 出库复核 / 发货链路，能产生追溯码出库核销事件
- 已部署包含 M-TC 上报接口的 `wms-api`
- 环境为 `dev` 或 `staging`
- "码上放心"平台提供真实测试环境地址，不使用生产环境
- 凭证通过 ADR-0013 约定的 secrets 机制注入，不写入仓库
- H2 审计追踪可记录上报请求、响应摘要、重试和补报动作

## 必需证据

1. 平台资料归档引用：
   - 正式接口文档版本
   - 鉴权方式说明
   - 错误码清单
   - 频率限制或限流策略
2. 测试环境凭证开通记录：
   - 账号 / 租户标识
   - 开通日期
   - 凭证存放位置引用，不能包含密钥明文
3. 成功上报证据：
   - 至少 1 条出库核销事件
   - 平台返回成功码
   - 平台流水号或可查询回执
4. 失败与重试证据：
   - 至少 1 条可控失败响应
   - 重试次数与间隔符合配置
   - 最终状态为已上报或待补报
5. 审计证据：
   - 上报动作写入 `audit_event`
   - 审计事件包含 trace ID / resource ID / 平台响应摘要

## 公开资料入口

以下公开入口只能作为申请账号、归档正式文档和核对接口边界的起点，不能替代 dev/staging 真实回执证据：

- 码上放心开放平台说明：`https://www.mashangfangxin.com/help/openPlatform`
  - 覆盖开发者注册、创建应用、接口调用、联调上线流程。
  - 公开说明提到未上线应用有调用次数限制，正式限流仍以平台实际开通和联调文档为准。
- 码上放心数据采集设备接入标准：`https://www.mashangfangxin.com/attach/码上放心数据采集设备接入标准.pdf`
  - 可用于核对采购入库、核销出库、扫码、上传等追溯数据采集接口语义。

归档时应把业务方或平台提供的正式版本另存到证据库，并在 Evidence JSON 中引用归档地址；不要直接把公开网页当成成功上报、失败重试或审计证据。

## 模板命令

先导出只读材料模板（不写任何 evidence JSON）：

```bash
just wave-4-external-dependencies-record --export-template
```

模板命令会输出 `WAVE_4_EXTERNAL_*` 变量、readiness、record check-only、正式 record 和 validate 命令；填充完成后先运行模板中的只读命令。

```bash
just wave-4-external-dependencies-record --export-template
```

只读边界说明：

- `--export-template` / `readiness --from-env` / `record --from-env --check-only` 都不写 `docs/retros/wave-4-external-dependencies.json`。
- 校验通过仅表示材料字段与边界自检通过，不代表 W6.E gate 关闭。
- 真实材料必须在后续 `just wave-4-external-dependencies-record --from-env --json` 下才会落盘。

### 材料入口完成标准

W6.E 材料入口完成，不等于真实 evidence 完成。入口完成标准是：

1. `just wave-4-external-dependencies-record --export-template` 能输出完整 `WAVE_4_EXTERNAL_*` 变量清单。
2. 对接方已给出正式接口文档、鉴权方式、错误码、频率限制、Vault 凭证引用、成功回执、失败重试日志和 `audit_event` 查询引用。
3. `just wave-4-external-dependencies-readiness --from-env --json` 输出 `ok=true`，并在 `evidence_items` / `evidence_scope` / `proof` / `validation` 中列出每项材料、负责人和校验结果。
4. `just wave-4-external-dependencies-record --from-env --check-only --json` 输出 `ok=true`，且 `writes_runtime_evidence=false`、`closes_gate=false`。
5. 材料引用不能指向 WMS 自己的追溯码查询接口、`/api/codes`、`wms-api` OpenAPI 或本仓库内部契约；这些只能证明 WMS 对外查询能力，不能替代“码上放心”真实 dev/staging 平台 evidence。

## Evidence JSON

真实 evidence 写盘目标为 `docs/retros/wave-4-external-dependencies.json`，并通过 `just wave-4-external-dependencies-validate`：
以下 JSON 仅为字段结构示例，不得复制为真实 evidence；真实 evidence 必须由 record 命令生成。

```json
{
  "environment": "staging",
  "platform": "码上放心",
  "api_doc_ref": "s3://wms-staging-evidence/wave4/traceability/api-doc-20260603.pdf",
  "auth_doc_ref": "s3://wms-staging-evidence/wave4/traceability/auth-20260603.md",
  "error_code_doc_ref": "s3://wms-staging-evidence/wave4/traceability/error-codes-20260603.md",
  "rate_limit_doc_ref": "s3://wms-staging-evidence/wave4/traceability/rate-limit-20260603.md",
  "credential_ref": "vault://wms/staging/traceability/masxf",
  "success_report_log_ref": "ci/staging/wave4-traceability-success/123",
  "failure_retry_log_ref": "ci/staging/wave4-traceability-retry/123",
  "audit_event_query_ref": "ci/staging/wave4-traceability-audit/123",
  "reported_events": 1,
  "failed_events_exercised": 1,
  "pending_replay_queue_verified": true
}
```

按 `docs/domain/clarifications.md` #50，Wave 4 开发完成不再等待本文件对应的真实外部 evidence；但这里的证据仍是后续试运行/预发布前必须单独关闭的外部依赖 gate。不得用公开网页、本地测试、localhost、127.0.0.1、0.0.0.0、local、prod、production、mock、fake、stub 或 example 数据伪造 evidence。

拿到真实 dev/staging 资料和回执后，先填充 `WAVE_4_EXTERNAL_*` 变量：`$WAVE_4_EXTERNAL_API_DOC_REF`、`$WAVE_4_EXTERNAL_AUTH_DOC_REF`、`$WAVE_4_EXTERNAL_ERROR_CODE_DOC_REF`、`$WAVE_4_EXTERNAL_RATE_LIMIT_DOC_REF`、`$WAVE_4_EXTERNAL_CREDENTIAL_REF`、`$WAVE_4_EXTERNAL_SUCCESS_REPORT_LOG_REF`、`$WAVE_4_EXTERNAL_FAILURE_RETRY_LOG_REF`、`$WAVE_4_EXTERNAL_AUDIT_EVENT_QUERY_REF` 等。再用 readiness 命令只读检查材料边界；readiness 不会写入 `docs/retros/wave-4-external-dependencies.json`，也不能关闭 W6.E gate。检查通过后，再用 record 命令追加 `--check-only` 做 recorder 级预检；该模式只复用正式 validator 校验证据字段和引用边界，不写 evidence，不能关闭 W6.E gate。最后移除 `--check-only` 生成正式 evidence。脚本会在写文件前复用 `validate_wave4_external_dependencies.py` 的正式校验规则：

所有接口文档、鉴权、错误码、频率限制、Vault、上报、重试和审计证据引用必须包含当前 `environment` 标记（`dev` 或 `staging`），并且不能指向 local / prod / production / mock / fake / stub / example。

```bash
just wave-4-external-dependencies-readiness --from-env --json
just wave-4-external-dependencies-record --from-env --check-only --json
just wave-4-external-dependencies-record --from-env --json
```

写入真实 evidence 后执行外部依赖证据检查：

```bash
just wave-4-external-dependencies-validate
```

## 完成通知

Wave 4 完成通知必须走门禁目标，不能在 Codex hook 或 shell hook 中直接 `curl` webhook：

```bash
export WAVE4_COMPLETION_WEBHOOK_URL="$WMS_STAGING_QYWX_WEBHOOK_URL"
just wave-4-notify-if-complete
```

`just wave-4-notify-if-complete` 会先执行 `wave-4-complete-check` 等价的严格检查：

- 检查失败：输出当前阻塞项，不发送 webhook，退出码为 0，方便放入 Codex 结束类 hook 反复调用。
- 检查通过：从 `WAVE4_COMPLETION_WEBHOOK_URL` 读取 webhook URL 并发送企业微信文本通知。
- 仓库内禁止写入 webhook key；如需长期使用，放在本机 `.env` 或运行环境 secrets 中。

## 拒绝边界

- `environment` 是 `local` / `prod` / `production`
- 任一引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`prod`、`production`、`mock`、`fake`、`stub`、`example`
- 任一引用保留模板占位，如 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认`
- 只提供 WMS 内部契约或单元测试，没有真实平台测试环境回执
- 只提供平台账号开通，没有接口文档 / 鉴权 / 错误码 / 频率限制
- 证据中包含密钥、token、私钥或可直接使用的明文凭证
- 成功路径存在，但失败 / 重试 / 待补报路径没有证据

## 完成判定

`just wave-4-complete-check` 按 clarifications #50 允许 W4.D 真实外部 evidence 延期，因此 Wave 4 开发完成不等同于“码上放心”真实 dev/staging 联调完成。外部 evidence 的完成判定仍以 `just wave-4-external-dependencies-validate` 为准。
