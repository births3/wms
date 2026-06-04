#!/usr/bin/env python3
"""Record Wave 3 real PDA and L7 runtime evidence."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_wave3_pda_runtime_evidence import (
    DEFAULT_EVIDENCE,
    validate_wave3_pda_runtime_payload,
)


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "environment": args.environment,
        "pda_model": args.pda_model,
        "android_version": args.android_version,
        "scan_input_method": args.scan_input_method,
        "pda_device_ref": args.pda_device_ref,
        "spike005_result_ref": args.spike005_result_ref,
        "m2_scan_log_ref": args.m2_scan_log_ref,
        "m3_scan_log_ref": args.m3_scan_log_ref,
        "offline_replay_log_ref": args.offline_replay_log_ref,
        "idempotency_replay_log_ref": args.idempotency_replay_log_ref,
        "audit_event_query_ref": args.audit_event_query_ref,
        "l7_run_ref": args.l7_run_ref,
        "usability_review_ref": args.usability_review_ref,
        "barcode_samples_scanned": args.barcode_samples_scanned,
        "m2_operations_exercised": args.m2_operations_exercised,
        "m3_operations_exercised": args.m3_operations_exercised,
        "offline_replays_exercised": args.offline_replays_exercised,
        "idempotency_replays_exercised": args.idempotency_replays_exercised,
        "real_pda_used": args.real_pda_used,
        "physical_scan_key_verified": args.physical_scan_key_verified,
        "dev_or_staging_service_verified": args.dev_or_staging_service_verified,
        "audit_event_verified": args.audit_event_verified,
        "l7_review_completed": args.l7_review_completed,
        "usability_review_completed": args.usability_review_completed,
    }


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    ok, message = validate_wave3_pda_runtime_payload(payload)
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
    parser.add_argument("--pda-model", required=True)
    parser.add_argument("--android-version", required=True)
    parser.add_argument("--scan-input-method", required=True)
    parser.add_argument("--pda-device-ref", required=True)
    parser.add_argument("--spike005-result-ref", required=True)
    parser.add_argument("--m2-scan-log-ref", required=True)
    parser.add_argument("--m3-scan-log-ref", required=True)
    parser.add_argument("--offline-replay-log-ref", required=True)
    parser.add_argument("--idempotency-replay-log-ref", required=True)
    parser.add_argument("--audit-event-query-ref", required=True)
    parser.add_argument("--l7-run-ref", required=True)
    parser.add_argument("--usability-review-ref", required=True)
    parser.add_argument("--barcode-samples-scanned", type=int, required=True)
    parser.add_argument("--m2-operations-exercised", type=int, required=True)
    parser.add_argument("--m3-operations-exercised", type=int, required=True)
    parser.add_argument("--offline-replays-exercised", type=int, required=True)
    parser.add_argument("--idempotency-replays-exercised", type=int, required=True)
    parser.add_argument("--real-pda-used", action="store_true")
    parser.add_argument("--physical-scan-key-verified", action="store_true")
    parser.add_argument("--dev-or-staging-service-verified", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    parser.add_argument("--l7-review-completed", action="store_true")
    parser.add_argument("--usability-review-completed", action="store_true")
    args = parser.parse_args(argv)

    ok, message = write_payload(args.output, build_payload(args), force=args.force)
    mark = "✓" if ok else "✘"
    stream = sys.stdout if ok else sys.stderr
    print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
