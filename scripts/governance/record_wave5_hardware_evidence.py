#!/usr/bin/env python3
"""Record Wave 5 packing hardware evidence after real device checks."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from _wave_evidence_recorder import (
    apply_from_env as _apply_from_env,
    check_only_result,
    check_payload as _check_payload,
    display_evidence_file as _display_evidence_file,
    missing_env_var_owners as _missing_env_var_owners,
    missing_from_env_result as _missing_from_env_result,
    missing_required_args as _missing_required_args,
    write_payload as _write_payload,
)
from validate_wave5_hardware_evidence import (
    DEFAULT_EVIDENCE,
    REPO_ROOT,
    validate_wave5_hardware_payload,
)

STRING_ARGS = (
    "environment",
    "station_code",
    "scale_device_ref",
    "bluetooth_printer_ref",
    "waybill_printer_ref",
    "calibration_record_ref",
    "scale_reading_log_ref",
    "bluetooth_print_log_ref",
    "waybill_print_log_ref",
    "audit_event_query_ref",
)
COUNT_ARGS = (
    "scale_readings_recorded",
    "bluetooth_labels_printed",
    "waybills_printed",
)
BOOL_ARGS = (
    "hardware_connected",
    "print_artifacts_reviewed",
    "audit_event_verified",
)
ENV_VARS = {
    "environment": "WAVE_5_ENVIRONMENT",
    "station_code": "WAVE_5_STATION_CODE",
    "scale_device_ref": "WAVE_5_SCALE_DEVICE_REF",
    "bluetooth_printer_ref": "WAVE_5_BLUETOOTH_PRINTER_REF",
    "waybill_printer_ref": "WAVE_5_WAYBILL_PRINTER_REF",
    "calibration_record_ref": "WAVE_5_CALIBRATION_RECORD_REF",
    "scale_reading_log_ref": "WAVE_5_SCALE_READING_LOG_REF",
    "bluetooth_print_log_ref": "WAVE_5_BLUETOOTH_PRINT_LOG_REF",
    "waybill_print_log_ref": "WAVE_5_WAYBILL_PRINT_LOG_REF",
    "audit_event_query_ref": "WAVE_5_AUDIT_EVENT_QUERY_REF",
    "scale_readings_recorded": "WAVE_5_SCALE_READINGS_RECORDED",
    "bluetooth_labels_printed": "WAVE_5_BLUETOOTH_LABELS_PRINTED",
    "waybills_printed": "WAVE_5_WAYBILLS_PRINTED",
    "hardware_connected": "WAVE_5_HARDWARE_CONNECTED",
    "print_artifacts_reviewed": "WAVE_5_PRINT_ARTIFACTS_REVIEWED",
    "audit_event_verified": "WAVE_5_AUDIT_EVENT_VERIFIED",
}
ENV_VAR_OWNERS = {
    "WAVE_5_ENVIRONMENT": ("运维 / 部署负责人", "真实 dev/staging 环境"),
    "WAVE_5_STATION_CODE": ("现场设备负责人 / 包装工位负责人", "真实包装工位"),
    "WAVE_5_SCALE_DEVICE_REF": ("现场设备负责人", "电子秤资产引用"),
    "WAVE_5_BLUETOOTH_PRINTER_REF": ("现场设备负责人", "蓝牙打印机资产引用"),
    "WAVE_5_WAYBILL_PRINTER_REF": ("现场设备负责人", "面单打印机资产引用"),
    "WAVE_5_CALIBRATION_RECORD_REF": ("现场设备负责人", "校准记录"),
    "WAVE_5_SCALE_READING_LOG_REF": ("联调执行人 / 测试负责人", "称重日志"),
    "WAVE_5_BLUETOOTH_PRINT_LOG_REF": ("联调执行人 / 测试负责人", "蓝牙打印日志"),
    "WAVE_5_WAYBILL_PRINT_LOG_REF": ("联调执行人 / 测试负责人", "面单打印日志"),
    "WAVE_5_AUDIT_EVENT_QUERY_REF": ("后端 / 数据库操作人", "audit_event 查询"),
    "WAVE_5_SCALE_READINGS_RECORDED": ("联调执行人 / 测试负责人", "称重计数"),
    "WAVE_5_BLUETOOTH_LABELS_PRINTED": ("联调执行人 / 测试负责人", "蓝牙标签打印计数"),
    "WAVE_5_WAYBILLS_PRINTED": ("联调执行人 / 测试负责人", "面单打印计数"),
    "WAVE_5_HARDWARE_CONNECTED": ("现场设备负责人", "硬件连接确认"),
    "WAVE_5_PRINT_ARTIFACTS_REVIEWED": ("联调执行人 / 测试负责人", "打印产物复核"),
    "WAVE_5_AUDIT_EVENT_VERIFIED": ("后端 / 数据库操作人", "审计事件复核"),
}
EXPORT_TEMPLATE = """# Wave 5 M-PK hardware evidence materials
# Fill with real dev/staging hardware evidence refs. Do not use local/prod/production/mock/fake/stub/example refs.
export WAVE_5_ENVIRONMENT=staging
export WAVE_5_STATION_CODE=
export WAVE_5_SCALE_DEVICE_REF=
export WAVE_5_BLUETOOTH_PRINTER_REF=
export WAVE_5_WAYBILL_PRINTER_REF=
export WAVE_5_CALIBRATION_RECORD_REF=
export WAVE_5_SCALE_READING_LOG_REF=
export WAVE_5_BLUETOOTH_PRINT_LOG_REF=
export WAVE_5_WAYBILL_PRINT_LOG_REF=
export WAVE_5_AUDIT_EVENT_QUERY_REF=
export WAVE_5_SCALE_READINGS_RECORDED=1
export WAVE_5_BLUETOOTH_LABELS_PRINTED=1
export WAVE_5_WAYBILLS_PRINTED=1
export WAVE_5_HARDWARE_CONNECTED=true
export WAVE_5_PRINT_ARTIFACTS_REVIEWED=true
export WAVE_5_AUDIT_EVENT_VERIFIED=true

