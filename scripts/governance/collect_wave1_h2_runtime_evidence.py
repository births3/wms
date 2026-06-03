#!/usr/bin/env python3
"""Collect Wave 1 H2 runtime evidence after real dev performance runs."""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from urllib.parse import urlparse

REPO_ROOT = Path(__file__).resolve().parent.parent.parent


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
        r"(^|[^0-9a-z])(prod|production|prodution|localhost|127\.0\.0\.1|0\.0\.0\.0|stub|mock|fake|example)([^0-9a-z]|$)",
        value.lower(),
    ) is not None


def validate_external_ref(label: str, value: str, environment: str) -> None:
    if not value:
        raise EvidenceError(f"{label} is required")
    if contains_forbidden_boundary(value):
        raise EvidenceError(f"{label} must not point to localhost/prod/stub/mock/fake/example boundaries")
    if not contains_environment_token(value, environment):
        raise EvidenceError(f"{label} must include environment token: {environment}")


def database_boundary_value(database_url: str) -> str:
    parsed = urlparse(database_url)
    if not parsed.scheme or not parsed.netloc:
        return database_url
    host = parsed.hostname or ""
    port = f":{parsed.port}" if parsed.port else ""
    return f"{host}{port}{parsed.path}"


def validate_database_url(database_url: str, environment: str) -> None:
    boundary = database_boundary_value(database_url)
    if contains_forbidden_boundary(boundary):
        raise EvidenceError("database-url must not point to localhost/prod/stub/mock/fake/example boundaries")
    if not contains_environment_token(boundary, environment):
        raise EvidenceError(f"database-url must include environment token: {environment}")


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
    args = parser.parse_args(argv)

    try:
        validate_database_url(args.database_url, args.environment)
        validate_external_ref("benchmark-log-ref", args.benchmark_log_ref, args.environment)
        validate_external_ref("cron-log-ref", args.cron_log_ref, args.environment)
        wrk_text = args.wrk_output.read_text(encoding="utf-8")
        p99_ms, observed_qps = parse_wrk_output(wrk_text)
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
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {args.output}")
        return 0
    except (EvidenceError, OSError, ValueError) as error:
        print(f"wave1 h2 runtime evidence rejected: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
