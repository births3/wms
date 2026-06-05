#!/usr/bin/env python3
"""Validate Wave 3 real PDA and L7 evidence JSON."""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_EVIDENCE = REPO_ROOT / "docs/retros/wave-3-pda-runtime-evidence.json"

BLOCKED_REF_TOKENS = (
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "prod",
    "production",
    "mock",
    "fake",
    "stub",
    "browser",
    "simulator",
    "example",
)

REQUIRED_REFS = (
    "pda_device_ref",
    "spike005_result_ref",
    "m2_scan_log_ref",
    "m3_scan_log_ref",
    "offline_replay_log_ref",
    "idempotency_replay_log_ref",
    "audit_event_query_ref",
    "l7_run_ref",
    "usability_review_ref",
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


def _has_environment_token(value: str, environment: str) -> bool:
    return re.search(
        rf"(^|[^0-9a-z]){re.escape(environment)}([^0-9a-z]|$)",
        value.lower(),
    ) is not None


def _positive_int(payload: dict[str, object], key: str) -> bool:
    value = payload.get(key)
    return isinstance(value, int) and value >= 1


def validate_wave3_pda_runtime_payload(
    payload: object,
    *,
    allow_example_refs: bool = False,
) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "Wave 3 PDA evidence 顶层必须是 object"

    environment = str(payload.get("environment", "")).lower()
    if environment not in {"dev", "staging"}:
        return False, "environment 必须是真实 dev 或 staging，不能是 local/prod/mock/fake/stub/example"

    for key in ("pda_model", "android_version", "scan_input_method"):
        if not str(payload.get(key, "")).strip():
            return False, f"{key} 必须记录真实 PDA 设备信息"

    scan_input_method = str(payload.get("scan_input_method", "")).lower()
    if _bad_ref(scan_input_method, allow_example_refs=allow_example_refs):
        return False, "scan_input_method 不能是 browser/simulator/mock/fake/stub/example"

    missing_refs = [key for key in REQUIRED_REFS if not payload.get(key)]
    if missing_refs:
        return False, f"缺少必需证据引用: {', '.join(missing_refs)}"

    ref_values = [str(payload.get(key, "")) for key in REQUIRED_REFS]
    if any(_bad_ref(value, allow_example_refs=allow_example_refs) for value in ref_values):
        return False, "证据引用不能指向 local/prod/mock/fake/stub/example/browser/simulator 边界"

    missing_environment_refs = [
        key
        for key in REQUIRED_REFS
        if not _has_environment_token(str(payload.get(key, "")), environment)
    ]
    if missing_environment_refs:
        return False, f"证据引用必须包含 environment 标记 {environment}: {', '.join(missing_environment_refs)}"

    required_counts = (
        "barcode_samples_scanned",
        "m2_operations_exercised",
        "m3_operations_exercised",
        "offline_replays_exercised",
        "idempotency_replays_exercised",
    )
    invalid_counts = [key for key in required_counts if not _positive_int(payload, key)]
    if invalid_counts:
        return False, f"计数必须 >= 1: {', '.join(invalid_counts)}"

    required_flags = (
        "real_pda_used",
        "physical_scan_key_verified",
        "dev_or_staging_service_verified",
        "audit_event_verified",
        "l7_review_completed",
        "usability_review_completed",
    )
    invalid_flags = [key for key in required_flags if payload.get(key) is not True]
    if invalid_flags:
        return False, f"布尔证据必须为 true: {', '.join(invalid_flags)}"

    return True, "Wave 3 PDA runtime evidence 内容有效"


def validate_one(path: Path, *, allow_example_refs: bool) -> tuple[bool, str]:
    payload, error = read_json(path)
    if error:
        return False, error
    ok, message = validate_wave3_pda_runtime_payload(
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
