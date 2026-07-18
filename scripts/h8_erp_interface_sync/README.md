# H8 ERP 接口表同步 Worker

独立进程：连接 MSSQL 接口库 → 认领 `pending` → 调用 WMS OpenAPI → 回写状态。

详见 [docs/runbooks/h8-erp-interface-table-sync.md](../../docs/runbooks/h8-erp-interface-table-sync.md)。

```bash
export WMS_API_TOKEN=...
python3 scripts/h8_erp_interface_sync/sync_worker.py --once
```
