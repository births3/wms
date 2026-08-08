# Runbook：H8 ERP 防腐层双通道（本地）

> 对应：`docs/infra/technical-specs.md` H8
> **通道 B**：接口表 + Worker
> **通道 A**：WMS OpenAPI 入站 + ERP HTTP 回调出站
>
> “双通道”表示本地可分别验证两种通道，不表示生产双写；生产按 US-H8-001 选择唯一
> 路由，主备切换沿用同一 Idempotency-Key。

> **v1.9 提醒**：`deploy/docker-compose.h8-erp-if.yml` 与下方 2026-07 历史证据仍是旧
> `if_*` 软件资产，不能验证当前 Rust Worker 或 US-H8-004。v1.9 联调使用 ERP 提供的
> `x_wmsinter_*` 版本化 DDL（当前为 `zbpf7_test`），不得把两套表混跑。

## 1. 启动接口库（通道 B）与容器 ERP 厂商（通道 A）

```bash
cd deploy
# 通道 B：MSSQL 接口表
docker compose -f docker-compose.h8-erp-if.yml up -d
./h8-erp-if/wait-and-init.sh

# 通道 A：容器化外部 ERP 厂商（独立进程/端口/回执存储）
export H8_ERP_VENDOR_BEARER_TOKEN='replace-with-local-test-token'
docker compose -f docker-compose.h8-erp-vendor.yml up -d --build
export ERP_CALLBACK_BASE=http://127.0.0.1:18092
curl -fsS \
  -H "Authorization: Bearer $H8_ERP_VENDOR_BEARER_TOKEN" \
  "$ERP_CALLBACK_BASE/healthz"
```

首个正式版本前只维护当前 MSSQL schema 基线。若本地持久卷来自旧 migration，
必须按 ADR-0038 删除并重建该开发卷后再初始化，不能把初始化脚本当作旧结构升级器。

应用：`01` 入站三表、`03` 出站+销退、`04` 商品变更回写。
端口 **14333**，SA 默认 `Wms_Erp_If_Dev_2026!`（仅本地）。
ERP 厂商容器端口 **18092**，厂商标识 `container-erp-vendor-a`；回执 `GET /receipts/{source_outbox_id}`。

## 2. WMS 与环境变量

```bash
export WMS_API_BASE=http://127.0.0.1:18090
export WMS_API_TOKEN='...'   # login 需 owner_code
export WMS_DB_URL='postgres://...'   # 出站读 outbox
export H8_CONNECTOR_ID='...' # 当前接口库对应的 H8 连接 UUID；一个 Worker 只绑定一个连接
export WMS_H8_SECRET_ALIASES='{"vault://h8/worker-db":"本机注入的接口库密码"}'
# 可选：H8_WORKER_ID（默认主机名+PID）/ H8_WORKER_VERSION / H8_HEARTBEAT_TTL_SEC

# API 对 REST 连接测试采用精确 endpoint 白名单；必须写主机/IP + 端口
export WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS='erp.example.internal:443,10.20.30.40:443,[fd00::10]:443'
# 仅本机开发需要；默认/生产不得设置
export WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP='true'

# API 侧完整报文短期保留；生产必须由 Secret 管理器注入，不写入仓库或日志
export WMS_ENCRYPTION_MASTER_KEY='至少 32 字节的主密钥'
export WMS_ENCRYPTION_KEY_VERSION='v2' # 可选，默认 v1
# 轮换期按版本提供尚未到期的旧密钥；值由 Secret 管理器注入，禁止写入文件或日志
export WMS_ENCRYPTION_PREVIOUS_MASTER_KEYS='{"v1":"至少 32 字节的旧主密钥"}'

# 出站生产路由来自 H8 ERP 连接配置：channel_mode + api_base_url。
# Worker 不接受 --transport，也不读取全局首条 active 连接；每条 outbox 按
# 货主、可用 warehouse_id、outbound 和 message_type 调 route-resolve。
# 历史 rest_primary_table_fallback 说明见旧 if_* 证据；当前 Rust Worker
# 仅接受 interface_table，避免与 v1.9 x_wmsinter_* 通道混用。
```

非本机 REST 连接测试只允许 HTTPS，目标 `host:port` 必须精确出现在
`WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS`；HTTPS URL 未写端口时按 `443` 匹配。只写主机、
通配符、同主机不同端口均拒绝；IPv6 必须写成 `[address]:port`。明确列入的企业内网 IP
可以使用；未指定、回环、链路本地、组播和非规范 IP 表示一律拒绝。本机开发仅例外允许
`http://localhost` / `http://127.0.0.1`，仍须把实际端口（HTTP URL 未写时为 `80`）
加入 endpoint 白名单，并显式设置 `WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP=true`。该开关
默认关闭，生产不得启用。白名单中的 DNS 名由部署运维负责保证解析目标可信；连接探测
不接受 URL 用户信息、查询串、重定向，也不读取本机 `.curlrc`。

