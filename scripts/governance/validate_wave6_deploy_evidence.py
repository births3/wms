#!/usr/bin/env python3
"""Validate Wave 6 gray release deployment evidence JSON."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from _wave_evidence_validator import (
    bad_ref,
    blocked_ref_fields,
    blocked_ref_message,
    evidence_execution_status,
    has_environment_token as _has_environment_token,
    placeholder_fields,
    positive_int as _positive_int,
    validate_one as _validate_one,
)

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_EVIDENCE = REPO_ROOT / "docs/retros/wave-6-deploy-evidence.json"

BLOCKED_REF_TOKENS = (
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "local",
    "dev",
    "prod",
    "production",
    "mock",
    "fake",
    "stub",
    "example",
)

REQUIRED_REFS = (
    "release_plan_ref",
    "artifact_ref",
    "canary_config_ref",
    "smoke_gate_ref",
    "observability_dashboard_ref",
    "rollback_drill_log_ref",
    "approval_record_ref",
    "audit_event_query_ref",
)
PLACEHOLDER_TOKENS = ("yyyy", "<", ">", "todo", "tbd", "待填", "待确认")


def _bad_ref(value: str, *, allow_example_refs: bool) -> bool:
    return bad_ref(
        value,
        allow_example_refs=allow_example_refs,
        blocked_ref_tokens=BLOCKED_REF_TOKENS,
    )


def _placeholder_fields(payload: dict[str, object], keys: tuple[str, ...]) -> list[str]:
    return placeholder_fields(payload, keys, placeholder_tokens=PLACEHOLDER_TOKENS)


def validate_wave6_deploy_payload(
    payload: object,
    *,
    allow_example_refs: bool = False,
) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "Wave 6 deploy evidence 顶层必须是 object"

    environment = str(payload.get("environment", "")).lower()
    if environment != "staging":
        return False, "environment 必须是 staging；W6.H 首次试运行灰度发布 gate 不能用 dev/local/prod/production/mock/fake/stub/example 证据关闭"

    deployment_mode = str(payload.get("deployment_mode", ""))
    if deployment_mode not in {"docker-compose", "kubernetes"}:
        return False, "deployment_mode 必须是 docker-compose 或 kubernetes"

    release_version = str(payload.get("release_version", "")).strip()
    if not release_version:
        return False, "release_version 必须记录本次灰度发布版本"

    missing_refs = [key for key in REQUIRED_REFS if not payload.get(key)]
    if missing_refs:
        return False, f"缺少必需证据引用: {', '.join(missing_refs)}"

    placeholder_fields = _placeholder_fields(
        payload,
        ("deployment_mode", "release_version", *REQUIRED_REFS),
    )
    if placeholder_fields:
        return False, f"真实 Wave 6 deploy evidence 不能保留模板占位: {', '.join(placeholder_fields)}"

    bad_ref_fields = blocked_ref_fields(
        payload,
        REQUIRED_REFS,
        is_bad_ref=_bad_ref,
        allow_example_refs=allow_example_refs,
    )
    if bad_ref_fields:
        return False, blocked_ref_message(
            "local/dev/prod/production/mock/fake/stub/example",
            bad_ref_fields,
        )

    missing_environment_refs = [
        key
        for key in REQUIRED_REFS
        if not _has_environment_token(str(payload.get(key, "")), environment)
    ]
    if missing_environment_refs:
        return False, f"证据引用必须包含 environment 标记 {environment}: {', '.join(missing_environment_refs)}"

    required_counts = (
        "canary_stages_exercised",
        "smoke_checks_passed",
        "rollback_drills_exercised",
    )
    invalid_counts = [key for key in required_counts if not _positive_int(payload, key)]
    if invalid_counts:
        return False, f"计数必须 >= 1: {', '.join(invalid_counts)}"

    required_flags = (
        "canary_used",
        "full_release_blocked",
        "rollback_verified",
        "audit_event_verified",
        "dual_approval_recorded",
    )
    invalid_flags = [key for key in required_flags if payload.get(key) is not True]
    if invalid_flags:
        return False, f"布尔证据必须为 true: {', '.join(invalid_flags)}"

    return True, "Wave 6 deploy evidence 内容有效"


def validate_one(path: Path, *, allow_example_refs: bool) -> tuple[bool, str]:
    return _validate_one(
        path,
        allow_example_refs=allow_example_refs,
        validate=validate_wave6_deploy_payload,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-file", default=DEFAULT_EVIDENCE, type=Path)
    parser.add_argument(
        "--allow-example-refs",
        action="store_true",
        help=(
            "Allow refs containing example domain tokens when validating templates; "
            "template placeholders are still rejected."
        ),
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    ok, message = validate_one(
        args.evidence_file,
        allow_example_refs=args.allow_example_refs,
    )

    if args.json:
        print(json.dumps({
            "ok": ok,
            "status": evidence_execution_status(ok, message),
            "path": str(args.evidence_file),
            "message": message,
        }, ensure_ascii=False, indent=2))
    else:
        mark = "✓" if ok else "✘"
        print(f"{mark} {message}")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
