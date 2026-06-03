#!/usr/bin/env bash
# Wave 1 automatic rollback probe for real dev/staging validation.
#
# This probe requires a real HTTP smoke URL or a real Prometheus signal.
# Missing runtime signal configuration is treated as missing evidence and exits
# non-zero. A failing signal triggers wave1_rollback.sh --execute.

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  wave1_auto_rollback_probe.sh --environment dev|staging --target k8s --context <ctx> --namespace <ns> [--deployment wms-api] [--smoke-url <url>]
  wave1_auto_rollback_probe.sh --environment dev|staging --target k8s --context <ctx> --namespace <ns> [--deployment wms-api] [--prometheus-url <url>] [--promql <query>]
  wave1_auto_rollback_probe.sh --environment dev|staging --target docker-compose --previous-version <sha> --compose-file <path> [--smoke-url <url>]
  wave1_auto_rollback_probe.sh --environment dev|staging --target docker-compose --previous-version <sha> --compose-file <path> [--prometheus-url <url>] [--promql <query>]

Signal configuration:
  HTTP smoke:   use --smoke-url or SMOKE_URL. HTTP 2xx/3xx means healthy.
  Prometheus:   use --prometheus-url or PROMETHEUS_URL together with
                --promql or PROMETHEUS_QUERY. The query must evaluate to 0
                when healthy and >0 when rollback should trigger.

If the runtime signal fails, the script invokes wave1_rollback.sh --execute.
If no real signal configuration is supplied, the script exits non-zero because
there is no dev/staging runtime evidence.

Evidence output:
  --evidence-file <path> writes Wave 1 runtime evidence JSON only after a
  failed real signal triggers rollback and rollback exits 0. When used, also
  pass --rollback-log-ref and --external-log-ref.

Readiness:
  --check-only validates the exact runtime boundary, command availability and
  evidence references, then exits without calling the signal endpoint, running
  rollback or writing evidence.
USAGE
}

environment=""
target=""
deployment="wms-api"
context=""
namespace=""
previous_version=""
compose_file=""
smoke_url="${SMOKE_URL:-}"
prometheus_url="${PROMETHEUS_URL:-}"
promql="${PROMETHEUS_QUERY:-}"
curl_max_time="${CURL_MAX_TIME_SECONDS:-10}"
allow_local_test_signal="${WAVE1_ALLOW_LOCAL_TEST_SIGNAL:-false}"
evidence_file=""
rollback_log_ref=""
external_log_ref=""
check_only="false"

need_value() {
  if [ -z "${2:-}" ] || [[ "${2:-}" == --* ]]; then
    echo "$1 requires a value" >&2
    exit 2
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --environment)
      need_value "$1" "${2:-}"
      environment="${2:-}"
      shift 2
      ;;
    --target)
      need_value "$1" "${2:-}"
      target="${2:-}"
      shift 2
      ;;
    --deployment)
      need_value "$1" "${2:-}"
      deployment="${2:-}"
      shift 2
      ;;
    --context)
      need_value "$1" "${2:-}"
      context="${2:-}"
      shift 2
      ;;
    --namespace)
      need_value "$1" "${2:-}"
      namespace="${2:-}"
      shift 2
      ;;
    --previous-version)
      need_value "$1" "${2:-}"
      previous_version="${2:-}"
      shift 2
      ;;
    --compose-file)
      need_value "$1" "${2:-}"
      compose_file="${2:-}"
      shift 2
      ;;
    --smoke-url)
      need_value "$1" "${2:-}"
      smoke_url="${2:-}"
      shift 2
      ;;
    --prometheus-url)
      need_value "$1" "${2:-}"
      prometheus_url="${2:-}"
      shift 2
      ;;
    --promql)
      need_value "$1" "${2:-}"
      promql="${2:-}"
      shift 2
      ;;
    --curl-max-time)
      need_value "$1" "${2:-}"
      curl_max_time="${2:-}"
      shift 2
      ;;
    --evidence-file)
      need_value "$1" "${2:-}"
      evidence_file="${2:-}"
      shift 2
      ;;
    --rollback-log-ref)
      need_value "$1" "${2:-}"
      rollback_log_ref="${2:-}"
      shift 2
      ;;
    --external-log-ref)
      need_value "$1" "${2:-}"
      external_log_ref="${2:-}"
      shift 2
      ;;
    --check-only)
      check_only="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$environment" in
  dev|staging) ;;
  *)
    echo "--environment must be dev or staging" >&2
    exit 2
    ;;
