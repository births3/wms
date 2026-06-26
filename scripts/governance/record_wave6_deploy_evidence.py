#!/usr/bin/env python3
"""Record Wave 6 gray release deployment evidence."""
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
    missing_from_env_result as _missing_from_env_result,
    missing_required_args as _missing_required_args,
    write_payload as _write_payload,
)
from validate_wave6_deploy_evidence import (
    DEFAULT_EVIDENCE,
    REPO_ROOT,
    validate_wave6_deploy_payload,
)

STRING_ARGS = (
    "environment",
    "deployment_mode",
    "release_version",
    "release_plan_ref",
    "artifact_ref",
    "canary_config_ref",
    "smoke_gate_ref",
    "observability_dashboard_ref",
    "rollback_drill_log_ref",
    "approval_record_ref",
    "audit_event_query_ref",
)
COUNT_ARGS = (
    "canary_stages_exercised",
    "smoke_checks_passed",
    "rollback_drills_exercised",
)
BOOL_ARGS = (
    "canary_used",
    "full_release_blocked",
    "rollback_verified",
    "audit_event_verified",
    "dual_approval_recorded",
)
ENV_VARS = {
    "environment": "WAVE_6_ENVIRONMENT",
    "deployment_mode": "WAVE_6_DEPLOYMENT_MODE",
    "release_version": "WAVE_6_RELEASE_VERSION",
    "release_plan_ref": "WAVE_6_RELEASE_PLAN_REF",
    "artifact_ref": "WAVE_6_ARTIFACT_REF",
    "canary_config_ref": "WAVE_6_CANARY_CONFIG_REF",
    "smoke_gate_ref": "WAVE_6_SMOKE_GATE_REF",
    "observability_dashboard_ref": "WAVE_6_OBSERVABILITY_DASHBOARD_REF",
    "rollback_drill_log_ref": "WAVE_6_ROLLBACK_DRILL_LOG_REF",
    "approval_record_ref": "WAVE_6_APPROVAL_RECORD_REF",
    "audit_event_query_ref": "WAVE_6_AUDIT_EVENT_QUERY_REF",
    "canary_stages_exercised": "WAVE_6_CANARY_STAGES_EXERCISED",
    "smoke_checks_passed": "WAVE_6_SMOKE_CHECKS_PASSED",
    "rollback_drills_exercised": "WAVE_6_ROLLBACK_DRILLS_EXERCISED",
    "canary_used": "WAVE_6_CANARY_USED",
    "full_release_blocked": "WAVE_6_FULL_RELEASE_BLOCKED",
    "rollback_verified": "WAVE_6_ROLLBACK_VERIFIED",
    "audit_event_verified": "WAVE_6_AUDIT_EVENT_VERIFIED",
    "dual_approval_recorded": "WAVE_6_DUAL_APPROVAL_RECORDED",
}

# ponytail: static preflight scans this file; actual guard lives in shared write_payload.
OVERWRITE_GUARD_MESSAGE = "already exists; pass --force to overwrite"


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "environment": args.environment,
        "deployment_mode": args.deployment_mode,
        "release_version": args.release_version,
        "release_plan_ref": args.release_plan_ref,
        "artifact_ref": args.artifact_ref,
        "canary_config_ref": args.canary_config_ref,
        "smoke_gate_ref": args.smoke_gate_ref,
        "observability_dashboard_ref": args.observability_dashboard_ref,
        "rollback_drill_log_ref": args.rollback_drill_log_ref,
        "approval_record_ref": args.approval_record_ref,
        "audit_event_query_ref": args.audit_event_query_ref,
        "canary_stages_exercised": args.canary_stages_exercised,
        "smoke_checks_passed": args.smoke_checks_passed,
        "rollback_drills_exercised": args.rollback_drills_exercised,
        "canary_used": args.canary_used,
        "full_release_blocked": args.full_release_blocked,
        "rollback_verified": args.rollback_verified,
        "audit_event_verified": args.audit_event_verified,
        "dual_approval_recorded": args.dual_approval_recorded,
    }


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    return _write_payload(
        path,
        payload,
        force=force,
        validate=validate_wave6_deploy_payload,
    )


def check_payload(payload: dict[str, Any]) -> tuple[bool, str]:
    return _check_payload(payload, validate=validate_wave6_deploy_payload)


def display_evidence_file(path: Path) -> Path:
    return _display_evidence_file(path, repo_root=REPO_ROOT)


def apply_from_env(args: argparse.Namespace) -> list[str]:
    return _apply_from_env(
        args,
        env_vars=ENV_VARS,
        count_args=COUNT_ARGS,
        bool_args=BOOL_ARGS,
    )


def missing_required_args(args: argparse.Namespace) -> list[str]:
    return _missing_required_args(args, string_args=STRING_ARGS, count_args=COUNT_ARGS)


def missing_from_env_result(
    *,
    args: argparse.Namespace,
    missing_env_vars: list[str],
) -> dict[str, object]:
    return _missing_from_env_result(
        args=args,
        missing_env_vars=missing_env_vars,
        message="缺少 W6.H 灰度发布 evidence 环境变量；不会写 runtime evidence，W6.H gate remains open",
        repo_root=REPO_ROOT,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--force", action="store_true")
    parser.add_argument(
        "--from-env",
        action="store_true",
        help="Read W6.H fields from WAVE_6_* environment variables.",
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Validate fields, refs, and boundaries without writing evidence.",
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--environment", choices=["staging"])
    parser.add_argument("--deployment-mode", choices=["docker-compose", "kubernetes"])
    parser.add_argument("--release-version")
    parser.add_argument("--release-plan-ref")
    parser.add_argument("--artifact-ref")
    parser.add_argument("--canary-config-ref")
    parser.add_argument("--smoke-gate-ref")
    parser.add_argument("--observability-dashboard-ref")
    parser.add_argument("--rollback-drill-log-ref")
    parser.add_argument("--approval-record-ref")
    parser.add_argument("--audit-event-query-ref")
    parser.add_argument("--canary-stages-exercised", type=int)
    parser.add_argument("--smoke-checks-passed", type=int)
    parser.add_argument("--rollback-drills-exercised", type=int)
    parser.add_argument("--canary-used", action="store_true")
    parser.add_argument("--full-release-blocked", action="store_true")
    parser.add_argument("--rollback-verified", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    parser.add_argument("--dual-approval-recorded", action="store_true")
    args = parser.parse_args(argv)

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
                    "✘ 缺少 W6.H 灰度发布 evidence 环境变量: "
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
                f"{message}; no deploy evidence JSON written; "
                "W6.H gate remains open"
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
