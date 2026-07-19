# H8 ERP 防腐层 Worker

| 通道 | 入站 | 出站 |
|------|------|------|
| **B 接口表** | MSSQL `if_in_*` → WMS OpenAPI | WMS outbox → `if_out_message` |
| **A HTTP** | ERP 直调 WMS OpenAPI | WMS outbox → `ERP_CALLBACK_BASE` + path |

US-H8-001：生产 `channel_mode` 为 `rest` / `interface_table` / `rest_primary_table_fallback`（主备降级，**非**同时双写）。Worker 的 `--transport both` 仅本地联调，需 `H8_ALLOW_LOCAL_DUAL_TRANSPORT=1`。

```bash
export WMS_API_TOKEN=...
export WMS_DB_URL=postgres://...
# B 接口表
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --transport table
# A REST
python3 scripts/h8_erp_interface_sync/channel_a_callback_mock.py --port 18091 &
export ERP_CALLBACK_BASE=http://127.0.0.1:18091
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out --transport http
# 主备降级：REST 失败后转接口表（同一 outbox id / 幂等键）
export H8_CHANNEL_MODE=rest_primary_table_fallback
# 或 H8_OUTBOUND_TRANSPORT=failover
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out --transport failover
# ERP 确认出站
python3 scripts/h8_erp_interface_sync/ack_if_out.py --all
# 主备 + ERP mock 证据
python3 scripts/h8_erp_interface_sync/run_failover_erp_evidence.py
```

详见 [docs/runbooks/h8-erp-interface-table-sync.md](../../docs/runbooks/h8-erp-interface-table-sync.md)。
