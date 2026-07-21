#!/usr/bin/env bash
# H8-004 软件路径：证明探查账号可以 SELECT 且不能 INSERT/UPDATE/DELETE。
set -euo pipefail

CONTAINER="${H8_MSSQL_CONTAINER:-wms-mssql-erp-if}"
PROBE_USER="${H8_MSSQL_PROBE_USER:-wms_h8_probe}"
PROBE_PASSWORD="${H8_MSSQL_PROBE_PASSWORD:-Wms_H8_Probe_Dev_2026!}"

sql_probe() {
  docker exec "${CONTAINER}" /opt/mssql-tools18/bin/sqlcmd \
    -S localhost -d wms_erp_if -U "${PROBE_USER}" -P "${PROBE_PASSWORD}" -C -b -Q "$1"
}

echo "==> SELECT probe"
sql_probe "SELECT TOP 1 external_doc_no, sync_status FROM dbo.if_in_asn ORDER BY updated_at DESC" >/dev/null
echo "SELECT allowed"

echo "==> DEMO seed assertions"
sql_probe "IF NOT EXISTS (SELECT 1 FROM dbo.if_in_asn WHERE external_doc_no = N'DEMO-ASN-001' AND sync_status = N'pending') BEGIN RAISERROR (N'DEMO-ASN-001 pending row missing', 16, 1); END"
sql_probe "IF NOT EXISTS (SELECT 1 FROM dbo.if_in_product_master WHERE external_doc_no = N'DEMO-PM-001' AND sync_status = N'pending') BEGIN RAISERROR (N'DEMO-PM-001 pending row missing', 16, 1); END"
echo "DEMO seed rows visible"

for statement in \
  "UPDATE TOP (0) dbo.if_in_asn SET sync_status = sync_status" \
  "DELETE TOP (0) FROM dbo.if_in_asn" \
  "BEGIN TRANSACTION; INSERT INTO dbo.if_in_asn (external_doc_no, owner_id, warehouse_id, supplier_id, product_code, expected_qty, expected_arrival_at, idempotency_key) VALUES (N'PROBE-DENY', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', N'PROBE', 1, SYSUTCDATETIME(), N'PROBE-DENY'); ROLLBACK TRANSACTION"; do
  if sql_probe "$statement" >/dev/null 2>&1; then
    echo "DML unexpectedly allowed: ${statement}" >&2
    exit 1
  fi
  echo "DML denied: ${statement%% *}"
done
