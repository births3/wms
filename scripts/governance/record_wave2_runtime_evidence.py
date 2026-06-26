#!/usr/bin/env python3
"""Record Wave 2 config-center Feature Flag runtime evidence."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from report_wave2_completion import DEFAULT_RUNTIME_EVIDENCE, validate_wave2_runtime_payload


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "environment": args.environment,
        "service_url": args.service_url,
        "source_switched_to": "config_center",
        "migrated_count": args.migrated_count,
        "reconcile": {
            "matched": args.reconcile_matched,
            "missing_in_config_center": [],
            "mismatched": [],
        },
        "business_smoke": {
            "path": args.business_smoke_path,
            "enabled_flag": args.business_smoke_enabled_flag,
            "success_status": args.business_smoke_success_status,
            "fail_closed_error_code": args.business_smoke_fail_closed_error_code,
        },
        "smoke_log_ref": args.smoke_log_ref,
        "reconcile_log_ref": args.reconcile_log_ref,
        "archive_ref": args.archive_ref,
    }


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    ok, message = validate_wave2_runtime_payload(payload)
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
    parser.add_argument("--output", type=Path, default=DEFAULT_RUNTIME_EVIDENCE)
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--environment", choices=["dev", "staging"], required=True)
    parser.add_argument("--service-url", required=True)
    parser.add_argument("--migrated-count", type=int, required=True)
    parser.add_argument("--reconcile-matched", type=int, required=True)
    parser.add_argument("--business-smoke-path", default="/api/v1/inventory/batches")
    parser.add_argument(
        "--business-smoke-enabled-flag",
        default="m3_inventory_batches_config_center_smoke",
    )
    parser.add_argument("--business-smoke-success-status", type=int, default=200)
    parser.add_argument(
        "--business-smoke-fail-closed-error-code",
        default="M1_CONFIG_FLAG_MISSING",
    )
    parser.add_argument("--smoke-log-ref", required=True)
    parser.add_argument("--reconcile-log-ref", required=True)
    parser.add_argument("--archive-ref", required=True)
    args = parser.parse_args(argv)

    ok, message = write_payload(args.output, build_payload(args), force=args.force)
    mark = "✓" if ok else "✘"
    stream = sys.stdout if ok else sys.stderr
    print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