esac

case "$target" in
  k8s|docker-compose) ;;
  *)
    echo "--target must be k8s or docker-compose" >&2
    exit 2
    ;;
esac

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
rollback_script="${script_dir}/wave1_rollback.sh"

contains_environment_token() {
  local value="${1,,}"
  local token="${2,,}"
  [[ "$value" =~ (^|[^[:alnum:]])${token}([^[:alnum:]]|$) ]]
}

contains_production_token() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(prod|production|prodution)([^[:alnum:]]|$) ]]
}

contains_stub_runtime_boundary() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(stub|mock|fake)([^[:alnum:]]|$) ]]
}

contains_example_runtime_boundary() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])example([^[:alnum:]]|$) ]]
}

contains_local_runtime_boundary() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(localhost|127\.0\.0\.1|0\.0\.0\.0)([^[:alnum:]]|$) ]]
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "required command not found: $1" >&2
    exit 2
  fi
}

validate_environment_boundary() {
  local label="$1"
  local value="$2"

  if contains_production_token "$value"; then
    echo "${label} must not point to a production boundary" >&2
    exit 2
  fi
  if contains_stub_runtime_boundary "$value"; then
    echo "${label} must not point to a stub/mock/fake boundary" >&2
    exit 2
  fi
  if contains_example_runtime_boundary "$value"; then
    echo "${label} must not point to an example boundary" >&2
    exit 2
  fi
  if ! contains_environment_token "$value" "$environment"; then
    echo "${label} must include the selected environment token (${environment})" >&2
    exit 2
  fi
}

validate_evidence_ref() {
  local label="$1"
  local value="$2"

  if [ -z "$value" ]; then
    echo "${label} is required when --evidence-file is used" >&2
    exit 2
  fi
  if contains_production_token "$value"; then
    echo "${label} must not point to a production boundary" >&2
    exit 2
  fi
  if contains_stub_runtime_boundary "$value"; then
    echo "${label} must not point to a stub/mock/fake boundary" >&2
    exit 2
  fi
  if contains_example_runtime_boundary "$value"; then
    echo "${label} must not point to an example boundary" >&2
    exit 2
  fi
  if [ "$allow_local_test_signal" != "true" ] && contains_local_runtime_boundary "$value"; then
    echo "${label} must not reference localhost/127.0.0.1 unless WAVE1_ALLOW_LOCAL_TEST_SIGNAL=true" >&2
    exit 2
  fi
  if ! contains_environment_token "$value" "$environment"; then
    echo "${label} must include the selected environment token (${environment})" >&2
    exit 2
  fi
}

validate_prometheus_boundary() {
  local url="$1"
  local query="$2"

  if contains_production_token "$url" || contains_production_token "$query"; then
    echo "Prometheus boundary must not reference prod/production/prodution" >&2
    exit 2
  fi
  if contains_stub_runtime_boundary "$url" || contains_stub_runtime_boundary "$query"; then
    echo "Prometheus boundary must not reference stub/mock/fake" >&2
    exit 2
  fi
  if contains_example_runtime_boundary "$url" || contains_example_runtime_boundary "$query"; then
    echo "Prometheus boundary must not reference example" >&2
    exit 2
  fi
  if [ "$allow_local_test_signal" != "true" ] && contains_local_runtime_boundary "$url"; then
    echo "Prometheus boundary must not reference localhost/127.0.0.1 unless WAVE1_ALLOW_LOCAL_TEST_SIGNAL=true" >&2
    exit 2
  fi
  if contains_environment_token "$url" "$environment" || contains_environment_token "$query" "$environment"; then
    return 0
  fi

  echo "Prometheus URL or PromQL must include the selected environment token (${environment})" >&2
  exit 2
}

validate_signal_boundary_only() {
  local mode="$1"

  case "$mode" in
    http)
      require_command curl
      validate_environment_boundary "--smoke-url" "$smoke_url"
      if [ "$allow_local_test_signal" != "true" ] && contains_local_runtime_boundary "$smoke_url"; then
        echo "HTTP smoke URL must not reference localhost/127.0.0.1 unless WAVE1_ALLOW_LOCAL_TEST_SIGNAL=true" >&2
        exit 2
      fi
      ;;
    prometheus)
      require_command curl
      require_command python3
      validate_prometheus_boundary "$prometheus_url" "$promql"
      ;;
    *)
      echo "unsupported signal mode: $mode" >&2
      exit 2
      ;;
  esac
}

