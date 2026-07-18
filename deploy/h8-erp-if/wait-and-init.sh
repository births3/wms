#!/usr/bin/env bash
# 等待 MSSQL 就绪并执行建表脚本。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="${ROOT}/docker-compose.h8-erp-if.yml"
SA_PASSWORD="${H8_MSSQL_SA_PASSWORD:-Wms_Erp_If_Dev_2026!}"
CONTAINER="${H8_MSSQL_CONTAINER:-wms-mssql-erp-if}"

echo "==> waiting for ${CONTAINER}"
for i in $(seq 1 60); do
  if docker exec "${CONTAINER}" /opt/mssql-tools18/bin/sqlcmd \
    -S localhost -U sa -P "${SA_PASSWORD}" -C -Q "SELECT 1" &>/dev/null; then
    echo "==> mssql ready"
    break
  fi
  if [[ "$i" -eq 60 ]]; then
    echo "mssql not ready" >&2
    exit 1
  fi
  sleep 2
done

run_sql() {
  local file="$1"
  echo "==> apply $(basename "$file")"
  docker exec -i "${CONTAINER}" /opt/mssql-tools18/bin/sqlcmd \
    -S localhost -U sa -P "${SA_PASSWORD}" -C -b -i "/docker-init/$(basename "$file")"
}

# init 目录已挂载到容器 /docker-init
run_sql "${ROOT}/h8-erp-if/init/01_schema.sql"
run_sql "${ROOT}/h8-erp-if/init/03_if_out_and_return.sql"
if [[ "${H8_APPLY_SEED:-0}" == "1" ]]; then
  run_sql "${ROOT}/h8-erp-if/init/02_seed_example.sql"
fi

echo "==> H8 ERP interface schema OK"
