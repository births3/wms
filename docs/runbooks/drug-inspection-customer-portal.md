# 药检单客户平台运行手册

> 范围：US-DI-003 独立客户平台、WMS H2 投影桥、客户查询库、H-FILE 只读附件和导出前缀。
> 架构依据：[ADR-0042](../adr/0042-drug-inspection-customer-portal.md)。
> 接口契约：[`shared/openapi/customer-portal-openapi.yaml`](../../shared/openapi/customer-portal-openapi.yaml)。

## 不可破坏的边界

1. 客户 Web 只访问客户平台 API；客户请求不得访问 WMS API 或 WMS PostgreSQL。
2. WMS 是客户、订单和药检单版本的写入事实源；客户平台仅消费 H2 投影。
3. 客户平台使用独立 PostgreSQL、独立 JWT 密钥和独立客户账号。
4. 客户平台主服务对 `wms-attachments` 只读；导出任务只读授权附件并读写
   `wms-exports/<customer_id>/`。
5. 客户平台或投影失败不得阻塞 WMS 入库、库存、发货和药检单确认。
6. **下载通道（首期基线）**：鉴权通过后签发约 15 分钟下载会话，浏览器经
   `GET /api/v1/files/{token}` 由客户平台流式回传；不得配置为经 WMS 代下。
   不是对象存储预签名直出；`PORTAL_H_FILE_STORAGE_ROOT` 为只读存储根（测试本地盘 /
   生产 H-FILE 映射）。

## 配置

### 客户平台服务

| 环境变量 | 必填 | 说明 |
|---|---:|---|
| `PORTAL_DATABASE_URL` | 是 | 独立客户查询库连接串，禁止指向 WMS 库 |
| `PORTAL_JWT_SECRET` | 是 | 至少 32 字节随机密钥，通过密钥管理系统注入 |
| `PORTAL_PROJECTION_KEY` | 是 | WMS 投影入口共享密钥，与 WMS 配置一致 |
| `PORTAL_H_FILE_STORAGE_ROOT` | 是 | 测试实现的文件根；生产映射 H-FILE/Object Storage |
| `PORTAL_BIND` | 否 | 默认 `0.0.0.0:9010` |

### WMS 投影桥

| 环境变量 | 必填 | 说明 |
|---|---:|---|
| `WMS_MDI_PORTAL_URL` | 是 | 客户平台 API 内网地址，不填写时桥接 worker 不启动 |
| `WMS_MDI_PORTAL_PROJECTION_KEY` | 是 | 必须与 `PORTAL_PROJECTION_KEY` 一致 |

部署时不得把 JWT 密钥、投影密钥或数据库口令写入仓库、镜像层、启动日志或审计 diff。

## 部署顺序

1. 创建独立 PostgreSQL 数据库和最小权限账号。
2. 准备 H-FILE 权限：附件前缀只读、`wms-exports` 客户专属前缀读写。
3. 启动客户平台 API；服务启动时自动执行自身 `migrations/`。
4. 检查 `GET /health` 返回 `{"status":"ok"}`。
5. 启动客户 Web，并确认其 API 基址只指向客户平台。
6. 最后向 WMS 注入两个桥接变量并滚动 WMS；不要反向建立客户平台到 WMS 的网络访问。

开发环境启动：

```bash
cd backend
PORTAL_DATABASE_URL='postgres://portal:***@127.0.0.1:5432/wms_customer_portal' \
PORTAL_JWT_SECRET='通过密钥管理注入' \
PORTAL_PROJECTION_KEY='通过密钥管理注入' \
PORTAL_H_FILE_STORAGE_ROOT='../var/customer-portal-files' \
PORTAL_BIND='0.0.0.0:9010' \
cargo run -p wms-customer-portal-api
```

## 发布验证

至少执行以下检查：

1. 客户平台数据库与 WMS 数据库的主机/库名/账号均不同。
2. 以客户管理员、普通多地址账号、无地址账号、历史权限账号分别登录。
3. 投递同一个 `event_id` 两次，第二次返回 `duplicate=true`，查询结果不重复。
4. 已发货/已签收订单可见，其他状态不可见；跨客户或跨地址订单返回 404 或空列表。
5. 单份下载会话约 15 分钟后失效（须重新授权）；导出 ZIP 7 天后不可下载。
6. ZIP 对相同药检单版本去重，清单保留“资料暂缺”行；下载响应来自 portal 流式读盘，
   不是对象存储预签名 URL。
