#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

for command in docker curl python3; do
  command -v "$command" >/dev/null || {
    echo "missing required command: $command" >&2
    exit 2
  }
done

: "${WMS_H9_SMOKE_AUTHORIZATION:?set a staging Authorization header (for example: Bearer <token>)}"
: "${WMS_H9_SMOKE_PRINT_URL:?set the full prepared category-PDF URL for this staging smoke}"

read -r api_port worker_port minio_api_port minio_console_port < <(
  python3 - <<'PY'
import socket

ports = []
for _ in range(4):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    ports.append(str(sock.getsockname()[1]))
    sock.close()
print(" ".join(ports))
PY
)

smoke_dir="$(mktemp -d "${TMPDIR:-/tmp}/wms-h9-render-smoke.XXXXXX")"
db_secret_file="$smoke_dir/wms_staging_db_password.txt"
override_file="$smoke_dir/compose.override.yml"
render_request="$smoke_dir/render-request.json"
render_response="$smoke_dir/render-response.json"
worker_headers="$smoke_dir/worker-headers.txt"
worker_pdf="$smoke_dir/worker.pdf"

export COMPOSE_PROJECT_NAME="wms-h9-render-smoke-${RANDOM}-${BASHPID}"
export WMS_STAGING_API_PORT="$api_port"
export WMS_STAGING_MINIO_API_PORT="$minio_api_port"
export WMS_STAGING_MINIO_CONSOLE_PORT="$minio_console_port"
export WMS_STAGING_DB_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(24))')"
export WMS_JWT_SECRET="$(python3 -c 'import secrets; print(secrets.token_urlsafe(48))')"
export WMS_HFILE_ACCESS_KEY="wms_h9_smoke"
export WMS_HFILE_SECRET_KEY="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"
export WMS_HFILE_REGION="us-east-1"
export WMS_H9_RENDER_TOKEN="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"
printf '%s' "$WMS_STAGING_DB_PASSWORD" >"$db_secret_file"

cat >"$override_file" <<EOF
services:
  wms-api-staging:
    ports:
      - "${api_port}:8080"
  h9-render-worker-staging:
    ports:
      - "${worker_port}:18090"
secrets:
  wms_staging_db_password:
    file: ${db_secret_file}
EOF

cat >"$render_request" <<'EOF'
{"template":{"panels":[{"index":0,"paperType":"A4","printElements":[{"options":{"field":"wms_order_no","title":"出库单号","left":20,"top":20,"width":260,"height":24,"fontSize":18},"printElementType":{"type":"text"}}]}]},"data":{"wms_order_no":"AR-09-COMPOSE-SMOKE"}}
EOF

compose=(docker compose -f deploy/docker-compose.staging.yml -f "$override_file")
cleanup() {
  "${compose[@]}" down -v --remove-orphans >/dev/null 2>&1 || true
  rm -rf "$smoke_dir"
}
trap cleanup EXIT

"${compose[@]}" config --quiet
"${compose[@]}" up -d --build wms-api-staging

wait_http() {
  local url="$1"
  for _ in $(seq 1 60); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "timed out waiting for $url" >&2
  return 1
}

api_base="http://127.0.0.1:${api_port}"
wait_http "$api_base/healthz"
wait_http "$api_base/readyz"

resolve_api_target() {
  case "$1" in
    /*) printf '%s%s\n' "$api_base" "$1" ;;
    http://*|https://*) printf '%s\n' "$1" ;;
    *) echo "API smoke target must be an absolute URL or path: $1" >&2; return 2 ;;
  esac
}

core_url="$(resolve_api_target "${WMS_H9_SMOKE_CORE_URL:-/api/v1/inventory/batches}")"
print_url="$(resolve_api_target "$WMS_H9_SMOKE_PRINT_URL")"
curl -fsS -H "Authorization: $WMS_H9_SMOKE_AUTHORIZATION" "$core_url" >/dev/null

idempotency_key="${WMS_H9_SMOKE_IDEMPOTENCY_KEY:-ar-09-compose-smoke-$(date +%s)}"
print_response_status="$(curl -sS -o "$render_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $WMS_H9_SMOKE_AUTHORIZATION" \
  -H "Idempotency-Key: $idempotency_key" \
  "$print_url")"
if [[ "$print_response_status" != "502" ]]; then
  echo "worker-down print request returned $print_response_status, expected 502" >&2
  cat "$render_response" >&2
  exit 1
fi
python3 - "$render_response" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    body = json.load(handle)
assert body.get("code") == "H9_CATEGORY_PDF_RENDER_FAILED", body
PY

"${compose[@]}" up -d --build h9-render-worker-staging
wait_http "http://127.0.0.1:${worker_port}/healthz"

wrong_status="$(curl -sS -o /dev/null -w '%{http_code}' \
  -X POST \
  -H "Authorization: Bearer wrong-${WMS_H9_RENDER_TOKEN}" \
  -H 'Content-Type: application/json' \
  --data-binary "@$render_request" \
  "http://127.0.0.1:${worker_port}/render")"
[[ "$wrong_status" == "401" ]] || {
  echo "wrong render token returned $wrong_status, expected 401" >&2
  exit 1
}

curl -sS -D "$worker_headers" -o "$worker_pdf" \
  -X POST \
  -H "Authorization: Bearer $WMS_H9_RENDER_TOKEN" \
  -H 'Content-Type: application/json' \
  --data-binary "@$render_request" \
  "http://127.0.0.1:${worker_port}/render"
grep -qi '^content-type: application/pdf' "$worker_headers"
head -c 5 "$worker_pdf" | grep -q '^%PDF-'

recovered_status="$(curl -sS -o "$render_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $WMS_H9_SMOKE_AUTHORIZATION" \
  -H "Idempotency-Key: $idempotency_key" \
  "$print_url")"
if [[ "$recovered_status" != "200" ]]; then
  echo "recovered print retry returned $recovered_status, expected 200" >&2
  cat "$render_response" >&2
  exit 1
fi
python3 - "$render_response" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    body = json.load(handle)
assert body.get("status") == "completed", body
PY

echo "AR-09 render-worker compose smoke passed (project=$COMPOSE_PROJECT_NAME)"
