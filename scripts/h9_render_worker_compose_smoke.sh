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
compose_env_file="$smoke_dir/compose.env"
override_file="$smoke_dir/compose.override.yml"
cleanup_log="$smoke_dir/cleanup.log"
render_request="$smoke_dir/render-request.json"
render_response="$smoke_dir/render-response.json"
list_response="$smoke_dir/list-response.json"
suite_request="$smoke_dir/suite-request.json"
suite_response="$smoke_dir/suite-response.json"
suite_test_response="$smoke_dir/suite-test-response.json"
suite_publish_response="$smoke_dir/suite-publish-response.json"
cutoff_request="$smoke_dir/cutoff-request.json"
cutoff_response="$smoke_dir/cutoff-response.json"
instance_response="$smoke_dir/instance-response.json"
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
export WMS_E2E_SEED=1

evidence_root="${WMS_H9_SMOKE_EVIDENCE_DIR:-artifacts/h9-render-worker-compose-smoke}"
if [[ "$evidence_root" = /* ]]; then
  evidence_base="$evidence_root"
else
  evidence_base="$repo_root/$evidence_root"
fi
evidence_dir="$evidence_base/$COMPOSE_PROJECT_NAME"
mkdir -p "$evidence_dir"

api_healthz_status="not-run"
api_readyz_status="not-run"
core_status="not-run"
print_response_status="not-run"
worker_down_code="not-run"
worker_down_list_status="not-run"
worker_healthz_status="not-run"
wrong_status="not-run"
worker_render_status="not-run"
recovered_status="not-run"
recovered_list_status="not-run"
cleanup_status="not-run"

printf '%s' "$WMS_STAGING_DB_PASSWORD" >"$db_secret_file"
cat >"$compose_env_file" <<EOF
COMPOSE_PROJECT_NAME=$COMPOSE_PROJECT_NAME
WMS_STAGING_API_PORT=$api_port
WMS_STAGING_MINIO_API_PORT=$minio_api_port
WMS_STAGING_MINIO_CONSOLE_PORT=$minio_console_port
WMS_STAGING_DB_PASSWORD=$WMS_STAGING_DB_PASSWORD
WMS_JWT_SECRET=$WMS_JWT_SECRET
WMS_HFILE_ACCESS_KEY=$WMS_HFILE_ACCESS_KEY
WMS_HFILE_SECRET_KEY=$WMS_HFILE_SECRET_KEY
WMS_HFILE_REGION=$WMS_HFILE_REGION
WMS_H9_RENDER_TOKEN=$WMS_H9_RENDER_TOKEN
WMS_E2E_SEED=$WMS_E2E_SEED
EOF

auth_token="$(WMS_JWT_SECRET="$WMS_JWT_SECRET" python3 - <<'PY'
import base64
import hashlib
import hmac
import json
import os
import time

def encode(value):
    return base64.urlsafe_b64encode(
        json.dumps(value, separators=(",", ":")).encode()
    ).rstrip(b"=").decode()

now = int(time.time())
header = encode({"alg": "HS256", "typ": "JWT"})
payload = encode({
    "sub": "00000000-0000-0000-0000-000000000101",
    "owner_id": "00000000-0000-0000-0000-000000000001",
    "user_name": "ar-09-compose-smoke",
    "permissions": [
        "m3.read",
        "h9.print_orchestration.read",
        "h9.print_orchestration.write",
        "h9.print_pdf.read",
        "h9.print_pdf.prepare",
    ],
    "jti": "ar-09-compose-smoke",
    "iat": now,
    "exp": now + 3600,
})
body = f"{header}.{payload}"
signature = base64.urlsafe_b64encode(
    hmac.new(os.environ["WMS_JWT_SECRET"].encode(), body.encode(), hashlib.sha256).digest()
).rstrip(b"=").decode()
print(f"{body}.{signature}")
PY
)"
auth_header="Bearer $auth_token"
smoke_key_prefix="ar-09-compose"

cat >"$override_file" <<EOF
services:
  wms-api-staging:
    entrypoint: ["/app/wms-api-e2e"]
    environment:
      WMS_E2E_SEED: "1"
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

compose=(
  docker compose
  --env-file "$compose_env_file"
  -p "$COMPOSE_PROJECT_NAME"
  -f deploy/docker-compose.staging.yml
  -f "$override_file"
)
db_query() {
  "${compose[@]}" exec -T -e "PGPASSWORD=$WMS_STAGING_DB_PASSWORD" postgres-staging \
    psql -U wms_staging -d wms_staging -Atc "$1"
}
copy_evidence() {
  local source="$1"
  local target="$2"
  if [[ -f "$source" ]]; then
    cp "$source" "$evidence_dir/$target"
  fi
}
cleanup() {
  local exit_status="$?"
  if "${compose[@]}" down -v --remove-orphans >"$cleanup_log" 2>&1; then
    cleanup_status=0
  else
    cleanup_status=$?
  fi
  copy_evidence "$cleanup_log" cleanup.log
  copy_evidence "$render_request" render-request.json
  copy_evidence "$worker_headers" worker-headers.txt
  copy_evidence "$worker_pdf" worker.pdf
  copy_evidence "$render_response" recovered-response.json
  copy_evidence "$list_response" recovered-list-response.json
  python3 - "$evidence_dir/evidence.json" "$exit_status" "$cleanup_status" \
    "$api_port" "$worker_port" "$minio_api_port" "$minio_console_port" \
    "$api_healthz_status" "$api_readyz_status" "$core_status" \
    "$print_response_status" "$worker_down_code" "$worker_down_list_status" \
    "$worker_healthz_status" "$wrong_status" "$worker_render_status" \
    "$recovered_status" "$recovered_list_status" <<'PY'
import json
import os
import sys
from datetime import datetime, timezone

(
    output_path,
    exit_status,
    cleanup_status,
    api_port,
    worker_port,
    minio_api_port,
    minio_console_port,
    api_healthz_status,
    api_readyz_status,
    core_status,
    print_response_status,
    worker_down_code,
    worker_down_list_status,
    worker_healthz_status,
    wrong_status,
    worker_render_status,
    recovered_status,
    recovered_list_status,
) = sys.argv[1:]

def as_int(value):
    try:
        return int(value)
    except ValueError:
        return None

evidence = {
    "schema_version": 1,
    "captured_at": datetime.now(timezone.utc).isoformat(),
    "project_name": os.environ["COMPOSE_PROJECT_NAME"],
    "ports": {
        "api": as_int(api_port),
        "worker": as_int(worker_port),
        "minio_api": as_int(minio_api_port),
        "minio_console": as_int(minio_console_port),
    },
    "checks": {
        "api_healthz": api_healthz_status,
        "api_readyz": api_readyz_status,
        "authenticated_core_api": core_status,
        "worker_down_print_http": print_response_status,
        "worker_down_print_code": worker_down_code,
        "worker_down_persisted_status": worker_down_list_status,
        "worker_healthz": worker_healthz_status,
        "worker_wrong_token_http": wrong_status,
        "worker_correct_token_http": worker_render_status,
        "recovered_print_http": recovered_status,
        "recovered_persisted_status": recovered_list_status,
    },
    "process": {
        "smoke_exit_code": as_int(exit_status),
        "cleanup_exit_code": as_int(cleanup_status),
        "cleanup_command": "docker compose down -v --remove-orphans",
    },
    "secrets_included": False,
}
with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(evidence, handle, ensure_ascii=False, indent=2)
    handle.write("\n")
PY
  printf 'AR-09 evidence written: %s\n' "$evidence_dir"
  rm -rf "$smoke_dir"
  exit "$exit_status"
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
api_healthz_status="$(curl -sS -o /dev/null -w '%{http_code}' "$api_base/healthz")"
api_readyz_status="$(curl -sS -o /dev/null -w '%{http_code}' "$api_base/readyz")"

resolve_api_target() {
  case "$1" in
    /*) printf '%s%s\n' "$api_base" "$1" ;;
    http://*|https://*) printf '%s\n' "$1" ;;
    *) echo "API smoke target must be an absolute URL or path: $1" >&2; return 2 ;;
  esac
}

core_url="$(resolve_api_target "${WMS_H9_SMOKE_CORE_URL:-/api/v1/inventory/batches}")"
core_status="$(curl -sS -o /dev/null -w '%{http_code}' -H "Authorization: $auth_header" "$core_url")"
[[ "$core_status" =~ ^2[0-9][0-9]$ ]] || {
  echo "authenticated core API returned $core_status, expected 2xx" >&2
  exit 1
}

template_version_id="$(db_query "SELECT id FROM print_template_versions WHERE template_type_code = 'delivery_note' AND status = 'published' ORDER BY created_at DESC LIMIT 1" | tr -d '\r\n')"
test -n "$template_version_id"
python3 - "$template_version_id" "$suite_request" <<'PY'
import json
import sys

template_version_id, output_path = sys.argv[1:]
with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(
        {
            "name": "AR-09 Compose Smoke Suite",
            "warehouse_id": "00000000-0000-0000-0000-000000001301",
            "scope": "customer",
            "customer_id": "00000000-0000-0000-0000-000000001201",
            "delivery_address_id": None,
            "route_code": None,
            "effective_from": "2026-01-01T00:00:00Z",
            "effective_to": None,
            "items": [
                {
                    "category_code": "delivery_note",
                    "copies": 1,
                    "sort_order": 1,
                    "output_slot": "delivery_note",
                    "required": True,
                    "ready_policy": "wait_hold_instance",
                    "failure_policy": "pause_suite",
                    "source_mode": "rendered",
                    "template_version_id": template_version_id,
                    "external_file_ref": None,
                },
                {
                    "category_code": "invoice",
                    "copies": 1,
                    "sort_order": 2,
                    "output_slot": "invoice",
                    "required": True,
                    "ready_policy": "wait_hold_instance",
                    "failure_policy": "pause_suite",
                    "source_mode": "external_file",
                    "template_version_id": None,
                    "external_file_ref": "h-file:invoice",
                },
            ],
        },
        handle,
    )
PY

suite_status="$(curl -sS -o "$suite_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $auth_header" \
  -H 'Content-Type: application/json' \
  -H "Idempotency-Key: ${smoke_key_prefix}-suite" \
  --data-binary "@$suite_request" \
  "$api_base/api/v1/print-orchestration/print-suites/versions")"
[[ "$suite_status" == "200" ]] || { cat "$suite_response" >&2; exit 1; }
suite_id="$(python3 - "$suite_response" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)["id"])
PY
)"

suite_test_status="$(curl -sS -o "$suite_test_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $auth_header" \
  -H 'Content-Type: application/json' \
  -H "Idempotency-Key: ${smoke_key_prefix}-suite-test" \
  --data '{"group_ids":["00000000-0000-0000-0000-000000009610"]}' \
  "$api_base/api/v1/print-orchestration/print-suites/versions/$suite_id/test")"
[[ "$suite_test_status" == "200" ]] || { cat "$suite_test_response" >&2; exit 1; }

suite_publish_status="$(curl -sS -o "$suite_publish_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $auth_header" \
  -H "Idempotency-Key: ${smoke_key_prefix}-suite-publish" \
  "$api_base/api/v1/print-orchestration/print-suites/versions/$suite_id/publish")"
[[ "$suite_publish_status" == "200" ]] || { cat "$suite_publish_response" >&2; exit 1; }

python3 - "$cutoff_request" <<'PY'
import json
import sys
with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(
        {
            "warehouse_id": "00000000-0000-0000-0000-000000001301",
            "delivery_address_id": "00000000-0000-0000-0000-000000001211",
            "order_ids": ["00000000-0000-0000-0000-000000009611"],
            "reason": "AR-09 Compose worker recovery smoke",
        },
        handle,
    )
PY
cutoff_status="$(curl -sS -o "$cutoff_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $auth_header" \
  -H 'Content-Type: application/json' \
  -H "Idempotency-Key: ${smoke_key_prefix}-cutoff" \
  --data-binary "@$cutoff_request" \
  "$api_base/api/v1/print-orchestration/delivery-note-groups/manual-cutoff")"
[[ "$cutoff_status" == "200" ]] || { cat "$cutoff_response" >&2; exit 1; }
group_id="$(python3 - "$cutoff_response" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)["id"])
PY
)"
curl -fsS -o "$instance_response" \
  -H "Authorization: $auth_header" \
  "$api_base/api/v1/print-orchestration/suite-instances?group_id=$group_id"
instance_id="$(python3 - "$instance_response" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    body = json.load(handle)
assert len(body.get("data", [])) == 1, body
print(body["data"][0]["id"])
PY
)"
print_url="$api_base/api/v1/print-orchestration/suite-instances/$instance_id/category-pdfs/prepare"
list_url="$api_base/api/v1/print-orchestration/suite-instances/$instance_id/category-pdfs"

idempotency_key="${WMS_H9_SMOKE_IDEMPOTENCY_KEY:-ar-09-compose-smoke-$(date +%s)}"
print_response_status="$(curl -sS -o "$render_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $auth_header" \
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
worker_down_code="$(python3 - "$render_response" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle).get("code", "missing"))
PY
)"
cp "$render_response" "$evidence_dir/worker-down-response.json"
curl -fsS -o "$list_response" \
  -H "Authorization: $auth_header" \
  "$list_url"
python3 - "$list_response" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    body = json.load(handle)
assert body.get("preparation_status") == "failed", body
rendered_items = [item for item in body.get("data", []) if item.get("source_mode") == "rendered"]
assert rendered_items, body
assert all(item.get("processing_status") == "failed" for item in rendered_items), body
PY
worker_down_list_status=failed
cp "$list_response" "$evidence_dir/worker-down-list-response.json"

"${compose[@]}" up -d --build h9-render-worker-staging
wait_http "http://127.0.0.1:${worker_port}/healthz"
worker_healthz_status="$(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:${worker_port}/healthz")"

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

worker_render_status="$(curl -sS -D "$worker_headers" -o "$worker_pdf" -w '%{http_code}' \
  -X POST \
  -H "Authorization: Bearer $WMS_H9_RENDER_TOKEN" \
  -H 'Content-Type: application/json' \
  --data-binary "@$render_request" \
  "http://127.0.0.1:${worker_port}/render")"
[[ "$worker_render_status" == "200" ]] || {
  echo "correct render token returned $worker_render_status, expected 200" >&2
  exit 1
}
grep -qi '^content-type: application/pdf' "$worker_headers"
head -c 5 "$worker_pdf" | grep -q '^%PDF-'

recovered_status="$(curl -sS -o "$render_response" -w '%{http_code}' \
  -X POST \
  -H "Authorization: $auth_header" \
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
cp "$render_response" "$evidence_dir/recovered-response.json"
curl -fsS -o "$list_response" \
  -H "Authorization: $auth_header" \
  "$list_url"
python3 - "$list_response" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    body = json.load(handle)
assert body.get("preparation_status") == "completed", body
assert body.get("data"), body
assert all(item.get("processing_status") == "ready" for item in body["data"]), body
PY
recovered_list_status=completed
cp "$list_response" "$evidence_dir/recovered-list-response.json"

echo "AR-09 render-worker compose smoke passed (project=$COMPOSE_PROJECT_NAME)"
