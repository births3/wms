#!/usr/bin/env python3
"""Record Wave 5 packing hardware evidence after real device checks."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_wave5_hardware_evidence import DEFAULT_EVIDENCE, validate_wave5_hardware_payload


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "environment": args.environment,
        "station_code": args.station_code,
        "scale_device_ref": args.scale_device_ref,
        "bluetooth_printer_ref": args.bluetooth_printer_ref,
        "waybill_printer_ref": args.waybill_printer_ref,
        "calibration_record_ref": args.calibration_record_ref,
        "scale_reading_log_ref": args.scale_reading_log_ref,
        "bluetooth_print_log_ref": args.bluetooth_print_log_ref,
        "waybill_print_log_ref": args.waybill_print_log_ref,
        "audit_event_query_ref": args.audit_event_query_ref,
        "scale_readings_recorded": args.scale_readings_recorded,
        "bluetooth_labels_printed": args.bluetooth_labels_printed,
        "waybills_printed": args.waybills_printed,
        "hardware_connected": args.hardware_connected,
        "print_artifacts_reviewed": args.print_artifacts_reviewed,
        "audit_event_verified": args.audit_event_verified,
    }


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    ok, message = validate_wave5_hardware_payload(payload)
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
    parser.add_argument("--station-code", required=True)
    parser.add_argument("--scale-device-ref", required=True)
    parser.add_argument("--bluetooth-printer-ref", required=True)
    parser.add_argument("--waybill-printer-ref", required=True)
    parser.add_argument("--calibration-record-ref", required=True)
    parser.add_argument("--scale-reading-log-ref", required=True)
    parser.add_argument("--bluetooth-print-log-ref", required=True)
    parser.add_argument("--waybill-print-log-ref", required=True)
    parser.add_argument("--audit-event-query-ref", required=True)
    parser.add_argument("--scale-readings-recorded", type=int, required=True)
    parser.add_argument("--bluetooth-labels-printed", type=int, required=True)
    parser.add_argument("--waybills-printed", type=int, required=True)
    parser.add_argument("--hardware-connected", action="store_true")
    parser.add_argument("--print-artifacts-reviewed", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    args = parser.parse_args(argv)

    ok, message = write_payload(args.output, build_payload(args), force=args.force)
    mark = "✓" if ok else "✘"
    stream = sys.stdout if ok else sys.stderr
    print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