权限：`mpm.execute`、`m2.write`、`m4.write`、商品写和 `h8.erp_connector.write`；迁移 `202607230002_mpm_persistent_mapping.sql` 会登记 `mpm.execute`，缺权限时重登刷新 JWT。
扩展 outbox：`202607180005_h8_erp_outbox_extensions.sql`。

### 2.1 H8 接口表探查凭据

探查 API 不得回退使用 Worker 写账号。SQL Server DBA 需创建独立登录并加入已授予 16 张
v1.9 接口表 SELECT、显式拒绝 DML 的 `r_wms_probe_readonly` 角色：

```sql
CREATE LOGIN wms_probe_test WITH PASSWORD = '<由 Secret 管理器生成的独立密码>', CHECK_POLICY = ON;
USE zbpf7_test;
CREATE USER wms_probe_test FOR LOGIN wms_probe_test;
EXEC sp_addrolemember N'r_wms_probe_readonly', N'wms_probe_test';
```

随后在 H8 连接中配置 `interface_probe_db_username=wms_probe_test` 和独立 probe secret alias，
并把该 alias 注入 **WMS API** 的 `WMS_H8_SECRET_ALIASES`。只注入 Worker alias、连接仍引用
无法解析的旧 probe alias 时，页面会返回“接口表读取失败”；禁止用 Worker 密码补齐 probe
alias。API 还必须配置当前单货主接口码 `H8_OWNER_CODE=ZBPF7`。

## 3. 入站类型（通道 B 接口表 → WMS API）

| type | 表 | API |
|------|-----|-----|
| product_master | `x_wmsinter_GoodsInfo` | H8 `product_master` |
| customer_master | `x_wmsinter_CustomerInfo` | H8 `customer_master` |
| supplier_master | `x_wmsinter_SupplierInfo` | H8 `supplier_master` |
| asn | `x_wmsinter_InboundOrder` + `InboundOrderItems` | H8 `asn` |
| outbound_order | `x_wmsinter_OutboundOrder` + `OutboundOrderItems` | H8 `outbound_order` |
| order_cancel | `x_wmsinter_OrderCommand` | H8 `order_cancel` |
| inventory_seed_snapshot | `x_wmsinter_InventoryPushHeader` + `InventoryPushItems` | H8 `inventory_seed_snapshot` |

`product_master` 的储存条件、特殊药品分类和全部包装单位必须由 ERP 发布器在 INSERT 前完成受控映射；未命中数据留在 ERP 发布失败队列，不得进入接口表。WMS 共享 H8 入口仍执行二次校验：违规行返回 422，H8 消息置死信且 M1 零写入；Rust Worker 将该接口行置 `handelflag=4`，不得生成 `pending_mapping` 商品。

档案补录不再使用旧 `if_in_product_change`：WMS 通过 `x_wmsinter_WmsEvent`
`archive_revision` 上报，ERP 修正后以原 `CorrelationID` 发布新版 `GoodsInfo`。

## 4. 出站类型（WMS outbox → B 或 A）

| outbox 表 | event_type | 通道 B 目标 | 回调 path（通道 A） |
|-----------|------------|-------------|---------------------|
| receiving_putaway_erp_feedback_outbox | inbound_putaway_completed / order_status | `InboundFeedback` / `OrderFeedback` | /inbound-complete |
| inventory_status_erp_feedback_outbox | inventory_status_changed | `WmsEvent` | /inventory-status |
| stock_adjustment_erp_feedback_outbox | stock_loss/surplus_completed | `WmsEvent` | /stock-adjustment |
| archive_revision_erp_feedback_outbox | archive_revision | `WmsEvent` | /archive-revision |
| reconciliation_erp_feedback_outbox | reconciliation_diff | `WmsEvent` | /reconciliation-diff |
| shipment_confirm_erp_feedback_outbox | shipment_confirm | `OutboundFeedback` + `OrderFeedback` | /shipment-confirm |
| inventory_snapshot_erp_feedback_outbox | inventory_snapshot | `InventoryReceiveHeader` + `InventoryReceiveItems` | /inventory-snapshot |

档案补录 outbox：`max_attempts=5`、失败退避 5 分钟、`deadline_at` 默认 24h，超时/超次 → `dead`。

通道 B 发布主记录时置 `handelflag=0`；ERP 业务事务提交后置 5。Rust Worker 只根据对应
`x_wmsinter_*` 业务回执确认出站，不再读取或写入旧 `if_out_message`。

