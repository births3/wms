#!/usr/bin/env bash
# Wave 1 H2 audit maintenance helper.
#
# Intended for dev/staging cron validation until H-SCH owns scheduled jobs.
# It creates the next monthly audit partition and seals the requested audit day.

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  DATABASE_URL=postgres://... audit_maintenance.sh [--seal-date YYYY-MM-DD]

Defaults:
  --seal-date defaults to yesterday in the local shell timezone.
  AUDIT_MAINTENANCE_BIN defaults to audit-maintenance.
USAGE
}

seal_date=""

need_value() {
  if [ -z "${2:-}" ] || [[ "${2:-}" == --* ]]; then
    echo "$1 requires a value" >&2
    exit 2
  fi
}

contains_forbidden_boundary() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(local|localhost|127\.0\.0\.1|0\.0\.0\.0|prod|production|prodution|stub|mock|fake|example)([^[:alnum:]]|$) ]]
}

contains_template_placeholder() {
  local value="${1,,}"
  [[ "$value" =~ (^|[^[:alnum:]])(yyyy|todo|tbd)([^[:alnum:]]|$) ]] \
    || [[ "$value" == *"<"*">"* ]] \
    || [[ "$value" == *"待填"* ]] \
    || [[ "$value" == *"待确认"* ]]
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --seal-date)
      need_value "$1" "${2:-}"
      seal_date="${2:-}"
      shift 2
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

if [ -z "${DATABASE_URL:-}" ]; then
  echo "DATABASE_URL is required" >&2
  exit 2
fi
if contains_forbidden_boundary "$DATABASE_URL"; then
  echo "DATABASE_URL must not point to local/prod/production/stub/mock/fake/example boundaries" >&2
  exit 2
fi
if contains_template_placeholder "$DATABASE_URL"; then
  echo "DATABASE_URL must not contain a template placeholder" >&2
  exit 2
fi

maintenance_bin="${AUDIT_MAINTENANCE_BIN:-audit-maintenance}"
if ! command -v "$maintenance_bin" >/dev/null 2>&1; then
  echo "audit maintenance binary not found: ${maintenance_bin}" >&2
  exit 2
fi

if [ -z "$seal_date" ]; then
  seal_date="$(date -d yesterday +%F)"
fi

case "$seal_date" in
  [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) ;;
  *)
    echo "--seal-date must use YYYY-MM-DD" >&2
    exit 2
    ;;
esac

"$maintenance_bin" --seal-date "$seal_date"
