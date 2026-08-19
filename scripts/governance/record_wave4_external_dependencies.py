#!/usr/bin/env python3
"""Record Wave 4 external dependency evidence after real dev/staging checks."""
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
from validate_wave4_external_dependencies import (
    DEFAULT_EVIDENCE,
    REPO_ROOT,
    validate_wave4_external_dependency_payload,
)


STRING_ARGS = (
    "environment",
    "api_doc_ref",
    "auth_doc_ref",
    "error_code_doc_ref",
    "rate_limit_doc_ref",
    "credential_ref",
    "success_report_log_ref",
    "failure_retry_log_ref",
    "audit_event_query_ref",
)
COUNT_ARGS = (
    "reported_events",
    "failed_events_exercised",
)
ENV_VARS = {
    "environment": "WAVE_4_EXTERNAL_ENVIRONMENT",
    "api_doc_ref": "WAVE_4_EXTERNAL_API_DOC_REF",
    "auth_doc_ref": "WAVE_4_EXTERNAL_AUTH_DOC_REF",
    "error_code_doc_ref": "WAVE_4_EXTERNAL_ERROR_CODE_DOC_REF",
    "rate_limit_doc_ref": "WAVE_4_EXTERNAL_RATE_LIMIT_DOC_REF",
    "credential_ref": "WAVE_4_EXTERNAL_CREDENTIAL_REF",
    "success_report_log_ref": "WAVE_4_EXTERNAL_SUCCESS_REPORT_LOG_REF",
    "failure_retry_log_ref": "WAVE_4_EXTERNAL_FAILURE_RETRY_LOG_REF",
    "audit_event_query_ref": "WAVE_4_EXTERNAL_AUDIT_EVENT_QUERY_REF",
    "reported_events": "WAVE_4_EXTERNAL_REPORTED_EVENTS",
    "failed_events_exercised": "WAVE_4_EXTERNAL_FAILED_EVENTS_EXERCISED",
    "pending_replay_queue_verified": "WAVE_4_EXTERNAL_PENDING_REPLAY_QUEUE_VERIFIED",
}
ENV_VAR_OWNERS = {
    "WAVE_4_EXTERNAL_ENVIRONMENT": ("运维 / 部署负责人", "真实 dev/staging 环境"),
    "WAVE_4_EXTERNAL_API_DOC_REF": ("业务方 / 平台对接负责人", "正式接口文档归档"),
    "WAVE_4_EXTERNAL_AUTH_DOC_REF": ("业务方 / 平台对接负责人", "鉴权方式说明归档"),
    "WAVE_4_EXTERNAL_ERROR_CODE_DOC_REF": ("业务方 / 平台对接负责人", "错误码清单归档"),
    "WAVE_4_EXTERNAL_RATE_LIMIT_DOC_REF": ("业务方 / 平台对接负责人", "频率限制说明归档"),
    "WAVE_4_EXTERNAL_CREDENTIAL_REF": ("运维 / 安全负责人", "Vault 凭证引用"),
    "WAVE_4_EXTERNAL_SUCCESS_REPORT_LOG_REF": ("测试执行人 / 后端负责人", "成功上报回执"),
    "WAVE_4_EXTERNAL_FAILURE_RETRY_LOG_REF": ("测试执行人 / 后端负责人", "失败重试日志"),
    "WAVE_4_EXTERNAL_AUDIT_EVENT_QUERY_REF": ("后端 / 数据库操作人", "audit_event 查询"),
    "WAVE_4_EXTERNAL_REPORTED_EVENTS": ("测试执行人 / 后端负责人", "成功上报计数"),
    "WAVE_4_EXTERNAL_FAILED_EVENTS_EXERCISED": ("测试执行人 / 后端负责人", "失败路径计数"),
    "WAVE_4_EXTERNAL_PENDING_REPLAY_QUEUE_VERIFIED": (
        "测试执行人 / 后端负责人",
        "待补报队列验证",
    ),
}
EXPORT_TEMPLATE = """# Wave 4 码上放心外部依赖材料
# 请填写真实 dev/staging 的测试材料引用，不得使用 local/prod/production/mock/fake/stub/example 占位。
export WAVE_4_EXTERNAL_ENVIRONMENT=staging
export WAVE_4_EXTERNAL_API_DOC_REF=
export WAVE_4_EXTERNAL_AUTH_DOC_REF=
export WAVE_4_EXTERNAL_ERROR_CODE_DOC_REF=
export WAVE_4_EXTERNAL_RATE_LIMIT_DOC_REF=
export WAVE_4_EXTERNAL_CREDENTIAL_REF=
export WAVE_4_EXTERNAL_SUCCESS_REPORT_LOG_REF=
export WAVE_4_EXTERNAL_FAILURE_RETRY_LOG_REF=
export WAVE_4_EXTERNAL_AUDIT_EVENT_QUERY_REF=
export WAVE_4_EXTERNAL_REPORTED_EVENTS=1
export WAVE_4_EXTERNAL_FAILED_EVENTS_EXERCISED=1
export WAVE_4_EXTERNAL_PENDING_REPLAY_QUEUE_VERIFIED=true

just wave-4-external-dependencies-readiness --from-env --json
just wave-4-external-dependencies-record --from-env --check-only --json
just wave-4-external-dependencies-record --from-env --json
just wave-4-external-dependencies-validate"""

# ponytail: static preflight scans this file; actual guard lives in shared write_payload.
OVERWRITE_GUARD_MESSAGE = "already exists; pass --force to overwrite"


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
    return _write_payload(
        path,
        payload,
        force=force,
        validate=validate_wave4_external_dependency_payload,
    )


def check_payload(payload: dict[str, Any]) -> tuple[bool, str]:
    return _check_payload(payload, validate=validate_wave4_external_dependency_payload)


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
        bool_args=("pending_replay_queue_verified",),
    )


def missing_from_env_result(
    *,
    args: argparse.Namespace,
    missing_env_vars: list[str],
) -> dict[str, object]:
    return _missing_from_env_result(
        args=args,
        missing_env_vars=missing_env_vars,
        message="缺少 W6.E 外部依赖环境变量；不会写 runtime evidence，W6.E gate remains open",
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
        help="Read W6.E fields from WAVE_4_EXTERNAL_* environment variables.",
    )
    parser.add_argument(
        "--export-template",
        action="store_true",
        help="Print shell template for collecting real external dependency evidence refs.",
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Validate fields, refs, and boundaries without writing evidence.",
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--environment", choices=["dev", "staging"])
    parser.add_argument("--api-doc-ref")
    parser.add_argument("--auth-doc-ref")
    parser.add_argument("--error-code-doc-ref")
    parser.add_argument("--rate-limit-doc-ref")
    parser.add_argument("--credential-ref")
    parser.add_argument("--success-report-log-ref")
    parser.add_argument("--failure-retry-log-ref")
    parser.add_argument("--audit-event-query-ref")
    parser.add_argument("--reported-events", type=int)
    parser.add_argument("--failed-events-exercised", type=int)
    parser.add_argument(
        "--pending-replay-queue-verified",
        action="store_true",
        help="Set only after the real dev/staging replay queue check has passed.",
    )
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
                    "✘ 缺少 W6.E 外部依赖环境变量: "
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
                f"{message}; no external dependency evidence JSON written; "
                "W6.E gate remains open"
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
