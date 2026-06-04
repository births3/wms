#!/usr/bin/env python3
"""Record Wave 5 TMS+ evidence after real dev/staging integration checks."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_wave5_tms_evidence import DEFAULT_EVIDENCE, validate_wave5_tms_payload


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "environment": args.environment,
        "tms_system_ref": args.tms_system_ref,
        "dispatch_push_log_ref": args.dispatch_push_log_ref,
        "callback_log_ref": args.callback_log_ref,
        "failure_retry_log_ref": args.failure_retry_log_ref,
        "audit_event_query_ref": args.audit_event_query_ref,
        "credential_ref": args.credential_ref,
        "dispatches_received": args.dispatches_received,
        "callbacks_received": args.callbacks_received,
        "failed_callbacks_exercised": args.failed_callbacks_exercised,
        "retry_succeeded": args.retry_succeeded,
        "audit_event_verified": args.audit_event_verified,
    }


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    ok, message = validate_wave5_tms_payload(payload)
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
    parser.add_argument("--tms-system-ref", required=True)
    parser.add_argument("--dispatch-push-log-ref", required=True)
    parser.add_argument("--callback-log-ref", required=True)
    parser.add_argument("--failure-retry-log-ref", required=True)
    parser.add_argument("--audit-event-query-ref", required=True)
    parser.add_argument("--credential-ref", required=True)
    parser.add_argument("--dispatches-received", type=int, required=True)
    parser.add_argument("--callbacks-received", type=int, required=True)
    parser.add_argument("--failed-callbacks-exercised", type=int, required=True)
    parser.add_argument("--retry-succeeded", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    args = parser.parse_args(argv)

    ok, message = write_payload(args.output, build_payload(args), force=args.force)
    mark = "✓" if ok else "✘"
    stream = sys.stdout if ok else sys.stderr
    print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
