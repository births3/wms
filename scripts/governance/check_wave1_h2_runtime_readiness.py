#!/usr/bin/env python3
"""Check Wave 1 H2 dev database readiness before running the 1h wrk test."""
from __future__ import annotations

import argparse
import json
import sys

from collect_wave1_h2_runtime_evidence import (
    EvidenceError,
    count_audit_rows,
    count_recent_seals,
    validate_database_url,
)


def check_readiness(
    database_url: str,
    environment: str,
    min_baseline_rows: int,
    min_seal_days: int,
) -> tuple[bool, dict[str, int | str], list[str]]:
    validate_database_url(database_url, environment)
    baseline_rows = count_audit_rows(database_url)
    consecutive_success_days = count_recent_seals(database_url)
    facts: dict[str, int | str] = {
        "environment": environment,
        "baseline_rows": baseline_rows,
        "consecutive_success_days": consecutive_success_days,
        "min_baseline_rows": min_baseline_rows,
        "min_seal_days": min_seal_days,
    }

    issues: list[str] = []
    if baseline_rows < min_baseline_rows:
        issues.append(f"baseline_rows must be >= {min_baseline_rows}, got {baseline_rows}")
    if consecutive_success_days < min_seal_days:
        issues.append(
            f"consecutive_success_days must be >= {min_seal_days}, got {consecutive_success_days}"
        )
    return not issues, facts, issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database-url", required=True)
    parser.add_argument("--environment", default="dev", choices=["dev"])
    parser.add_argument("--min-baseline-rows", default=60_000_000, type=int)
    parser.add_argument("--min-seal-days", default=7, type=int)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    try:
        ok, facts, issues = check_readiness(
            args.database_url,
            args.environment,
            args.min_baseline_rows,
            args.min_seal_days,
        )
    except (EvidenceError, OSError, ValueError) as error:
        if args.json:
            print(json.dumps({
                "ok": False,
                "error": str(error),
            }, ensure_ascii=False, indent=2))
        else:
            print(f"wave1 h2 runtime readiness rejected: {error}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({
            "ok": ok,
            "facts": facts,
            "issues": issues,
        }, ensure_ascii=False, indent=2))
    elif ok:
        print(
            "wave1 h2 runtime readiness ok: "
            f"baseline_rows={facts['baseline_rows']} "
            f"consecutive_success_days={facts['consecutive_success_days']}"
        )
    else:
        print("wave1 h2 runtime readiness failed", file=sys.stderr)
        for issue in issues:
            print(f"  - {issue}", file=sys.stderr)

    return 0 if ok else 2


if __name__ == "__main__":
    sys.exit(main())
