# H3 API 文档与对接说明

## 访问入口

| 环境 | 入口 | 规则 |
|---|---|---|
| 开发 | `/api-docs` | Swagger UI，可使用 Try it out。 |
| 测试 / 预发 | `/api-docs`、`/redoc` | 仅允许内网访问。 |
| 生产 | `/redoc` | 只读 ReDoc；`/api-docs` 返回 404。 |
| 指标 | `/metrics` | Prometheus 文本指标，仅供内网抓取。 |

内网判断优先使用 `X-Forwarded-For`，其次使用 `X-Real-IP`，允许 `10.0.0.0/8`、`172.16.0.0/12`、`192.168.0.0/16`、`127.0.0.0/8` 和 `::1`。开发模式下缺少来源 IP 视为本地直连；生产模式下缺少来源 IP 时 `/redoc` 直接拒绝访问。

## 认证流程

PC / PDA 调用普通业务 API：

1. 调用 `POST /api/v1/auth/login`，提交货主编码、账号和密码。
2. 从响应中读取 `access_token`。
3. 后续请求携带 `Authorization: Bearer <access_token>`。
4. token 被登出、权限变更或过期后，客户端重新登录。

外部系统调用 API Key 接口：

1. 由 H1 API Key 生命周期管理发放 API Key。
2. 外部系统在请求头携带 `X-WMS-API-Key`。
3. 写接口同时携带 `Idempotency-Key`。
4. 运行时按 API Key 维度限流，限流和熔断事件写 H2 审计；服务启动时若设置了 `WMS_H3_API_KEY_AUDIT_OWNER_ID`，审计 actor 使用 `api-key:<hash>`。

## 韧性保护

默认值：

| 项 | 默认值 | 环境变量 |
|---|---:|---|
| 全局 QPS | 1000 | `WMS_API_GLOBAL_QPS` |
| 全局突发容量 | 1000 | `WMS_API_GLOBAL_BURST` |
| 单用户 QPS | 100 | `WMS_API_USER_QPS` |
| 单用户突发容量 | 100 | `WMS_API_USER_BURST` |
| 单 API Key QPS | 100 | `WMS_API_KEY_QPS` |
| 单 API Key 突发容量 | 100 | `WMS_API_KEY_BURST` |
| 熔断失败次数 | 5 | `WMS_API_CIRCUIT_FAILURES` |
| 熔断打开秒数 | 30 | `WMS_API_CIRCUIT_OPEN_SECONDS` |

超限响应：

- HTTP 状态码：`429 Too Many Requests`
- 响应头：`Retry-After`
- 错误码：`H3_RATE_LIMITED`

熔断降级响应：

- HTTP 状态码：`503 Service Unavailable`
- 响应头：`X-WMS-Circuit-State: open`
- 响应头：`X-WMS-Degraded-Response: true`
- 响应体 `details.degraded=true`、`details.data_may_be_stale=true`

Prometheus 指标：

- `wms_h3_rate_limit_rejected_total`
- `wms_h3_circuit_opened_total`
- `wms_h3_degraded_responses_total`

## curl 示例

每个 OpenAPI operation 的 curl 示例见 [OpenAPI curl 示例](curl-examples.md)。
