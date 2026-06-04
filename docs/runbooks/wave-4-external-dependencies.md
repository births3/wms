# Wave 4 External Dependency Evidence Runbook

> 用途：关闭 Wave 4 中需要外部系统真实确认的完成证据。当前主要覆盖 W4.D "码上放心"平台上报；内部三元组契约不能替代正式平台适配证据。

## 目标

证明 M-TC 追溯码核销事件可以按正式"码上放心"接口规范上报到真实测试环境，并满足 Wave 4 完成标准：

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

## Evidence JSON

真实证据建议写入 `docs/retros/wave-4-external-dependencies.json`：

```json
{
  "environment": "staging",
  "platform": "码上放心",
  "api_doc_ref": "s3://wms-staging-evidence/wave4/traceability/api-doc-YYYYMMDD.pdf",
  "auth_doc_ref": "s3://wms-staging-evidence/wave4/traceability/auth-YYYYMMDD.md",
  "error_code_doc_ref": "s3://wms-staging-evidence/wave4/traceability/error-codes-YYYYMMDD.md",
  "rate_limit_doc_ref": "s3://wms-staging-evidence/wave4/traceability/rate-limit-YYYYMMDD.md",
  "credential_ref": "vault://wms/staging/traceability/masxf",
  "success_report_log_ref": "ci/staging/wave4-traceability-success/123",
  "failure_retry_log_ref": "ci/staging/wave4-traceability-retry/123",
  "audit_event_query_ref": "ci/staging/wave4-traceability-audit/123",
  "reported_events": 1,
  "failed_events_exercised": 1,
  "pending_replay_queue_verified": true
}
```

按 `docs/domain/clarifications.md` #50，Wave 4 开发完成不再等待本文件对应的真实外部 evidence；但这里的证据仍是后续试运行/预发布前必须单独关闭的外部依赖 gate。不得用公开网页、本地测试、mock、stub 或 fake 数据伪造 evidence。

拿到真实 dev/staging 资料和回执后，用记录脚本生成 evidence。脚本会在写文件前复用 `validate_wave4_external_dependencies.py` 的正式校验规则：

所有接口文档、鉴权、错误码、频率限制、Vault、上报、重试和审计证据引用必须包含当前 `environment` 标记（`dev` 或 `staging`），并且不能指向 local / prod / mock / fake / stub / example。

```bash
just wave-4-external-dependencies-record \
  --environment staging \
  --api-doc-ref '<正式接口文档归档引用>' \
  --auth-doc-ref '<鉴权说明归档引用>' \
  --error-code-doc-ref '<错误码清单归档引用>' \
  --rate-limit-doc-ref '<频率限制说明归档引用>' \
  --credential-ref 'vault://wms/staging/traceability/masxf' \
  --success-report-log-ref '<成功上报日志或 CI 记录>' \
  --failure-retry-log-ref '<失败重试日志或 CI 记录>' \
  --audit-event-query-ref '<audit_event 查询证据>' \
  --reported-events 1 \
  --failed-events-exercised 1 \
  --pending-replay-queue-verified
```

写入真实 evidence 后执行外部依赖证据检查：

```bash
just wave-4-external-dependencies-validate
```

## 完成通知

Wave 4 完成通知必须走门禁目标，不能在 Codex hook 或 shell hook 中直接 `curl` webhook：

```bash
export WAVE4_COMPLETION_WEBHOOK_URL='<企微机器人 webhook URL>'
just wave-4-notify-if-complete
```

`just wave-4-notify-if-complete` 会先执行 `wave-4-complete-check` 等价的严格检查：

- 检查失败：输出当前阻塞项，不发送 webhook，退出码为 0，方便放入 Codex 结束类 hook 反复调用。
- 检查通过：从 `WAVE4_COMPLETION_WEBHOOK_URL` 读取 webhook URL 并发送企业微信文本通知。
- 仓库内禁止写入 webhook key；如需长期使用，放在本机 `.env` 或运行环境 secrets 中。

## 拒绝边界

- `environment` 是 `prod` / `production`
- 任一引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`mock`、`fake`、`stub`、`example`
- 只提供 WMS 内部契约或单元测试，没有真实平台测试环境回执
- 只提供平台账号开通，没有接口文档 / 鉴权 / 错误码 / 频率限制
- 证据中包含密钥、token、私钥或可直接使用的明文凭证
- 成功路径存在，但失败 / 重试 / 待补报路径没有证据

## 完成判定

`just wave-4-complete-check` 按 clarifications #50 允许 W4.D 真实外部 evidence 延期，因此 Wave 4 开发完成不等同于“码上放心”真实 dev/staging 联调完成。外部 evidence 的完成判定仍以 `just wave-4-external-dependencies-validate` 为准。