select_signal_mode() {
  if [ -n "$smoke_url" ] && { [ -n "$prometheus_url" ] || [ -n "$promql" ]; }; then
    echo "configure either HTTP smoke or Prometheus for one run, not both" >&2
    exit 2
  fi

  if [ -n "$smoke_url" ]; then
    printf 'http\n'
    return 0
  fi

  if [ -n "$prometheus_url" ] || [ -n "$promql" ]; then
    if [ -z "$prometheus_url" ] || [ -z "$promql" ]; then
      echo "Prometheus runtime evidence requires both PROMETHEUS_URL and PROMETHEUS_QUERY (or --prometheus-url and --promql)" >&2
      exit 2
    fi
    printf 'prometheus\n'
    return 0
  fi

  echo "missing runtime evidence: provide --smoke-url/SMOKE_URL or PROMETHEUS_URL + PROMETHEUS_QUERY" >&2
  exit 2
}

prometheus_query_endpoint() {
  if [[ "$prometheus_url" == */api/v1/query ]]; then
    printf '%s\n' "$prometheus_url"
  else
    printf '%s/api/v1/query\n' "${prometheus_url%/}"
  fi
}

probe_http_smoke() {
  require_command curl
  validate_environment_boundary "--smoke-url" "$smoke_url"
  if [ "$allow_local_test_signal" != "true" ] && contains_local_runtime_boundary "$smoke_url"; then
    echo "HTTP smoke URL must not reference localhost/127.0.0.1 unless WAVE1_ALLOW_LOCAL_TEST_SIGNAL=true" >&2
    exit 2
  fi

  local response_file http_code curl_exit
  response_file="$(mktemp)"

  if http_code="$(curl --silent --show-error --output "$response_file" --write-out '%{http_code}' --max-time "$curl_max_time" "$smoke_url")"; then
    if [ "$http_code" -ge 200 ] && [ "$http_code" -lt 400 ]; then
      rm -f "$response_file"
      echo "smoke gate healthy: type=http url=${smoke_url} http_status=${http_code}"
      return 0
    fi
    rm -f "$response_file"
    echo "smoke gate failure: type=http url=${smoke_url} http_status=${http_code}" >&2
    return 1
  fi

  curl_exit=$?
  rm -f "$response_file"
  echo "smoke gate request failed: type=http url=${smoke_url} curl_exit=${curl_exit}" >&2
  return 1
}

probe_prometheus() {
  require_command curl
  require_command python3
  validate_prometheus_boundary "$prometheus_url" "$promql"

  local endpoint response parse_output curl_exit parse_exit
  endpoint="$(prometheus_query_endpoint)"

  if response="$(curl --silent --show-error --get --max-time "$curl_max_time" --data-urlencode "query=${promql}" "$endpoint")"; then
    :
  else
    curl_exit=$?
    echo "smoke gate request failed: type=prometheus url=${endpoint} curl_exit=${curl_exit}" >&2
    return 1
  fi

  if parse_output="$(
    printf '%s' "$response" | python3 -c '
import json
import sys

payload = json.load(sys.stdin)
if payload.get("status") != "success":
    print(payload.get("error", "prometheus_status_not_success"), file=sys.stderr)
    sys.exit(3)

data = payload.get("data", {})
result_type = data.get("resultType")

values = []
if result_type == "vector":
    for item in data.get("result", []):
        value = item.get("value", [])
        if len(value) >= 2:
            values.append(float(value[1]))
elif result_type == "scalar":
    value = data.get("result", [])
    if len(value) >= 2:
        values.append(float(value[1]))
else:
    print(f"unsupported resultType={result_type}", file=sys.stderr)
    sys.exit(4)

max_value = max(values) if values else 0.0
print(max_value)
sys.exit(10 if max_value > 0 else 0)
')"; then
    :
  else
    parse_exit=$?
    case "$parse_exit" in
      10)
        echo "smoke gate failure: type=prometheus url=${endpoint} max_value=${parse_output}" >&2
        return 1
        ;;
      *)
        echo "failed to parse Prometheus response for runtime evidence" >&2
        return 2
        ;;
    esac
  fi

  echo "smoke gate healthy: type=prometheus url=${endpoint} max_value=${parse_output}"
  return 0
}