7. 使用真实 E2E 命令并保存截图：

```bash
PORTAL_DATABASE_URL='postgres://portal:***@127.0.0.1:5432/wms_customer_portal_e2e' \
pnpm --dir apps/customer-portal run test:e2e:real
```

## 监控与告警

### WMS H2 投影桥

每 5 分钟统计：

```sql
SELECT status, count(*) AS deliveries, max(attempt_count) AS max_attempts
FROM event_bus_delivery d
JOIN event_bus_subscription s ON s.id = d.subscription_id
WHERE s.subscriber_key = 'mdi-customer-portal'
GROUP BY status;
```

```sql
SELECT d.id, d.event_id, d.attempt_count, d.last_error, dl.created_at
FROM event_bus_delivery d
JOIN event_bus_subscription s ON s.id = d.subscription_id
LEFT JOIN event_bus_dead_letter dl ON dl.delivery_id = d.id
WHERE s.subscriber_key = 'mdi-customer-portal'
  AND d.status = 'dead_letter'
ORDER BY dl.created_at DESC;
```

- 正常目标：投影事件在 30 秒内进入 `delivered`。
- `pending` 最老事件超过 30 秒：告警。
- 连续失败采用 1、2 秒退避，默认第 3 次失败进入 `dead_letter`：立即告警。
- 禁止直接删除 H2 事件、投递或 DLQ 记录。修复密钥、网络或载荷后，由受控 H2 重放能力
  重新生成投递；当前未暴露面向业务用户的 DLQ 修改接口。

### 客户平台

每 5 分钟统计：

```sql
SELECT status, count(*), min(created_at) AS oldest
FROM portal_projection_events
GROUP BY status;
```

```sql
SELECT status, count(*), min(created_at) AS oldest
FROM portal_export_jobs
GROUP BY status;
```

- `portal_projection_events.failed/dead_letter` 非零：告警并与 WMS `event_id` 对照。
- `queued/processing` 导出超过 10 分钟：告警。
- `failed` 导出保留错误摘要；日志不得包含文件正文、密码、密钥或完整临时 URL。
- 监控查询库连接数、P95/P99、磁盘、`wms-exports` 容量和 7 天清理结果。

## 常见故障

### 客户看不到刚确认的药检单

1. 在 WMS 查对应 H2 delivery 是否 `pending/dead_letter`。
2. 检查 WMS 与客户平台投影密钥是否一致，以及内网地址/DNS/TLS。
3. 在客户库按 `event_id` 查 `portal_projection_events`。
4. 确认客户副本状态；`queued/processing/failed` 不得降级返回权威原件。
5. 确认订单状态为 `shipped/signed`，并核对稳定 `delivery_address_id`。

### 客户越权或无数据

1. 核对 `portal_users.customer_id`。
2. 普通账号核对 `portal_user_addresses`；无地址范围按设计返回无数据。
3. 核对订单地址 ID，不用地址名称或当前地址档案替代订单快照。
4. 历史版本必须同时满足账号 `can_view_report_history=true`。

### 导出失败

1. 检查是否超过 200 份或 2GB。
2. 检查 `wms-attachments` 只读权限和附件实际存在性。
3. 检查 `wms-exports/<customer_id>/` 写权限和磁盘/对象存储配额。
4. 已成功文件与缺失清单可同包返回；不得因资料暂缺静默丢行。

## 回滚

1. 先撤销 WMS 的 `WMS_MDI_PORTAL_URL`/投影密钥并滚动 WMS，使新事件继续留在 H2，
   不影响仓内作业。
2. 回滚客户 Web 和客户平台服务版本；保留客户查询库、H2 事件、DLQ、审计和导出对象。
3. 不回滚或删除已生成的客户副本、药检单版本、下载审计和地址快照。
4. 服务恢复后重新启用桥接并执行受控事件重放，确认积压回到 30 秒目标内。
