# H1-006 API Key 生命周期实现与验收切片

## 已实现范围

- 使用当前登录态的 `owner_id`，校验负责人和仓库范围归属。
- 创建、查询、轮换、吊销 API Key，写操作要求 `Idempotency-Key`。
- 数据库只保存 Key 哈希和生命周期元数据；明文只在首次创建或轮换响应中返回。
- 作用域只允许后端 `API_KEY_SCOPES` 受控清单中的业务 scope，以及 H8 Worker 专用
  `h8:worker`；管理端与后端使用同一受控清单。H8 Worker 还需按入站消息类型叠加
  `inbound:push`、`master-data:write`、`outbound:push`、`return:push`、
  `inventory:seed` 或 `order:command`。
- 失败计数、临时禁用和限流窗口使用 PostgreSQL 行锁事务，并写入 H2 append-only 审计。
- 已在应用入口统一接入 `X-WMS-API-Key`：按外部路由声明作用域，注入货主上下文；配置仓库范围的 Key 必须携带并匹配 `X-WMS-Warehouse-ID`。
- 每次统一鉴权后的调用写入 H2 审计，包含 API Key ID、请求路径、HTTP 方法、响应状态、来源 IP 和 User-Agent；不记录明文 secret。
- API Key 进入到期前 30 天窗口后由定时任务调用 H4 通知服务，使用 Key + 到期日去重，重复扫描不会重复生成通知记录。
- 管理端复用 `QueryPanel`、`DataGrid` 和 `Dialog`，入口为 `基础能力 / H1 权限租户 / H1 API Key 管理`。

## 已有证据

| 类型 | 命令或文件 | 结果 |
|---|---|---|
| 后端编译 | `cargo check --manifest-path backend/Cargo.toml -p wms-api` | 通过 |
| 数据库测试 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --test api_key_postgres` | 3/3 通过 |
| 契约同步 | `just openapi-sync`、`just openapi-check` | 已生成并待全局门禁复验 |
| 统一外部鉴权 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --test api_key_postgres external_api_key_auth_injects_owner_and_audits_request` | 通过；真实路由注入货主并验证路径/IP 审计 |
| 到期通知 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --test api_key_postgres api_key_expiry_reminder_is_deduplicated_through_h4` | 通过；30 天窗口与 H4 去重记录 |
| 前端自检 | `node apps/web-admin/self-checks/h1-api-key-slice-self-check.mjs` | 通过 |
| 前端类型 | `pnpm --dir apps/web-admin exec tsc --noEmit` | 通过 |
| 真实浏览器 E2E | `pnpm --dir prototypes exec playwright test --config=playwright-web-admin-h1-real-config.ts --grep 'H1 API Key 管理'` | 通过；真实 PostgreSQL/Redis 配置、9002 管理端、创建/轮换/吊销截图 |

## 验收结论

H1-006 的代码、数据库、API、管理端、统一外部鉴权、仓库范围、调用审计、到期提醒、PostgreSQL 测试和真实管理端浏览器证据已齐全，已从质量矩阵延期队列移入已验证故事。

同一配置中的 H1 角色权限与登录会话 E2E 也已在带认证的本机 Redis 配置下通过。
