#!/usr/bin/env python3
"""Record Wave 6 gray release deployment evidence."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_wave6_deploy_evidence import DEFAULT_EVIDENCE, validate_wave6_deploy_payload


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
    ok, message = validate_wave6_deploy_payload(payload)
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
    parser.add_argument("--deployment-mode", choices=["docker-compose", "kubernetes"], required=True)
    parser.add_argument("--release-version", required=True)
    parser.add_argument("--release-plan-ref", required=True)
    parser.add_argument("--artifact-ref", required=True)
    parser.add_argument("--canary-config-ref", required=True)
    parser.add_argument("--smoke-gate-ref", required=True)
    parser.add_argument("--observability-dashboard-ref", required=True)
    parser.add_argument("--rollback-drill-log-ref", required=True)
    parser.add_argument("--approval-record-ref", required=True)
    parser.add_argument("--audit-event-query-ref", required=True)
    parser.add_argument("--canary-stages-exercised", type=int, required=True)
    parser.add_argument("--smoke-checks-passed", type=int, required=True)
    parser.add_argument("--rollback-drills-exercised", type=int, required=True)
    parser.add_argument("--canary-used", action="store_true")
    parser.add_argument("--full-release-blocked", action="store_true")
    parser.add_argument("--rollback-verified", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    parser.add_argument("--dual-approval-recorded", action="store_true")
    args = parser.parse_args(argv)

    ok, message = write_payload(args.output, build_payload(args), force=args.force)
    mark = "✓" if ok else "✘"
    stream = sys.stdout if ok else sys.stderr
    print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
