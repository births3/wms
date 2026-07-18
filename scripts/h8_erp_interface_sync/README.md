# H8 ERP 防腐层 Worker

| 通道 | 入站 | 出站 |
|------|------|------|
| **B 接口表** | MSSQL `if_in_*` → WMS OpenAPI | WMS outbox → `if_out_message` |
| **A HTTP** | ERP 直调 WMS OpenAPI | WMS outbox → `ERP_CALLBACK_BASE` + path |

```bash
export WMS_API_TOKEN=...
export WMS_DB_URL=postgres://...
# B
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --transport table
# A
python3 scripts/h8_erp_interface_sync/channel_a_callback_mock.py --port 18091 &
export ERP_CALLBACK_BASE=http://127.0.0.1:18091
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out --transport http
# ERP 确认出站
python3 scripts/h8_erp_interface_sync/ack_if_out.py --all
```

详见 [docs/runbooks/h8-erp-interface-table-sync.md](../../docs/runbooks/h8-erp-interface-table-sync.md)。
