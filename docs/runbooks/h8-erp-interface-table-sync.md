# Runbook：H8 ERP 防腐层双通道（本地）

> 对应：`docs/infra/technical-specs.md` H8
> **通道 B**：接口表 + Worker
> **通道 A**：WMS OpenAPI 入站 + ERP HTTP 回调出站
>
> “双通道”表示本地可分别验证两种通道，不表示生产双写；生产按 US-H8-001 选择唯一
> 路由，主备切换沿用同一 Idempotency-Key。

## 1. 启动接口库（通道 B）

```bash
cd deploy
docker compose -f docker-compose.h8-erp-if.yml up -d
./h8-erp-if/wait-and-init.sh
```

应用：`01` 入站三表、`03` 出站+销退、`04` 商品变更回写。
端口 **14333**，SA 默认 `Wms_Erp_If_Dev_2026!`（仅本地）。

## 2. WMS 与环境变量

```bash
export WMS_API_BASE=http://127.0.0.1:18090
export WMS_API_TOKEN='...'   # login 需 owner_code
export WMS_DB_URL='postgres://...'   # 出站读 outbox

# 出站传输：生产使用 table | http；both 只用于本地双写联调
export H8_OUTBOUND_TRANSPORT=table
# 通道 A 回调根 URL（transport=http；本地 both 联调也需要）
export ERP_CALLBACK_BASE=http://127.0.0.1:18091
# both 仅本地联调：H8_ALLOW_LOCAL_DUAL_TRANSPORT=1
# 生产 channel_mode 见 US-H8-001（rest_primary_table_fallback=主备降级，非双写）
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

```bash
# 双向 + 接口表出站
python3 scripts/h8_erp_interface_sync/sync_worker.py --once

# 仅通道 A 出站
python3 scripts/h8_erp_interface_sync/channel_a_callback_mock.py --port 18091 &
export ERP_CALLBACK_BASE=http://127.0.0.1:18091
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out --transport http

# 入站指定类型
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction in --types product_change
```

单测：`cd scripts/h8_erp_interface_sync && python3 -m unittest test_h8_sync_worker -v`

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