just wave-5-hardware-materials --from-env --json
just wave-5-hardware-evidence-record --from-env --check-only --json
just wave-5-hardware-evidence-record --from-env --json
just wave-5-hardware-evidence-validate
"""

# ponytail: static preflight scans this file; actual guard lives in shared write_payload.
OVERWRITE_GUARD_MESSAGE = "already exists; pass --force to overwrite"


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
    return _write_payload(
        path,
        payload,
        force=force,
        validate=validate_wave5_hardware_payload,
    )


def check_payload(payload: dict[str, Any]) -> tuple[bool, str]:
    return _check_payload(payload, validate=validate_wave5_hardware_payload)


def missing_required_args(args: argparse.Namespace) -> list[str]:
    return _missing_required_args(args, string_args=STRING_ARGS, count_args=COUNT_ARGS)


def missing_env_var_owners(missing_env_vars: list[str]) -> list[dict[str, str]]:
    return _missing_env_var_owners(missing_env_vars, ENV_VAR_OWNERS)


def display_evidence_file(path: Path) -> Path:
    return _display_evidence_file(path, repo_root=REPO_ROOT)


def apply_from_env(args: argparse.Namespace) -> list[str]:
    return _apply_from_env(
        args,
        env_vars=ENV_VARS,
        count_args=COUNT_ARGS,
        bool_args=BOOL_ARGS,
    )


def missing_from_env_result(
    *,
    args: argparse.Namespace,
    missing_env_vars: list[str],
) -> dict[str, object]:
    return _missing_from_env_result(
        args=args,
        missing_env_vars=missing_env_vars,
        message="缺少 W6.F 硬件 evidence 环境变量；不会写 runtime evidence，W6.F gate remains open",
        repo_root=REPO_ROOT,
        owner_map=ENV_VAR_OWNERS,
    )


def print_export_template() -> None:
    print(EXPORT_TEMPLATE.rstrip())


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--force", action="store_true")
    parser.add_argument(
        "--from-env",
        action="store_true",
        help="Read W6.F fields from WAVE_5_* environment variables.",
    )
    parser.add_argument(
        "--export-template",
        action="store_true",
        help="Print a shell template for collecting real hardware evidence refs.",
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Validate fields, refs, and boundaries without writing evidence.",
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--environment", choices=["dev", "staging"])
    parser.add_argument("--station-code")
    parser.add_argument("--scale-device-ref")
    parser.add_argument("--bluetooth-printer-ref")
    parser.add_argument("--waybill-printer-ref")
    parser.add_argument("--calibration-record-ref")
    parser.add_argument("--scale-reading-log-ref")
    parser.add_argument("--bluetooth-print-log-ref")
    parser.add_argument("--waybill-print-log-ref")
    parser.add_argument("--audit-event-query-ref")
    parser.add_argument("--scale-readings-recorded", type=int)
    parser.add_argument("--bluetooth-labels-printed", type=int)
    parser.add_argument("--waybills-printed", type=int)
    parser.add_argument("--hardware-connected", action="store_true")
    parser.add_argument("--print-artifacts-reviewed", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    args = parser.parse_args(argv)

    if args.export_template:
        print_export_template()
        return 0

    if args.from_env:
        missing_env_vars = apply_from_env(args)
        if missing_env_vars:
            if args.json:
                print(
                    json.dumps(
                        missing_from_env_result(
                            args=args,
                            missing_env_vars=missing_env_vars,
                        ),
                        ensure_ascii=False,
                        indent=2,
                    ),
                )
            else:
                print(
                    "✘ 缺少 W6.F 硬件 evidence 环境变量: "
                    + ", ".join(missing_env_vars),
                    file=sys.stderr,
                )
            return 1

    missing = missing_required_args(args)
    if missing:
        parser.error(f"the following arguments are required: {', '.join(missing)}")

    payload = build_payload(args)
    if args.check_only:
        ok, message = check_payload(payload)
        if ok:
            message = (
                f"{message}; no hardware connection attempted; "
                "no evidence JSON written; W6.F gate remains open"
            )
    else:
        ok, message = write_payload(args.output, payload, force=args.force)
    if args.json and args.check_only:
        print(
            json.dumps(
                check_only_result(ok, message, display_evidence_file(args.output)),
                ensure_ascii=False,
                indent=2,
            ),
        )
    else:
        mark = "✓" if ok else "✘"
        stream = sys.stdout if ok else sys.stderr
        print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
