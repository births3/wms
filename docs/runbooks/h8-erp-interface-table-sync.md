# Runbook：H8 无 API ERP 接口表同步（本地）

> 对应：`docs/infra/technical-specs.md` H8 双通道 B
> 成功标准：Docker MSSQL 接口库 + 入站（商品/ASN/出库/销退）+ 出站（WMS outbox → `if_out_message`）+ 幂等

## 1. 启动接口库

```bash
cd deploy
docker compose -f docker-compose.h8-erp-if.yml up -d
./h8-erp-if/wait-and-init.sh
```

默认端口 **14333**，SA 密码默认 `Wms_Erp_If_Dev_2026!`（仅本地）。
`wait-and-init` 会应用 `01_schema.sql` + `03_if_out_and_return.sql`。

### 1.1 镜像拉取（daemon 代理故障时）

若 `docker pull mcr.microsoft.com/mssql/server:2022-latest` 失败，可用本机代理 + crane 后 `docker load`。当前用户不在 `docker` 组时需 `sudo docker` 或 PATH 包装。

## 2. 准备 WMS 与令牌

1. 启动 **当前代码** WMS API（PostgreSQL 已迁移）。
2. 货主下具备 warehouse / supplier / customer / product UUID。
3. 登录需 `owner_code`：

```bash
export WMS_API_BASE=http://127.0.0.1:18090
export WMS_API_TOKEN="$(
  curl -s -X POST "$WMS_API_BASE/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d '{"username":"<user>","password":"<pass>","owner_code":"<owner_code>"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["access_token"])'
)"
# 出站需要直连 WMS 库读 outbox
export WMS_DB_URL='postgres://...'
```

权限：`m2.write`、`m4.write`、商品创建相关；缺 `m4.write` 时跑迁移 `202607180004_m4_write_permission.sql` 后重新登录。

## 3. 入站：写入待同步行

```sql
USE wms_erp_if;
-- 商品 / ASN / 出库见 01_schema；销退（必填 batch_no + supplier_id）：
INSERT INTO dbo.if_in_return_order (
  external_doc_no, owner_id, warehouse_id, customer_id, supplier_id, product_code,
  expected_qty, expected_arrival_at, document_type, batch_no, idempotency_key, sync_status
) VALUES (
  N'ERP-RET-1', '<owner>', '<wh>', '<customer>', '<supplier>', N'P-EXIST-001',
  5, SYSUTCDATETIME(), N'sales_return', N'B-ORIG-1', N'h8-ret-1', N'pending'
);
```

`storage_condition` 枚举：`frozen` / `cold` / `cool` / `normal`。
销退 `sales_return` 业务规则：行级必须带原批号；OpenAPI 收货单仍用 `supplier_id` 引用主数据供应商，客户 UUID 记在 `customer_id`。

## 4. 出站：WMS outbox → if_out_message

业务写操作入队 WMS PG：

| outbox 表 | 典型 event_type |
|-----------|-----------------|
| `receiving_putaway_erp_feedback_outbox` | `inbound_putaway_completed` |
| `inventory_status_erp_feedback_outbox` | `inventory_status_changed` |
| `stock_adjustment_erp_feedback_outbox` | `stock_loss_completed` / `stock_surplus_completed` |

Worker 出站：

1. `FOR UPDATE SKIP LOCKED` 认领 `pending|failed`
2. 幂等插入 `if_out_message`（`source_outbox_table` + `source_outbox_id` 唯一）
3. 标记 WMS outbox `succeeded`

ERP 侧作业读取 `if_out_message` 中 `sync_status=pending`，处理完可更新为 `acked`/`success`。

## 5. 启动 Worker

```bash
# 双向一轮
python3 scripts/h8_erp_interface_sync/sync_worker.py --once

# 仅入站 / 仅出站
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction in
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out

# 指定入站类型
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --types asn,return_order
```

单测：`python3 -m unittest scripts.h8_erp_interface_sync.test_h8_sync_worker -v`
（或在包目录：`cd scripts/h8_erp_interface_sync && python3 -m unittest test_h8_sync_worker -v`）

## 6. 验收清单

| ID | 检查 |
|----|------|
| S0 | compose healthy；三+二表存在（in×4 + out×1） |
| S1 入站 | pending→success，WMS 有对应单据/商品 |
| S1 出站 | outbox pending→succeeded，`if_out_message` 有行 |
| 幂等 | 入站 `idempotency_key` 唯一；出站同源 outbox 不重复插 |

### 6.1 本机记录

- 2026-07-18：入站 product/ASN/outbound E2E success
- 补全：`if_out_message`（inventory_status outbox → pending 消息 + WMS outbox succeeded）；`if_in_return_order` → `sales_return` 收货单 success

## 7. 故障

| 现象 | 处理 |
|------|------|
| 入站 403 | JWT 缺 `m4.write` / `m2.write`，补权限后重登 |
| 出站 skip claim | 检查 `WMS_DB_URL`、表是否迁移、`psql` 在 PATH |
| 出站重复 | 查 `UQ_if_out_source`；已有行则仅更新 pending/failed 载荷 |
| 销退 400 | `sales_return` 与主数据 UUID 未对齐 |

## 8. 仍非本通道范围

- 通道 A REST/回调 URL 联调证据
- 档案补录 5min/24h 专用重试策略的产线告警
- 对账差异 M-RC 独立接口表（可后续加 event_type 或新表）
- 真实 ERP 实例与 S4 关闭质量矩阵故事
