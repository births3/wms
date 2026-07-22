# Runbook：H8 ERP 防腐层双通道（本地）

> 对应：`docs/infra/technical-specs.md` H8
> **通道 B**：接口表 + Worker
> **通道 A**：WMS OpenAPI 入站 + ERP HTTP 回调出站
>
> “双通道”表示本地可分别验证两种通道，不表示生产双写；生产按 US-H8-001 选择唯一
> 路由，主备切换沿用同一 Idempotency-Key。

## 1. 启动接口库（通道 B）与容器 ERP 厂商（通道 A）

```bash
cd deploy
# 通道 B：MSSQL 接口表
docker compose -f docker-compose.h8-erp-if.yml up -d
./h8-erp-if/wait-and-init.sh

# 通道 A：容器化外部 ERP 厂商（独立进程/端口/回执存储）
docker compose -f docker-compose.h8-erp-vendor.yml up -d --build
export ERP_CALLBACK_BASE=http://127.0.0.1:18092
curl -sS "$ERP_CALLBACK_BASE/healthz"
```

应用：`01` 入站三表、`03` 出站+销退、`04` 商品变更回写。
端口 **14333**，SA 默认 `Wms_Erp_If_Dev_2026!`（仅本地）。
ERP 厂商容器端口 **18092**，厂商标识 `container-erp-vendor-a`；回执 `GET /receipts/{source_outbox_id}`。

## 2. WMS 与环境变量

```bash
export WMS_API_BASE=http://127.0.0.1:18090
export WMS_API_TOKEN='...'   # login 需 owner_code
export WMS_DB_URL='postgres://...'   # 出站读 outbox
export H8_CONNECTOR_ID='...' # 当前接口库对应的 H8 连接 UUID；一个 Worker 只绑定一个连接
# 可选：H8_WORKER_ID（默认主机名+PID）/ H8_WORKER_VERSION / H8_HEARTBEAT_TTL_SEC

# API 侧完整报文短期保留；生产必须由 Secret 管理器注入，不写入仓库或日志
export WMS_ENCRYPTION_MASTER_KEY='至少 32 字节的主密钥'
export WMS_ENCRYPTION_KEY_VERSION='v2' # 可选，默认 v1
# 轮换期按版本提供尚未到期的旧密钥；值由 Secret 管理器注入，禁止写入文件或日志
export WMS_ENCRYPTION_PREVIOUS_MASTER_KEYS='{"v1":"至少 32 字节的旧主密钥"}'

# 出站生产路由来自 H8 ERP 连接配置：channel_mode + api_base_url。
# Worker 不接受 --transport，也不读取全局首条 active 连接；每条 outbox 按
# 货主、可用 warehouse_id、outbound 和 message_type 调 route-resolve。
# rest_primary_table_fallback：REST 连续失败（H8_HTTP_MAX_ATTEMPTS，默认 2）后
# 以同一 Idempotency-Key 写入 if_out_message，不双写。
```

权限：`m2.write`、`m4.write`、商品写；缺 `m4.write` 跑 `202607180004_m4_write_permission.sql` 后重登。
扩展 outbox：`202607180005_h8_erp_outbox_extensions.sql`。

## 3. 入站类型（通道 B 接口表 → WMS API）

| type | 表 | API |
|------|-----|-----|
| asn | if_in_asn | POST receiving-orders |
| outbound_order | if_in_outbound_order | POST outbound/orders |
| product_master | if_in_product_master | POST products |
| return_order | if_in_return_order | POST receiving-orders sales_return（必填 batch_no、supplier_id） |
| product_change | if_in_product_change | PATCH products/{id}（档案补录/主数据回写） |

## 4. 出站类型（WMS outbox → B 或 A）

| outbox 表 | event_type | 回调 path（通道 A） |
|-----------|------------|---------------------|
| receiving_putaway_erp_feedback_outbox | inbound_putaway_completed | /inbound-complete |
| inventory_status_erp_feedback_outbox | inventory_status_changed | /inventory-status |
| stock_adjustment_erp_feedback_outbox | stock_loss/surplus_completed | /stock-adjustment |
| archive_revision_erp_feedback_outbox | archive_revision | /archive-revision |
| reconciliation_erp_feedback_outbox | reconciliation_diff | /reconciliation-diff |
| shipment_confirm_erp_feedback_outbox | shipment_confirm | /shipment-confirm |
| inventory_snapshot_erp_feedback_outbox | inventory_snapshot | /inventory-snapshot |

档案补录 outbox：`max_attempts=5`、失败退避 5 分钟、`deadline_at` 默认 24h，超时/超次 → `dead`。

通道 B 写入 `if_out_message` 后，ERP 可确认：

```bash
python3 scripts/h8_erp_interface_sync/ack_if_out.py --all
```

## 5. Worker

