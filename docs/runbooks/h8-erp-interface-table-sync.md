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

### 1.1 镜像拉取（daemon 代理故障时）

若 `docker pull mcr.microsoft.com/mssql/server:2022-latest` 因 daemon 代理不可达失败，可用本机可用代理 + crane 拉取后 load：

```bash
# 示例：HTTPS_PROXY=http://127.0.0.1:7894 crane pull mcr.microsoft.com/mssql/server:2022-latest /tmp/mssql.tar
# docker load -i /tmp/mssql.tar
```

当前用户不在 `docker` 组时，`docker exec` / compose 需 `sudo`，或用包装脚本把 `docker` 指到 `sudo docker`。

## 2. 准备 WMS 与令牌

1. 启动 **当前代码** 的 WMS API（PostgreSQL 已 `wms-db-migrate`；旧 staging 镜像可能仅有 healthz、无业务路由）。
2. 货主下必须已有：`warehouse_id` / `supplier_id`（ASN）/ `customer_id`（出库）UUID。
3. 登录需带 `owner_code`：

```bash
export WMS_API_BASE=http://127.0.0.1:18090   # 按实际端口
export WMS_API_TOKEN="$(
  curl -s -X POST "$WMS_API_BASE/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d '{"username":"<user>","password":"<pass>","owner_code":"<owner_code>"}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["access_token"])'
)"
```

权限建议：`m2.write`（ASN）、`m4.write`（出库）、`m1.master_data.*` / 商品创建相关权限。
若库中缺 `m4.write` 权限码，执行迁移 `202607180004_m4_write_permission.sql` 后重新登录刷新 JWT。

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

- `storage_condition` 枚举：`frozen` / `cold` / `cool` / `normal`（worker 映射到商品 `attrs.storage_condition`）。
- ASN 需 `warehouse_id`、`supplier_id`、`product_code`、`expected_qty`、`expected_arrival_at`。
- 出库需 `customer_id`、`warehouse_id`、`product_code`、`planned_qty` 等。
- 同一 `idempotency_key` 在接口表有唯一约束，不可重复插入。

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

Worker 通过 `docker exec … sqlcmd` 访问容器内 MSSQL（无需本机装 ODBC）。认领使用 CTE + `UPDATE`（不可 `UPDATE TOP … ORDER BY`）。

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
| 幂等 | 同一 `idempotency_key` 在接口表唯一；WMS API Idempotency-Key 防重复建单 |

### 5.1 本机 E2E 记录（2026-07-18）

在独立库 `wms_h8_e2e` + 本机 `wms-api:18090` + 容器 `wms-mssql-erp-if` 上验证：

| 类型 | 接口表状态 | WMS 资源 |
|------|------------|----------|
| product_master | success | products.product_code=`P-H8-001` |
| asn | success | receiving_orders.receipt_no=`RCV-H8-001` |
| outbound_order | success | outbound_orders.wms_order_no=`WMS-OB-H8-1` |

修通过程要点：claim SQL 去掉非法 `ORDER BY`；商品 `storage_condition` 写入 `attrs`；补 `m4.write` 权限后出库 403 解除。

## 6. 故障

| 现象 | 处理 |
|------|------|
| sqlcmd 失败 | 确认容器名 `wms-mssql-erp-if`、密码、tools18 路径、`docker` 权限 |
| API 401 | 检查 `WMS_API_TOKEN`、登录是否带 `owner_code` |
| API 403 AUTH-005 | JWT 缺 `m4.write` / `m2.write` 等，补权限后重新登录 |
| API 400 引用不存在 | 接口表 UUID 未对齐 WMS 主数据 |
| 一直 pending 且 processed=0 | 认领 SQL 失败（历史 bug）或 docker 权限；看 worker 是否抛错 |
| mcr 镜像 pull 失败 | 见 §1.1 crane load |

## 7. 非目标

- 不关闭 US-M2-001 等质量矩阵故事  
- 不替代产线真实 ERP 实例（Docker 仅为模拟接口库）  
- 出站 `if_out_*` 本期可选未交付  
