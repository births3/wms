#!/usr/bin/env python3
"""Collect Wave 1 H2 runtime evidence after real dev performance runs."""
from __future__ import annotations

import argparse
import ipaddress
import json
import os
import re
import socket
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from urllib.parse import urlparse

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
H2_DRY_RUN_ALIAS = "dev-h2.wms.internal"
DEV_DB_HOST_ALLOWLIST_ENV = "WMS_DEV_DB_HOST_ALLOWLIST"
DEFAULT_FORMAL_DEV_DB_HOST = "pg-dev.wms.internal"


class EvidenceError(Exception):
    pass


def parse_latency_ms(value: str, unit: str) -> float:
    number = float(value)
    match unit.lower():
        case "us":
            return number / 1000.0
        case "ms":
            return number
        case "s":
            return number * 1000.0
        case _:
            raise EvidenceError(f"unsupported latency unit: {unit}")


def parse_wrk_output(text: str) -> tuple[float, float]:
    p99_match = re.search(r"^\s*99%\s+([0-9.]+)(us|ms|s)\s*$", text, re.MULTILINE)
    if not p99_match:
        raise EvidenceError("wrk output missing latency distribution 99% line")
    qps_match = re.search(r"Requests/sec:\s+([0-9.]+)", text)
    if not qps_match:
        raise EvidenceError("wrk output missing Requests/sec")
    p99_ms = parse_latency_ms(p99_match.group(1), p99_match.group(2))
    observed_qps = float(qps_match.group(1))
    return p99_ms, observed_qps


def parse_wrk_duration_seconds(text: str) -> int:
    match = re.search(r"Running\s+([0-9]+(?:\.[0-9]+)?)([smhd])\s+test\b", text, re.IGNORECASE)
    if not match:
        raise EvidenceError("wrk output missing Running <duration> test line")

    value = float(match.group(1))
    unit = match.group(2).lower()
    multiplier = {
        "s": 1,
        "m": 60,
        "h": 3600,
        "d": 86400,
    }[unit]
    return int(value * multiplier)


