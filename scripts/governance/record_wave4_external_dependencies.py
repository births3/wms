#!/usr/bin/env python3
"""Record Wave 4 external dependency evidence after real dev/staging checks."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_wave4_external_dependencies import (
    DEFAULT_EVIDENCE,
    validate_wave4_external_dependency_payload,
)


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "environment": args.environment,
        "platform": "码上放心",
        "api_doc_ref": args.api_doc_ref,
        "auth_doc_ref": args.auth_doc_ref,
        "error_code_doc_ref": args.error_code_doc_ref,
        "rate_limit_doc_ref": args.rate_limit_doc_ref,
        "credential_ref": args.credential_ref,
        "success_report_log_ref": args.success_report_log_ref,
        "failure_retry_log_ref": args.failure_retry_log_ref,
        "audit_event_query_ref": args.audit_event_query_ref,
        "reported_events": args.reported_events,
        "failed_events_exercised": args.failed_events_exercised,
        "pending_replay_queue_verified": args.pending_replay_queue_verified,
    }


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    ok, message = validate_wave4_external_dependency_payload(payload)
    if not ok:
        return False, message

    if path.exists() and not force:
        return False, f"{path} already exists; pass --force to overwrite"

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return True, f"wrote {path}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--environment", choices=["dev", "staging"], required=True)
    parser.add_argument("--api-doc-ref", required=True)
    parser.add_argument("--auth-doc-ref", required=True)
    parser.add_argument("--error-code-doc-ref", required=True)
    parser.add_argument("--rate-limit-doc-ref", required=True)
    parser.add_argument("--credential-ref", required=True)
    parser.add_argument("--success-report-log-ref", required=True)
    parser.add_argument("--failure-retry-log-ref", required=True)
    parser.add_argument("--audit-event-query-ref", required=True)
    parser.add_argument("--reported-events", type=int, required=True)
    parser.add_argument("--failed-events-exercised", type=int, required=True)
    parser.add_argument(
        "--pending-replay-queue-verified",
        action="store_true",
        help="Set only after the real dev/staging replay queue check has passed.",
    )
    args = parser.parse_args(argv)

    ok, message = write_payload(args.output, build_payload(args), force=args.force)
    mark = "✓" if ok else "✘"
    stream = sys.stdout if ok else sys.stderr
    print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
