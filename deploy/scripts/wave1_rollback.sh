#!/usr/bin/env bash
# Wave 1 rollback helper for dev/staging validation.
#
# The script deliberately supports both ADR-0016 rollback paths and does not
# choose a deployment target for the project. Default mode is dry-run.

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  wave1_rollback.sh --target k8s --environment dev|staging [--deployment wms-api] [--context ctx] [--namespace ns] [--execute]
  wave1_rollback.sh --target docker-compose --environment dev|staging --previous-version <sha> [--compose-file path] [--compose-env-file path] [--execute]

Default mode prints the command. Add --execute to run it.
When --execute is used, k8s requires --context and --namespace; docker-compose requires --compose-file.

ADR-0016 rollback commands represented:
  k8s:            kubectl rollout undo deployment/wms-api
  docker-compose: WMS_VERSION=<prev-sha> docker compose up -d --no-build
USAGE
}

target=""
environment=""
deployment="wms-api"
context=""
namespace=""
previous_version=""
compose_file=""
compose_file_abs=""
compose_env_file="${WAVE1_COMPOSE_ENV_FILE:-}"
compose_env_file_abs=""
execute="false"
docker_bin="${WAVE1_DOCKER_BIN:-docker}"

need_value() {
  if [ -z "${2:-}" ] || [[ "${2:-}" == --* ]]; then
    echo "$1 requires a value" >&2
    exit 2
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --target)
      need_value "$1" "${2:-}"
      target="${2:-}"
      shift 2
      ;;
    --environment)
      need_value "$1" "${2:-}"
      environment="${2:-}"
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
    --compose-env-file)
      need_value "$1" "${2:-}"
      compose_env_file="${2:-}"
      shift 2
      ;;
    --execute)
      execute="true"
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

contains_environment_token() {
  local value="${1,,}"
  local token="${2,,}"
  [[ "$value" =~ (^|[^[:alnum:]])${token}([^[:alnum:]]|$) ]]
}

contains_production_token() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(prod|production|prodution)([^[:alnum:]]|$) ]]
}

contains_local_boundary() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(local|localhost|127\.0\.0\.1|0\.0\.0\.0)([^[:alnum:]]|$) ]]
}

contains_stub_runtime_boundary() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(stub|mock|fake)([^[:alnum:]]|$) ]]
}

contains_example_boundary() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])example([^[:alnum:]]|$) ]]
}

contains_template_placeholder() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(yyyy|todo|tbd)([^[:alnum:]]|$) ]] \
    || [[ "$value" == *"<"*">"* ]] \
    || [[ "$value" == *"待填"* ]] \
    || [[ "$value" == *"待确认"* ]]
}

validate_runtime_value() {
  local label="$1"
  local value="$2"

  if contains_production_token "$value"; then
    echo "${label} must not point to a production boundary when --execute is used" >&2
    exit 2
  fi
  if contains_local_boundary "$value"; then
    echo "${label} must not point to a local boundary when --execute is used" >&2
    exit 2
  fi
  if contains_stub_runtime_boundary "$value"; then
    echo "${label} must not point to a stub/mock/fake boundary when --execute is used" >&2
    exit 2
  fi
  if contains_example_boundary "$value"; then
    echo "${label} must not point to an example boundary when --execute is used" >&2
    exit 2
  fi
  if contains_template_placeholder "$value"; then
    echo "${label} must not contain a template placeholder when --execute is used" >&2
    exit 2
  fi
}

validate_environment_boundary() {
  local label="$1"
  local value="$2"

  validate_runtime_value "$label" "$value"
  if ! contains_environment_token "$value" "$environment"; then
    echo "${label} must include the selected environment token (${environment}) when --execute is used" >&2
    exit 2
  fi
}

case "$target" in
  k8s)
    if [ -z "$deployment" ]; then
      echo "--deployment is required for k8s rollback" >&2
      exit 2
    fi
    cmd=(kubectl rollout undo "deployment/${deployment}")
    if [ -n "$context" ]; then
      cmd+=(--context "$context")
    fi
    if [ -n "$namespace" ]; then
      cmd+=(--namespace "$namespace")
    fi
    ;;
  docker-compose)
    if [ -z "$previous_version" ]; then
      echo "--previous-version is required for docker-compose rollback" >&2
      exit 2
    fi
    if [ -n "$compose_file" ]; then
      if [ "$execute" = "true" ] && [ ! -f "$compose_file" ]; then
        echo "--compose-file must point to an existing file when --execute is used" >&2
        exit 2
      fi
      if [ -f "$compose_file" ]; then
        compose_dir="$(cd "$(dirname "$compose_file")" && pwd -P)"
        compose_file_abs="${compose_dir}/$(basename "$compose_file")"
      else
        compose_file_abs="$compose_file"
      fi
    fi
    if [ -n "$compose_env_file" ]; then
      if [ "$execute" = "true" ] && [ ! -f "$compose_env_file" ]; then
        echo "--compose-env-file must point to an existing file when --execute is used" >&2
        exit 2
      fi
      if [ -f "$compose_env_file" ]; then
        compose_env_dir="$(cd "$(dirname "$compose_env_file")" && pwd -P)"
        compose_env_file_abs="${compose_env_dir}/$(basename "$compose_env_file")"
      else
        compose_env_file_abs="$compose_env_file"
      fi
    fi
    cmd=("$docker_bin" compose)
    if [ -n "$compose_env_file_abs" ]; then
      cmd+=(--env-file "$compose_env_file_abs")
    fi
    if [ -n "$compose_file" ]; then
      cmd+=(-f "$compose_file_abs")
    fi
    cmd+=(up -d --no-build)
    ;;
  *)
    echo "--target must be k8s or docker-compose" >&2
    exit 2
    ;;
esac

if [ "$execute" = "true" ]; then
  case "$target" in
    k8s)
      if [ -z "$context" ] || [ -z "$namespace" ]; then
        echo "--context and --namespace are required for k8s rollback when --execute is used" >&2
        exit 2
      fi
      validate_environment_boundary "--context" "$context"
      validate_environment_boundary "--namespace" "$namespace"
      ;;
    docker-compose)
      if [ -z "$compose_file" ]; then
        echo "--compose-file is required for docker-compose rollback when --execute is used" >&2
        exit 2
      fi
      validate_runtime_value "--previous-version" "$previous_version"
      validate_environment_boundary "--compose-file" "$compose_file_abs"
      if [ -n "$compose_env_file_abs" ]; then
        validate_runtime_value "--compose-env-file" "$compose_env_file_abs"
      fi
      ;;
  esac
fi

boundary="target=${target} environment=${environment} execute=${execute}"
case "$target" in
  k8s)
    boundary="${boundary} context=${context:-<not-set>} namespace=${namespace:-<not-set>} deployment=${deployment}"
    ;;
  docker-compose)
    boundary="${boundary} compose_file=${compose_file_abs:-<not-set>} compose_env_file=${compose_env_file_abs:-<not-set>} previous_version=${previous_version}"
    ;;
esac
echo "wave1 rollback ${boundary}"

if [ "$execute" != "true" ]; then
  if [ "$target" = "docker-compose" ]; then
    printf 'dry-run: WMS_VERSION=%q ' "$previous_version"
  else
    printf 'dry-run: '
  fi
  printf '%q ' "${cmd[@]}"
  printf '\n'
  exit 0
fi

if [ "$target" = "docker-compose" ]; then
  WMS_VERSION="$previous_version" "${cmd[@]}"
else
  "${cmd[@]}"
fi