def run_psql_scalar(database_url: str, sql: str) -> str:
    result = subprocess.run(
        ["psql", database_url, "-v", "ON_ERROR_STOP=1", "-Atc", sql],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise EvidenceError(result.stderr.strip() or "psql command failed")
    return result.stdout.strip()


def count_audit_rows(database_url: str) -> int:
    return int(run_psql_scalar(database_url, "SELECT count(*) FROM audit_event;"))


def count_recent_seals(database_url: str) -> int:
    sql = """
WITH expected_days AS (
  SELECT generate_series(
    CURRENT_DATE - INTERVAL '7 days',
    CURRENT_DATE - INTERVAL '1 day',
    INTERVAL '1 day'
  )::date AS seal_date
)
SELECT count(*)
  FROM expected_days d
  JOIN audit_chain_seal s ON s.seal_date = d.seal_date;
"""
    return int(run_psql_scalar(database_url, sql))


def contains_environment_token(value: str, environment: str) -> bool:
    return re.search(rf"(^|[^0-9a-z]){re.escape(environment.lower())}([^0-9a-z]|$)", value.lower()) is not None


def contains_forbidden_boundary(value: str) -> bool:
    return re.search(
        r"(^|[^0-9a-z])(staging|local|prod|production|prodution|localhost|127\.0\.0\.1|0\.0\.0\.0|stub|mock|fake|example)([^0-9a-z]|$)",
        value.lower(),
    ) is not None


def validate_external_ref(label: str, value: str, environment: str) -> None:
    if not value:
        raise EvidenceError(f"{label} is required")
    if contains_forbidden_boundary(value):
        raise EvidenceError(
            f"{label} must be a dev boundary and must not point to staging/localhost/local/prod/production/stub/mock/fake/example boundaries"
        )
    if not contains_environment_token(value, environment):
        raise EvidenceError(f"{label} must include environment token: {environment}")


def database_boundary_value(database_url: str) -> str:
    parsed = urlparse(database_url)
    if not parsed.scheme or not parsed.netloc:
        return database_url
    host = parsed.hostname or ""
    port = f":{parsed.port}" if parsed.port else ""
    return f"{host}{port}{parsed.path}"


def database_host(database_url: str) -> str:
    parsed = urlparse(database_url)
    if parsed.scheme and parsed.netloc:
        return parsed.hostname or ""
    boundary = database_url.rsplit("@", 1)[-1]
    host_port = boundary.split("/", 1)[0]
    if host_port.startswith("[") and "]" in host_port:
        return host_port[1:].split("]", 1)[0]
    return host_port.split(":", 1)[0]


def validate_database_url(database_url: str, environment: str) -> None:
    host = database_host(database_url)
    if not host:
        raise EvidenceError("database-url host is required")
    if contains_forbidden_boundary(host):
        raise EvidenceError(
            "database-url must be a dev boundary and must not point to staging/localhost/local/prod/production/stub/mock/fake/example boundaries"
        )
    if is_raw_ip(host):
        raise EvidenceError("database-url host must be a dev DNS name, not a raw IP address")
    if not contains_environment_token(host, environment):
        raise EvidenceError(f"database-url must include environment token: {environment}")


def is_raw_ip(host: str) -> bool:
    try:
        ipaddress.ip_address(host)
    except ValueError:
        return False
    return True


def allowed_formal_dev_hosts() -> set[str]:
    raw = os.environ.get(DEV_DB_HOST_ALLOWLIST_ENV, "").strip() or DEFAULT_FORMAL_DEV_DB_HOST
    hosts = {host.strip().lower() for host in raw.split(",") if host.strip()}
    if not hosts:
        raise EvidenceError(f"{DEV_DB_HOST_ALLOWLIST_ENV} must contain at least one dev DB host")
    for host in hosts:
        if is_raw_ip(host):
            raise EvidenceError(f"{DEV_DB_HOST_ALLOWLIST_ENV} host {host} must be a DNS name, not raw IP")
        if not contains_environment_token(host, "dev"):
            raise EvidenceError(f"{DEV_DB_HOST_ALLOWLIST_ENV} host {host} must include dev boundary token")
        if host == H2_DRY_RUN_ALIAS:
            raise EvidenceError(f"{DEV_DB_HOST_ALLOWLIST_ENV} must not include dry-run alias {H2_DRY_RUN_ALIAS}")
    return hosts


def resolve_host_ips(host: str) -> list[str]:
    return sorted({info[4][0] for info in socket.getaddrinfo(host, None)})


def validated_host_ips(host: str) -> list[str]:
    try:
        ips = resolve_host_ips(host)
    except OSError as error:
        raise EvidenceError(f"database-url host {host} must resolve before formal H2 runtime evidence: {error}") from error
    if not ips:
        raise EvidenceError(f"database-url host {host} did not resolve to any IP address")
    for ip in ips:
        try:
            parsed = ipaddress.ip_address(ip)
        except ValueError as error:
            raise EvidenceError(f"database-url host {host} resolved invalid IP {ip}") from error
        if parsed.is_loopback:
            raise EvidenceError(
                f"database-url host {host} resolves to loopback and cannot be used for formal H2 runtime evidence"
            )
    return ips


def reject_loopback_resolution(host: str) -> None:
    validated_host_ips(host)


def validate_formal_database_url(database_url: str) -> None:
    host = database_host(database_url).lower()
    if H2_DRY_RUN_ALIAS == host:
        raise EvidenceError(
            "database-url uses dev-h2.wms.internal, which is only allowed for readiness dry-run and cannot be used for formal H2 runtime evidence"
        )
    if host not in allowed_formal_dev_hosts():
        raise EvidenceError(
            f"database-url host {host} is not in {DEV_DB_HOST_ALLOWLIST_ENV}; add the real dev PostgreSQL DNS before formal H2 runtime evidence"
        )
    reject_loopback_resolution(host)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database-url", required=True)
    parser.add_argument("--wrk-output", required=True, type=Path)
    parser.add_argument("--benchmark-log-ref", required=True)
    parser.add_argument("--cron-log-ref", required=True)
    parser.add_argument("--environment", default="dev", choices=["dev"])
    parser.add_argument("--duration-seconds", required=True, type=int)
    parser.add_argument("--target-qps", default=1000, type=int)
    parser.add_argument("--seal-failure-count", default=0, type=int)
    parser.add_argument(
        "--output",
        default=REPO_ROOT / "docs/retros/wave-1-h2-runtime-evidence.json",
        type=Path,
    )
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)

    try:
        validate_database_url(args.database_url, args.environment)
        validate_formal_database_url(args.database_url)
        db_host = database_host(args.database_url).lower()
        db_resolved_ips = validated_host_ips(db_host)
        validate_external_ref("benchmark-log-ref", args.benchmark_log_ref, args.environment)
        validate_external_ref("cron-log-ref", args.cron_log_ref, args.environment)
        wrk_text = args.wrk_output.read_text(encoding="utf-8")
        p99_ms, observed_qps = parse_wrk_output(wrk_text)
        observed_duration_seconds = parse_wrk_duration_seconds(wrk_text)
        baseline_rows = count_audit_rows(args.database_url)
        consecutive_success_days = count_recent_seals(args.database_url)

        if baseline_rows < 60_000_000:
            raise EvidenceError(f"baseline_rows must be >= 60000000, got {baseline_rows}")
        if args.target_qps < 1000:
            raise EvidenceError(f"target_qps must be >= 1000, got {args.target_qps}")
        if observed_qps < 1000:
            raise EvidenceError(f"observed_qps must be >= 1000, got {observed_qps}")
        if args.duration_seconds < 3600:
            raise EvidenceError(f"duration_seconds must be >= 3600, got {args.duration_seconds}")
        if observed_duration_seconds != args.duration_seconds:
            raise EvidenceError(
                "wrk output duration does not match --duration-seconds: "
                f"wrk={observed_duration_seconds}, cli={args.duration_seconds}"
            )
        if observed_duration_seconds < 3600:
            raise EvidenceError(
                f"wrk output duration_seconds must be >= 3600, got {observed_duration_seconds}"
            )
        if p99_ms >= 200.0:
            raise EvidenceError(f"p99_ms must be < 200, got {p99_ms}")
        if consecutive_success_days < 7:
            raise EvidenceError(
                f"seal_cron consecutive_success_days must be >= 7, got {consecutive_success_days}"
            )
        if args.seal_failure_count != 0:
            raise EvidenceError("seal_cron failure_count must be 0")

        payload = {
            "environment": args.environment,
            "captured_at": datetime.now().astimezone().isoformat(timespec="seconds"),
            "database": {
                "host": db_host,
                "resolved_ips": db_resolved_ips,
            },
            "performance": {
                "tool": "wrk",
                "baseline_rows": baseline_rows,
                "target_qps": args.target_qps,
                "observed_qps": observed_qps,
                "duration_seconds": args.duration_seconds,
                "p99_ms": p99_ms,
                "benchmark_log_ref": args.benchmark_log_ref,
            },
            "seal_cron": {
                "consecutive_success_days": consecutive_success_days,
                "failure_count": args.seal_failure_count,
                "last_seal_verified": True,
                "cron_log_ref": args.cron_log_ref,
            },
        }
        if args.output.exists() and not args.force:
            raise EvidenceError(f"{args.output} already exists; pass --force to overwrite")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {args.output}")
        return 0
    except (EvidenceError, OSError, ValueError) as error:
        print(f"wave1 h2 runtime evidence rejected: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