run_signal_gate() {
  local mode="$1"

  case "$mode" in
    http)
      probe_http_smoke
      ;;
    prometheus)
      probe_prometheus
      ;;
    *)
      echo "unsupported signal mode: $mode" >&2
      exit 2
      ;;
  esac
}

build_rollback_args() {
  rollback_args=(--target "$target" --environment "$environment")

  case "$target" in
    k8s)
      rollback_args+=(--deployment "$deployment" --context "$context" --namespace "$namespace")
      ;;
    docker-compose)
      rollback_args+=(--previous-version "$previous_version" --compose-file "$compose_file")
      ;;
  esac
}

validate_rollback_configuration() {
  case "$target" in
    k8s)
      require_command kubectl
      if [ -z "$context" ] || [ -z "$namespace" ]; then
        echo "k8s rollback requires --context and --namespace" >&2
        exit 2
      fi
      validate_environment_boundary "--context" "$context"
      validate_environment_boundary "--namespace" "$namespace"
      ;;
    docker-compose)
      require_command docker
      if [ -z "$previous_version" ] || [ -z "$compose_file" ]; then
        echo "docker-compose rollback requires --previous-version and --compose-file" >&2
        exit 2
      fi
      if [ ! -f "$compose_file" ]; then
        echo "--compose-file must point to an existing file" >&2
        exit 2
      fi
      validate_environment_boundary "--compose-file" "$compose_file"
      ;;
  esac
}

validate_evidence_configuration() {
  if [ -z "$evidence_file" ]; then
    return 0
  fi

  validate_evidence_ref "--rollback-log-ref" "$rollback_log_ref"
  validate_evidence_ref "--external-log-ref" "$external_log_ref"
}

write_evidence_file() {
  local signal_type="$1"
  local signal_url="$2"
  local rollback_exit_code="$3"

  if [ -z "$evidence_file" ]; then
    return 0
  fi

  require_command python3
  validate_evidence_ref "--rollback-log-ref" "$rollback_log_ref"
  validate_evidence_ref "--external-log-ref" "$external_log_ref"

  EVIDENCE_FILE="$evidence_file" \
  ENVIRONMENT="$environment" \
  SIGNAL_TYPE="$signal_type" \
  SIGNAL_URL="$signal_url" \
  ROLLBACK_EXIT_CODE="$rollback_exit_code" \
  ROLLBACK_LOG_REF="$rollback_log_ref" \
  EXTERNAL_LOG_REF="$external_log_ref" \
  python3 - <<'PY'
import json
import os
from datetime import datetime, timezone
from pathlib import Path

path = Path(os.environ["EVIDENCE_FILE"])
payload = {
    "environment": os.environ["ENVIRONMENT"],
    "captured_at": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
    "signal_type": os.environ["SIGNAL_TYPE"],
    "signal_url": os.environ["SIGNAL_URL"],
    "rollback_triggered": True,
    "rollback_exit_code": int(os.environ["ROLLBACK_EXIT_CODE"]),
    "rollback_log_ref": os.environ["ROLLBACK_LOG_REF"],
    "external_log_ref": os.environ["EXTERNAL_LOG_REF"],
}
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(f"wrote {path}")
PY
}

signal_mode="$(select_signal_mode)"
build_rollback_args
validate_rollback_configuration

if [ "$check_only" = "true" ]; then
  validate_signal_boundary_only "$signal_mode"
  validate_evidence_configuration
  echo "wave1 auto rollback readiness ok environment=${environment} target=${target} signal=${signal_mode}"
  exit 0
fi

echo "wave1 auto rollback probe environment=${environment} target=${target} signal=${signal_mode}"

if run_signal_gate "$signal_mode"; then
  echo "runtime signal healthy; rollback not triggered"
  exit 0
else
  signal_exit=$?
  if [ "$signal_exit" -eq 1 ]; then
    echo "runtime signal failed; invoking rollback"
    set +e
    "$rollback_script" "${rollback_args[@]}" --execute
    rollback_exit=$?
    set -e
    if [ "$rollback_exit" -eq 0 ]; then
      case "$signal_mode" in
        http) evidence_signal_url="$smoke_url" ;;
        prometheus) evidence_signal_url="$(prometheus_query_endpoint)" ;;
        *) evidence_signal_url="" ;;
      esac
      write_evidence_file "$signal_mode" "$evidence_signal_url" "$rollback_exit"
    fi
    exit "$signal_exit"
  fi
  exit "$signal_exit"
fi
