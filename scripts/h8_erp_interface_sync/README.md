# H8 ERP 接口表同步 Worker

独立进程，**双向**：

| 方向 | 路径 |
|------|------|
| 入站 | MSSQL `if_in_*` → WMS OpenAPI |
| 出站 | WMS PG `*_erp_feedback_outbox` → MSSQL `if_out_message` |

详见 [docs/runbooks/h8-erp-interface-table-sync.md](../../docs/runbooks/h8-erp-interface-table-sync.md)。

```bash
export WMS_API_TOKEN=...
export WMS_DB_URL=postgres://...
python3 scripts/h8_erp_interface_sync/sync_worker.py --once              # both
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out
```
