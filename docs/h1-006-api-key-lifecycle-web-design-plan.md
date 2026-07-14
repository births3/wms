# H1-006 API Key 管理端页面设计

页面族为列表型管理页，复用公共 `QueryPanel`、`DataGrid` 和 `Dialog`。

## 页面分区

- 查询区：调用方/用途关键字、状态；默认展示核心条件。
- 列表区：调用方、用途、作用域、状态、过期时间、旧 Key 宽限时间、更新时间。
- 动作区：创建 Key、轮换、吊销、刷新；创建和轮换使用弹窗，明文只在成功反馈中展示一次。
- 状态区：加载、空数据、错误、写入失败和吊销确认均需要可见反馈。

## 字段 RTM

创建字段为 `caller_name`、`purpose`、`warehouse_ids`、`scopes`、`expires_at` 和负责人；`owner_id` 从服务端登录上下文派生。列表不返回明文 secret。

| 页面 | 字段 | 需求来源 | 契约 |
|---|---|---|---|
| H1 API Key 管理 | `caller_name`、`purpose`、`warehouse_ids`、`scopes`、`expires_at`、负责人 | US-H1-006 | `CreateApiKeyRequest`，owner 由服务端派生 |
| H1 API Key 管理 | `key_id`、`status`、`expires_at`、`grace_expires_at`、`revoked_at`、`created_at`、`updated_at` | US-H1-006 | `ApiKey`，列表不返回 secret |
| 创建/轮换弹窗 | secret 一次性提示 | US-H1-006 | 创建或轮换首次响应；幂等重放不返回 |

接口为：

- `GET /api/v1/auth/api-keys`
- `POST /api/v1/auth/api-keys`
- `POST /api/v1/auth/api-keys/{api_key_id}/rotate`
- `POST /api/v1/auth/api-keys/{api_key_id}/revoke`

## 动作 RTM

| 动作 | 需求来源 | 前端入口 | API / 契约 | 当前结论 |
|---|---|---|---|---|
| 创建 | US-H1-006 | DataGrid「创建 Key」→创建弹窗 | `POST /api/v1/auth/api-keys` + `Idempotency-Key` | 已接入 |
| 轮换 | US-H1-006 | 选择一行→「轮换」→轮换弹窗 | `POST /api/v1/auth/api-keys/{api_key_id}/rotate` + `Idempotency-Key` | 已接入 |
| 吊销 | US-H1-006 | 选择一行→「吊销」→确认 | `POST /api/v1/auth/api-keys/{api_key_id}/revoke` + `Idempotency-Key` | 已接入 |
| 查询/刷新 | US-H1-006 | QueryPanel / DataGrid「刷新」 | `GET /api/v1/auth/api-keys` | 已接入 |

## 状态 RTM

| 状态流转 | 需求来源 | 触发动作 | 当前结论 |
|---|---|---|---|
| `active` | US-H1-006 | 创建或有效调用 | 显示启用 |
| `active` + `grace_expires_at` | US-H1-006 | 轮换旧 Key | 显示轮换宽限 |
| `temporarily_disabled` | US-H1-006 | 鉴权失败或限流阈值 | 显示临时禁用 |
| `revoked` | US-H1-006 | 吊销 | 显示已吊销并禁用动作 |

## 证据 RTM

| 证据对象 | 需求来源 | 真实截图 | 动作验证 | 当前结论 |
|---|---|---|---|---|
| 管理端入口 | US-H1-006 | 待真实 9002 环境 | 菜单、路由和自检 | 已通过静态门禁 |
| 后端生命周期 | US-H1-006 | 不适用 | PostgreSQL 测试 3/3 | 已通过 |
| OpenAPI / api-client | US-H1-006 | 不适用 | `just openapi-sync`、`just openapi-check` | 已通过 |
| 浏览器真实数据与截图 | US-H1-006 | H1 API Key 定向 real E2E 截图 | 创建、轮换、吊销 | 已通过；截图归档于 `artifacts/screenshot-portal/real-web/h1-api-key/api-key-lifecycle.png` |

## 证据状态

页面入口、菜单登记、静态自检、TypeScript 类型检查、后端 PostgreSQL 测试、统一外部鉴权、调用审计、30 天 H4 到期提醒和 9002 浏览器截图均已完成。
