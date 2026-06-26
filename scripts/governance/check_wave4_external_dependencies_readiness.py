#!/usr/bin/env python3
"""Check Wave 4 external dependency readiness without writing evidence."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from record_wave4_external_dependencies import (
    ENV_VAR_OWNERS,
    ENV_VARS,
    apply_from_env,
    build_payload,
    missing_env_var_owners,
    missing_required_args,
)
from validate_wave4_external_dependencies import (
    DEFAULT_EVIDENCE,
    REPO_ROOT,
    validate_wave4_external_dependency_payload,
)

NEXT_COMMANDS = (
    "just wave-4-external-dependencies-record --from-env --check-only --json",
    "just wave-4-external-dependencies-record --from-env --json",
    "just wave-4-external-dependencies-validate",
)
EVIDENCE_ITEM_FIELDS = {
    "api_doc": "api_doc_ref",
    "auth": "auth_doc_ref",
    "error_codes": "error_code_doc_ref",
    "rate_limit": "rate_limit_doc_ref",
    "credential": "credential_ref",
    "success_log": "success_report_log_ref",
    "retry_log": "failure_retry_log_ref",
    "audit_query": "audit_event_query_ref",
}


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def check_payload(args: argparse.Namespace) -> dict[str, Any]:
    payload = build_payload(args)
    ok, message = validate_wave4_external_dependency_payload(
        payload,
        allow_example_refs=False,
    )
    return {
        "ok": ok,
        "message": message,
        "schema_version": 1,
        "mode": "readiness",
        "environment": args.environment,
        "platform": "码上放心",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": display_path(args.evidence_file),
        "evidence_scope": {
            "platform_source": "码上放心",
            "environment": args.environment,
            "scope_verified": ok,
            "internal_api_rejected": True,
        },
        "evidence_items": evidence_items(args),
        "proof": {
            "success_case_count": args.reported_events,
            "failure_case_count": args.failed_events_exercised,
            "pending_replay_queue_verified": args.pending_replay_queue_verified,
        },
        "validation": {
            "allow_example_refs": False,
            "blocked_tokens_checked": True,
            "placeholder_fields_checked": True,
            "environment_token_checked": True,
            "internal_wms_trace_api_rejected": True,
        },
        "next_commands": list(NEXT_COMMANDS),
    }


def evidence_items(args: argparse.Namespace) -> dict[str, dict[str, object]]:
    items: dict[str, dict[str, object]] = {}
    for item_name, field in EVIDENCE_ITEM_FIELDS.items():
        env_var = ENV_VARS[field]
        owner, requirement = ENV_VAR_OWNERS[env_var]
        ref = str(getattr(args, field, "") or "")
        items[item_name] = {
            "field": field,
            "env_var": env_var,
            "owner": owner,
            "evidence_requirement": requirement,
            "status": "provided" if ref.strip() else "missing",
            "ref": ref,
        }
    return items


def add_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--evidence-file", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument(
        "--from-env",
        action="store_true",
        help="Read W6.E fields from WAVE_4_EXTERNAL_* environment variables.",
    )
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
    parser.add_argument("--json", action="store_true")


def missing_from_env_result(
    args: argparse.Namespace,
    missing_env_vars: list[str],
) -> dict[str, Any]:
    return {
        "ok": False,
        "message": "缺少 W6.E 外部依赖环境变量；readiness 不写 runtime evidence，不能关闭 W6.E gate",
        "schema_version": 1,
        "mode": "readiness",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": display_path(args.evidence_file),
        "missing_env_vars": missing_env_vars,
        "missing_env_var_owners": missing_env_var_owners(missing_env_vars),
        "next_commands": list(NEXT_COMMANDS),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    add_arguments(parser)
    args = parser.parse_args(argv)

    if args.from_env:
        missing_env_vars = apply_from_env(args)
        if missing_env_vars:
            result = missing_from_env_result(args, missing_env_vars)
            if args.json:
                print(json.dumps(result, ensure_ascii=False, indent=2))
            else:
                print(f"✘ {result['message']}", file=sys.stderr)
                for env_var in missing_env_vars:
                    print(f"missing env: {env_var}", file=sys.stderr)
            return 1

    missing = missing_required_args(args)
    if missing:
        parser.error(f"the following arguments are required: {', '.join(missing)}")

    result = check_payload(args)
    ok = bool(result["ok"])

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        mark = "✓" if ok else "✘"
        print(f"{mark} {result['message']}")
        print("readiness only: 不写 runtime evidence，不能关闭 gate")
        print(f"evidence file: {result['evidence_file']}")
        for command in result["next_commands"]:
            print(f"next: {command}")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