通道 A 的 ERP 业务回执调用
`POST /api/v1/integration/erp-messages/{message_id}/receipt`，使用具备
`inbound:push` scope 的 `X-WMS-API-Key`，并携带与原出站消息一致的
`Idempotency-Key`、`schema_version` 和 `correlation_id`。`result=ok` 进入
`acked` 且不得携带 `error_summary`；`result=rejected` 必须带 `error_summary`
并进入 `dead`。生成的 curl 示例必须保留 API Key 与幂等头，禁止误写为 JWT Bearer。

技术送达后 H8 按 ADR-0018 L2 设置 `next_retry_at`。Worker 每轮先处理已到期回执：
服务端把同一消息推进为重试或第 5 次超时 `dead`；可重试时只把原幂等键指向的
原 outbox 从 `succeeded` 恢复为 `failed`，随后由既有出站管线重新发送，禁止复制消息。

## 5. Worker

Worker 启动后向「H8 ERP 消息 / Worker 状态」上报实例、版本、方向、当前认领数和心跳。
启动时只从环境读取 WMS 控制面地址、令牌和 `H8_CONNECTOR_ID`，随后先读取当前连接，
再读取 `/api/v1/config/erp-connectors/{id}/versions/{config_version}` 不可变快照。
接口库主机、端口、库名、用户名和密码 alias 全部来自该快照；密码仅通过
`WMS_H8_SECRET_ALIASES` / `WMS_SECRETS_MAP` 解析。生产 Worker 禁止使用
`H8_MSSQL_*`、Docker 容器名或全局 MSSQL 默认值覆盖连接配置；历史版本的在途消息按其
既有 `connector_config_version` 排空，新版本由新 Worker 实例承接。

Rust Worker 使用 Tiberius 连接快照中的 `host:port`，无需安装 `sqlcmd`，也不执行
`docker exec`。本运行手册中的 Docker、SA 密码和初始化变量仅用于开发接口库建库/证据
准备，不是生产 Worker 的传输配置。

每批认领前读取当前连接 + 方向的暂停控制；暂停时不触碰 MSSQL 待处理行，在途批次继续完成。
恢复或暂停到期后继续认领。心跳失败只告警，不中断已经在途的业务处理。

完整报文默认只计算摘要。管理员在 Worker 状态页按连接启用后，API 使用 PostgreSQL
`pgcrypto` 加密保存 1–30 天（默认 7 天）；查看明文需要写权限并产生 H2 审计。API 每小时
清除到期密文，保留消息、尝试、摘要和审计。部署迁移账号必须有创建 `pgcrypto` 扩展的权限；
密钥缺失或错误时禁止启用/解密，不得把数据库加密错误返回前端。

```bash
# 单轮联调
cargo run --manifest-path backend/Cargo.toml -p h8-erp-worker -- --once

# staging 常驻；restart=unless-stopped，前端「H8 ERP 消息 / Worker 状态」可查看心跳
docker compose --env-file deploy/env/staging.env \
  -f deploy/docker-compose.staging.yml up -d --build h8-erp-worker-staging

# 容器状态与日志
docker compose --env-file deploy/env/staging.env \
  -f deploy/docker-compose.staging.yml ps h8-erp-worker-staging
docker compose --env-file deploy/env/staging.env \
  -f deploy/docker-compose.staging.yml logs --tail=100 h8-erp-worker-staging
```

常驻服务只实现冻结的 `interface_table` 路由；连接切换为 REST 或
`rest_primary_table_fallback` 时会受控拒绝，不会静默双写。Python 脚本保留为历史验收资产，
不得与 Rust Worker 同时运行和竞争 outbox。

### REST 主动对账拉取（不属于 v1.9 接口表 Worker）

以下能力不在冻结的 v1.9 纯接口表通道中，Rust 常驻 Worker 不执行。需要启用时应单独治理并
实现 Rust 调度器，禁止重新启动历史 Python Worker 与接口表 Worker 并行竞争。

主动对账调度器应先以本轮唯一 Idempotency-Key 调用 `POST /api/v1/reconciliation/claims`。
`WMS_API_TOKEN` 必须来自持有 service-only `rc.reconciliation.ingest` 的货主级服务账号；
仓库主管的 `rc.reconciliation.execute` 不能认领窗口、续租、上报 Worker 失败或提交快照。
服务端只认领当前 JWT 货主，并要求存在 active、outbound、owner-wide
（`warehouse_ids=[]`）的 `inventory_snapshot` REST 路由；仓级路由不得与货主全量库存混算，
未到配置周期或已有 active claim 时返回 `claim=null`。

