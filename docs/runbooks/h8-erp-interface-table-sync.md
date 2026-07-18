# Runbook：H8 无 API ERP 接口表同步（本地）

> 对应：`docs/infra/technical-specs.md` H8 双通道 B  
> 成功标准：Docker MSSQL 接口库 + 三类入站（商品/ASN/出库）可认领并调 WMS API + 状态回写 + 幂等键

## 1. 启动接口库

```bash
cd deploy
docker compose -f docker-compose.h8-erp-if.yml up -d
./h8-erp-if/wait-and-init.sh
```

默认端口 **14333**，SA 密码默认 `Wms_Erp_If_Dev_2026!`（仅本地）。

可选应用占位种子（UUID 需改成真实值才有业务意义）：

```bash
H8_APPLY_SEED=1 ./h8-erp-if/wait-and-init.sh
```

## 2. 准备 WMS 与令牌

1. 启动 WMS API（PostgreSQL 已迁移）。
2. 准备货主下真实的：`owner` 上下文由 JWT 决定；接口表中的 `warehouse_id` / `supplier_id` / `customer_id` 必须是该货主下已有 UUID。
3. 导出 Bearer：

```bash
export WMS_API_BASE=http://127.0.0.1:8080
export WMS_API_TOKEN='<access_token>'
```

权限建议含：`m2.write` 或 ASN 创建所需、`m4.write`、`m1.write` / 商品创建权限。

## 3. 写入待同步行

用 SSMS / sqlcmd 向 `wms_erp_if` 插入 `sync_status=pending` 行，例如：

```sql
USE wms_erp_if;
INSERT INTO dbo.if_in_product_master (
  external_doc_no, owner_id, product_code, product_name, storage_condition,
  idempotency_key, sync_status
) VALUES (
  N'ERP-PM-1', '<owner_uuid>', N'P-H8-001', N'H8演示商品', N'normal',
  N'h8-pm-1', N'pending'
);
```

ASN / 出库表字段见 `deploy/h8-erp-if/init/01_schema.sql`。

## 4. 启动 Worker

```bash
# 单轮
python3 scripts/h8_erp_interface_sync/sync_worker.py --once

# 常驻
python3 scripts/h8_erp_interface_sync/sync_worker.py

# 只跑 ASN
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --types asn
```

Worker 通过 `docker exec … sqlcmd` 访问容器内 MSSQL（无需本机装 ODBC）。

## 5. 验收清单（S0–S2）

| ID | 检查 |
|----|------|
| S0.1 | `docker compose … up` 容器 healthy |
| S0.2 | `01_schema.sql` 三表存在 |
| S0.4 | worker `--once` 可启动 |
| S1.x | 行状态从 pending→processing→success/failed |
| S2.1 | ASN 同步后 WMS 存在收货单 |
| S2.2 | 出库同步后 WMS 存在出库单 |
| S2.3 | 商品同步后 WMS 存在商品 |
| 幂等 | 同一 `idempotency_key` 再跑不重复建单 |

## 6. 故障

| 现象 | 处理 |
|------|------|
| sqlcmd 失败 | 确认容器名 `wms-mssql-erp-if`、密码、tools18 路径 |
| API 401 | 检查 `WMS_API_TOKEN` |
| API 400 引用不存在 | 接口表 UUID 未对齐 WMS 主数据 |
| 一直 pending | worker 未跑或认领失败看 worker 日志 |

## 7. 非目标

- 不关闭 US-M2-001 等质量矩阵故事  
- 不替代产线真实 ERP 实例（Docker 仅为模拟接口库）  
- 出站 `if_out_*` 本期可选未交付  
