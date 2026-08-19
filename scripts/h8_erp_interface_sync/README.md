# H8 ERP 防腐层 Worker

| 通道 | 入站 | 出站 |
|------|------|------|
| **B 接口表** | MSSQL `if_in_*` → WMS OpenAPI | WMS outbox → `if_out_message` |
| **A HTTP** | ERP 直调 WMS OpenAPI | WMS outbox → 连接 `api_base_url` + path |

US-H8-001：生产 `channel_mode` 为 `rest` / `interface_table` / `rest_primary_table_fallback`（主备降级，**非**同时双写）。Worker 按每条消息调用 WMS `route-resolve`，以连接配置决定通道和 REST 回调地址；命令行和全局环境变量不能覆盖生产路由。

```bash
export WMS_API_TOKEN=...
export WMS_DB_URL=postgres://...
export H8_CONNECTOR_ID=<当前接口库对应的连接 UUID>
# 完整报文保留密钥配置在 WMS API，不配置在 Worker；详见 runbook
# 连接配置为 B 接口表后运行
python3 scripts/h8_erp_interface_sync/sync_worker.py --once
# 连接配置为 A REST 后运行
python3 scripts/h8_erp_interface_sync/channel_a_callback_mock.py --port 18091 &
# 在 H8 ERP 连接中配置 api_base_url=http://127.0.0.1:18091
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out
# 连接配置为 rest_primary_table_fallback 时，REST 失败后按原键转接口表
# ERP 确认出站
python3 scripts/h8_erp_interface_sync/ack_if_out.py --all
# 主备 + ERP mock 证据
python3 scripts/h8_erp_interface_sync/run_failover_erp_evidence.py

# 容器化外部 ERP 厂商（S4 风格回执证据）
cd deploy && docker compose -f docker-compose.h8-erp-vendor.yml up -d --build
export ERP_CALLBACK_BASE=http://127.0.0.1:18092
python3 scripts/h8_erp_interface_sync/run_container_erp_s4_evidence.py
```

详见 [docs/runbooks/h8-erp-interface-table-sync.md](../../docs/runbooks/h8-erp-interface-table-sync.md)。