claim 返回 `id`、`claim_token`、deterministic `window_key`、`attempt_no` 和有限
`lease_expires_at`。Worker 每个 ERP 分页前及最终提交前调用
`POST /api/v1/reconciliation/claims/{id}/renew`，租约只允许 30–900 秒；本实现使用 120 秒，
等于单次 ERP 请求超时边界的两倍。租约过期后其他 Worker 可认领相同窗口的新 attempt，旧
token 不再允许续租或提交，从而避免多 Worker 同时拉取同一窗口。

持有租约后 Worker 调用自研 ERP
`GET {api_base_url}/inventory-snapshots?owner_id=...&cursor=...`，所有分页必须保持同一
`snapshot_at`，再携带 claim id/token 一次提交 `POST /api/v1/reconciliation/runs`。
对账批次成功与 claim completed 在同一事务完成；同窗口同请求重放返回原结果，不同请求返回
幂等冲突。ERP 拉取或 WMS 提交失败时，Worker 调用
`POST /api/v1/reconciliation/claims/{id}/failed`，仅上报受控的 `pull|submit` 阶段和稳定
错误码，不上传外部响应正文：`pull` 只能配 `erp_pull_failed`，`submit` 只能配
`snapshot_submit_failed`；`lease + lease_expired` 仅由服务端租约接管流程写入。
数据库同时约束阶段/错误码配对及 active/completed/failed/expired 状态形态，绕过 API 的非法
组合也会失败。服务端追加 H2 审计并创建 H4 运维通知。失败上报自身不可用时，
Worker 保留并抛出原始异常，租约到期后由下一 Worker 接管。
连接配置必须存在 `bearer_secret_alias`，Worker 从 `WMS_H8_SECRET_ALIASES` /
`WMS_SECRETS_MAP` 解析该货主令牌；所有环境都禁止跨货主复用 `ERP_API_TOKEN`
等全局令牌。开发与测试环境也必须为连接填写显式 alias，并在本地 secret map
中提供对应值；alias 缺失或无法解析时按受控 503 保留消息等待重试。

H-SCH 不持有跨货主超级令牌，也不自行计算周期；每个货主连接实例使用自己的
`WMS_API_TOKEN`。建议调度器每 5 分钟调用一次，实际执行频率由
`reconciliation_rules.interval_hours` 控制。

以 WMS 为准生成的 `reconciliation_erp_feedback_outbox` 最多自动尝试 5 次；耗尽后标记
dead 并创建 H4 运维通知，不再无限重试。

单测：`cargo test --manifest-path backend/Cargo.toml -p h8-erp-worker`

### v1.9 入站闭环样本

在 `zbpf7_test` 以头明细同事务发布一张可验收单据。启动 WMS API 并取得令牌后执行：

```bash
export H8_BATCH_SIZE=1
cargo run --manifest-path backend/Cargo.toml -p h8-erp-worker -- --once
```

验收必须同时确认：头表按 0→2→1→5 推进，H8 持久接收后产生
`OrderFeedback(FeedbackType=1)`，业务对象只创建一次；明细只读且不被独立认领。非法载荷
按 v1.9 错误码进入 3/4，不得人工修改业务载荷。旧 `DEMO-ASN-*`、`success` 状态和
`wms_resource_id` 证据只属于历史 `if_*` 切片。

## 6. 验收

| ID | 检查 |
|----|------|
| B 入站 | 7 类 v1.9 主记录按 0→2→1/5，子记录不独立认领 |
| B 出站 | outbox → 对应 `x_wmsinter_*` 主记录 0 → ERP 业务提交 5 |
| 完成屏障 | 明细先发布并处理为 5，最后发布 `OrderFeedback(2/6)` |
| 只读探查 | 独立 probe 账号可 SELECT，DML 全拒绝 |

当前 Rust Worker 的最小回归：

```bash
cargo test --manifest-path backend/Cargo.toml -p h8-erp-worker
```

旧 `run_failover_l11_evidence.py` 与 `h8-failover-l11-evidence.json` 只证明历史 `if_*`
V2，不替代 v1.9 实库或客户正式 ERP S4。

### 历史本机记录（2026-07-18）

- 通道 A：archive_revision / reconciliation_diff / shipment_confirm → HTTP mock succeeded
- 通道 B：inventory_snapshot → if_out；ack_if_out → acked
- 入站 product_change：manufacturer 回写 `H8药厂-已补录`
- M-RC：自研 ERP 库存 HTTP 契约、到期窗口、真实 PostgreSQL 对账和浏览器证据已通过

## 7. 仍非本切片

- 真实 ERP 与质量矩阵 S4 关闭
- 内置 `process_*_outbox` 本地直接 succeeded 与 H8 worker **并存时的运维约定**：启用 H8 出站时勿并行跑会抢 outbox 的本地闭环 job
