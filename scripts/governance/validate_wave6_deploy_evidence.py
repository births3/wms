#!/usr/bin/env python3
"""Validate Wave 6 gray release deployment evidence JSON."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_EVIDENCE = REPO_ROOT / "docs/retros/wave-6-deploy-evidence.json"

BLOCKED_REF_TOKENS = (
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
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


def read_json(path: Path) -> tuple[object | None, str | None]:
    if not path.exists():
        return None, f"missing file: {path}"
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except json.JSONDecodeError as error:
        return None, f"invalid JSON: {path}: {error}"


def _bad_ref(value: str, *, allow_example_refs: bool) -> bool:
    lowered = value.lower()
    blocked = BLOCKED_REF_TOKENS if not allow_example_refs else BLOCKED_REF_TOKENS[:-1]
    return any(token in lowered for token in blocked)


def _positive_int(payload: dict[str, object], key: str) -> bool:
    value = payload.get(key)
    return isinstance(value, int) and value >= 1


def validate_wave6_deploy_payload(
    payload: object,
    *,
    allow_example_refs: bool = False,
) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "Wave 6 deploy evidence 顶层必须是 object"

    environment = str(payload.get("environment", "")).lower()
    if environment not in {"dev", "staging"}:
        return False, "environment 必须是真实 dev 或 staging，不能是 local/prod/example"

    deployment_mode = str(payload.get("deployment_mode", ""))
    if deployment_mode not in {"docker-compose", "kubernetes"}:
        return False, "deployment_mode 必须是 docker-compose 或 kubernetes"

    release_version = str(payload.get("release_version", "")).strip()
    if not release_version:
        return False, "release_version 必须记录本次灰度发布版本"

    missing_refs = [key for key in REQUIRED_REFS if not payload.get(key)]
    if missing_refs:
        return False, f"缺少必需证据引用: {', '.join(missing_refs)}"

    ref_values = [str(payload.get(key, "")) for key in REQUIRED_REFS]
    if any(_bad_ref(value, allow_example_refs=allow_example_refs) for value in ref_values):
        return False, "证据引用不能指向 local/prod/mock/fake/stub/example 边界"

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
    payload, error = read_json(path)
    if error:
        return False, error
    ok, message = validate_wave6_deploy_payload(
        payload,
        allow_example_refs=allow_example_refs,
    )
    return ok, f"{path}: {message}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-file", default=DEFAULT_EVIDENCE, type=Path)
    parser.add_argument(
        "--allow-example-refs",
        action="store_true",
        help="Allow example placeholder references; only use for validating templates.",
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