Worker 启动后向「H8 ERP 消息 / Worker 状态」上报实例、版本、方向、当前认领数和心跳。
每批认领前读取当前连接 + 方向的暂停控制；暂停时不触碰 MSSQL 待处理行，在途批次继续完成。
恢复或暂停到期后继续认领。心跳失败只告警，不中断已经在途的业务处理。

完整报文默认只计算摘要。管理员在 Worker 状态页按连接启用后，API 使用 PostgreSQL
`pgcrypto` 加密保存 1–30 天（默认 7 天）；查看明文需要写权限并产生 H2 审计。API 每小时
清除到期密文，保留消息、尝试、摘要和审计。部署迁移账号必须有创建 `pgcrypto` 扩展的权限；
密钥缺失或错误时禁止启用/解密，不得把数据库加密错误返回前端。

```bash
# 双向 + 接口表出站
python3 scripts/h8_erp_interface_sync/sync_worker.py --once

# 连接配置为 REST 后仅跑出站
python3 scripts/h8_erp_interface_sync/channel_a_callback_mock.py --port 18091 &
# 在 H8 ERP 连接中配置 api_base_url=http://127.0.0.1:18091
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out

# 入站指定类型
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction in --types product_change
```

单测：`cd scripts/h8_erp_interface_sync && python3 -m unittest test_h8_sync_worker test_exchange_lifecycle test_outbound_routing -v`

### ASN 入站闭环样本

初始化脚本提供独立的 `DEMO-ASN-FLOW-001`，引用 E2E 库中真实存在的货主、仓库、
供应商和商品，不占用 US-H8-004 只读探查固定样本。启动 WMS API 并取得令牌后执行：

```bash
export H8_BATCH_SIZE=1
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction in --types asn
```

验收必须同时确认：接口行由 `pending` 经 Worker 认领后变为 `success`；
`wms_resource_id` 指向唯一的 M2 `receiving_orders`；H2 存在 receive、convert、
business_api、receipt 审计；通过消息重放 API 对同一消息填写原因并确认后，Worker
应自动以原 Idempotency-Key 将既有终态接口行恢复为 `pending`、接管消息并重放，
不得人工修改接口表；M2 单据数量仍为 1 且资源 ID 不变。证据见
`docs/retros/h8-asn-inbound-flow-evidence.json`。

### ASN 死信与人工重放切片

用不存在的供应商引用准备独立 ASN，并设置 `H8_MAX_RETRY=1` 后运行同一 Worker：接口表与
H8 消息必须同时进入 `dead`，H8 留下 `h8_exchange_final_failure`、`h8_message_dead`
和 dead 尝试记录。随后由外部 ERP 更正供应商引用但不修改 `sync_status`，管理员调用既有
重放 API（原因 + 二次确认）；Worker 必须以原 Idempotency-Key 自动把原接口行恢复为
`pending`、认领并完成业务调用。最终必须同时断言：

- H8 使用同一消息 ID、连接 ID、连接编码、配置版本和通道，状态为 `succeeded`；
- MSSQL 原行状态为 `success`，`retry_count` 保留且资源 ID 指向 M2 单据；
- M2 同一 `external_ref` 只有一张收货单；
- H2 具备失败、死信、重放、认领和成功回执的完整关联动作。

定向自动化命令：

```bash
cd scripts/h8_erp_interface_sync
python3 -m unittest test_inbound_terminal_state test_h8_sync_worker test_exchange_lifecycle -v
cargo test --manifest-path ../../backend/Cargo.toml -p wms-api \
  inbound_lifecycle_persists_failure_retry_and_success_status --lib -- --nocapture
```

本地真实运行记录见 `docs/retros/h8-asn-manual-replay-evidence.json`；它只证明 V2 软件切片，
不替代客户 ERP dev/staging 的 V4 故障恢复证据。

## 6. 验收

| ID | 检查 |
|----|------|
| B 入站 | asn/outbound/product/return/product_change → success |
| B 出站 | outbox → if_out_message pending → ack → acked |
| A 出站 | mock `/_dump` 收到 archive/recon/shipment 等 path |
| 档案重试 | 超 max/deadline 变 dead |

### 本机记录（2026-07-18 补全）

- 通道 A：archive_revision / reconciliation_diff / shipment_confirm → HTTP mock succeeded
- 通道 B：inventory_snapshot → if_out；ack_if_out → acked
- 入站 product_change：manufacturer 回写 `H8药厂-已补录`

## 7. 仍非本切片

- 业务写路径**自动入队**档案补录/对账/发货 outbox（需各业务模块接线）
- 真实 ERP 与质量矩阵 S4 关闭
- 内置 `process_*_outbox` 本地直接 succeeded 与 H8 worker **并存时的运维约定**：启用 H8 出站时勿并行跑会抢 outbox 的本地闭环 job
