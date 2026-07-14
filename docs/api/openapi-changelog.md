# OpenAPI 变更日志

> 本文件记录 H3 OpenAPI 契约可见变更。Spec 文件仍以 Git diff 和提交记录为最终追踪来源。

## 2026-07-15

- 增加 M-SA 报损单创建、详情、质量审批回写、开始执行和完成执行 API。
- 报损执行契约返回 M-VR 双人策略、命中规则、两名操作人及 H4 审批证据摘要。
- 新增 `stock_loss` 单据类型与 M-CG 默认编号规则，OpenAPI 和 `@wms/api-client` 类型同步生成。

## 2026-07-08

- 增加生产只读文档入口 `GET /redoc`。
- 增加 Prometheus 文本指标入口 `GET /metrics`。
- H3 韧性保护补齐单用户、单 API Key、全局三层令牌桶限流。
- H3 熔断补齐打开、半开、恢复关闭状态。
- H3 限流和熔断事件接入 H2 append-only 审计。
- API 文档补齐对接认证说明和自动生成的 curl 示例索引。

## 变更要求

每次修改 OpenAPI 后必须执行：

```bash
just openapi-sync
python3 scripts/governance/generate_openapi_curl_examples.py
python3 scripts/governance/generate_openapi_curl_examples.py --check
just openapi-check
```
