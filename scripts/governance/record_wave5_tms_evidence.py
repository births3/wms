#!/usr/bin/env python3
"""Record Wave 5 TMS+ evidence after real dev/staging integration checks."""
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
from validate_wave5_tms_evidence import (
    DEFAULT_EVIDENCE,
    REPO_ROOT,
    validate_wave5_tms_payload,
)

STRING_ARGS = (
    "environment",
    "tms_system_ref",
    "dispatch_push_log_ref",
    "callback_log_ref",
    "failure_retry_log_ref",
    "audit_event_query_ref",
    "credential_ref",
)
COUNT_ARGS = (
    "dispatches_received",
    "callbacks_received",
    "failed_callbacks_exercised",
)
BOOL_ARGS = (
    "retry_succeeded",
    "audit_event_verified",
)
ENV_VARS = {
    "environment": "WAVE_5_TMS_ENVIRONMENT",
    "tms_system_ref": "WAVE_5_TMS_SYSTEM_REF",
    "dispatch_push_log_ref": "WAVE_5_TMS_DISPATCH_PUSH_LOG_REF",
    "callback_log_ref": "WAVE_5_TMS_CALLBACK_LOG_REF",
    "failure_retry_log_ref": "WAVE_5_TMS_FAILURE_RETRY_LOG_REF",
    "audit_event_query_ref": "WAVE_5_TMS_AUDIT_EVENT_QUERY_REF",
    "credential_ref": "WAVE_5_TMS_CREDENTIAL_REF",
    "dispatches_received": "WAVE_5_TMS_DISPATCHES_RECEIVED",
    "callbacks_received": "WAVE_5_TMS_CALLBACKS_RECEIVED",
    "failed_callbacks_exercised": "WAVE_5_TMS_FAILED_CALLBACKS_EXERCISED",
    "retry_succeeded": "WAVE_5_TMS_RETRY_SUCCEEDED",
    "audit_event_verified": "WAVE_5_TMS_AUDIT_EVENT_VERIFIED",
}
ENV_VAR_OWNERS = {
    "WAVE_5_TMS_ENVIRONMENT": ("运维 / 部署负责人", "真实 dev/staging 环境"),
    "WAVE_5_TMS_SYSTEM_REF": ("外部 TMS 对接方 / 平台对接负责人", "TMS 系统引用"),
    "WAVE_5_TMS_DISPATCH_PUSH_LOG_REF": ("联调执行人 / TMS 对接人", "调度推送日志"),
    "WAVE_5_TMS_CALLBACK_LOG_REF": ("联调执行人 / TMS 对接人", "回调日志"),
    "WAVE_5_TMS_FAILURE_RETRY_LOG_REF": ("联调执行人 / TMS 对接人", "失败重试日志"),
    "WAVE_5_TMS_AUDIT_EVENT_QUERY_REF": ("后端 / 数据库操作人", "audit_event 查询"),
    "WAVE_5_TMS_CREDENTIAL_REF": ("运维 / 安全负责人", "Vault 凭证引用"),
    "WAVE_5_TMS_DISPATCHES_RECEIVED": ("联调执行人 / TMS 对接人", "调度接收计数"),
    "WAVE_5_TMS_CALLBACKS_RECEIVED": ("联调执行人 / TMS 对接人", "回调接收计数"),
    "WAVE_5_TMS_FAILED_CALLBACKS_EXERCISED": ("联调执行人 / TMS 对接人", "失败回调演练计数"),
    "WAVE_5_TMS_RETRY_SUCCEEDED": ("联调执行人 / TMS 对接人", "重试成功确认"),
    "WAVE_5_TMS_AUDIT_EVENT_VERIFIED": ("后端 / 数据库操作人", "审计事件复核"),
}
EXPORT_TEMPLATE = """# Wave 5 TMS+ evidence materials
# Fill with real dev/staging TMS evidence refs. Do not use local/prod/production/mock/fake/stub/example refs.
export WAVE_5_TMS_ENVIRONMENT=staging
export WAVE_5_TMS_SYSTEM_REF=
export WAVE_5_TMS_DISPATCH_PUSH_LOG_REF=
export WAVE_5_TMS_CALLBACK_LOG_REF=
export WAVE_5_TMS_FAILURE_RETRY_LOG_REF=
export WAVE_5_TMS_AUDIT_EVENT_QUERY_REF=
export WAVE_5_TMS_CREDENTIAL_REF=
export WAVE_5_TMS_DISPATCHES_RECEIVED=1
export WAVE_5_TMS_CALLBACKS_RECEIVED=1
export WAVE_5_TMS_FAILED_CALLBACKS_EXERCISED=1
export WAVE_5_TMS_RETRY_SUCCEEDED=true
export WAVE_5_TMS_AUDIT_EVENT_VERIFIED=true

just wave-5-tms-materials --from-env --json
just wave-5-tms-evidence-record --from-env --check-only --json
just wave-5-tms-evidence-record --from-env --json
just wave-5-tms-evidence-validate
"""

# ponytail: static preflight scans this file; actual guard lives in shared write_payload.
OVERWRITE_GUARD_MESSAGE = "already exists; pass --force to overwrite"


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
    return _write_payload(
        path,
        payload,
        force=force,
        validate=validate_wave5_tms_payload,
    )


def check_payload(payload: dict[str, Any]) -> tuple[bool, str]:
    return _check_payload(payload, validate=validate_wave5_tms_payload)


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
        message="缺少 W6.G TMS evidence 环境变量；不会写 runtime evidence，W6.G gate remains open",
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
        help="Read W6.G fields from WAVE_5_TMS_* environment variables.",
    )
    parser.add_argument(
        "--export-template",
        action="store_true",
        help="Print a shell template for collecting real TMS evidence refs.",
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Validate fields, refs, and boundaries without writing evidence.",
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--environment", choices=["dev", "staging"])
    parser.add_argument("--tms-system-ref")
    parser.add_argument("--dispatch-push-log-ref")
    parser.add_argument("--callback-log-ref")
    parser.add_argument("--failure-retry-log-ref")
    parser.add_argument("--audit-event-query-ref")
    parser.add_argument("--credential-ref")
    parser.add_argument("--dispatches-received", type=int)
    parser.add_argument("--callbacks-received", type=int)
    parser.add_argument("--failed-callbacks-exercised", type=int)
    parser.add_argument("--retry-succeeded", action="store_true")
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
                    "✘ 缺少 W6.G TMS evidence 环境变量: "
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
                f"{message}; no TMS call attempted; "
                "no evidence JSON written; W6.G gate remains open"
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
