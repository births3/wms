#!/usr/bin/env python3
"""Validate Wave 4 external dependency evidence JSON."""
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
    validate_one as _validate_one,
)

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_EVIDENCE = REPO_ROOT / "docs/retros/wave-4-external-dependencies.json"

BLOCKED_REF_TOKENS = (
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "local",
    "prod",
    "production",
    "mock",
    "fake",
    "stub",
    "example",
)

REQUIRED_REFS = (
    "api_doc_ref",
    "auth_doc_ref",
    "error_code_doc_ref",
    "rate_limit_doc_ref",
    "credential_ref",
    "success_report_log_ref",
    "failure_retry_log_ref",
    "audit_event_query_ref",
)
PLACEHOLDER_TOKENS = ("yyyy", "<", ">", "todo", "tbd", "待填", "待确认")
INTERNAL_WMS_TRACE_REF_TOKENS = (
    "internal://",
    "wms-api",
    "/api/codes",
    "trace-code-openapi",
    "openapi/wms-openapi",
    "shared/openapi",
)


def _bad_ref(value: str, *, allow_example_refs: bool) -> bool:
    return bad_ref(
        value,
        allow_example_refs=allow_example_refs,
        blocked_ref_tokens=BLOCKED_REF_TOKENS,
    )


def _placeholder_fields(payload: dict[str, object], keys: tuple[str, ...]) -> list[str]:
    return placeholder_fields(payload, keys, placeholder_tokens=PLACEHOLDER_TOKENS)


def _internal_wms_trace_ref_fields(payload: dict[str, object]) -> list[str]:
    fields = []
    for key in REQUIRED_REFS:
        value = payload.get(key)
        if not isinstance(value, str):
            continue
        lowered = value.lower()
        if any(token in lowered for token in INTERNAL_WMS_TRACE_REF_TOKENS):
            fields.append(key)
    return fields


def validate_wave4_external_dependency_payload(
    payload: object,
    *,
    allow_example_refs: bool = False,
) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "Wave 4 external evidence 顶层必须是 object"

    environment = str(payload.get("environment", "")).lower()
    if environment not in {"dev", "staging"}:
        return False, "environment 必须是真实 dev 或 staging，不能是 local/prod/production/mock/fake/stub/example"

    if payload.get("platform") != "码上放心":
        return False, "platform 必须为 码上放心"

    missing_refs = [key for key in REQUIRED_REFS if not payload.get(key)]
    if missing_refs:
        return False, f"缺少必需证据引用: {', '.join(missing_refs)}"

    placeholder_fields = _placeholder_fields(payload, ("platform", *REQUIRED_REFS))
    if placeholder_fields:
        return False, f"真实 Wave 4 external evidence 不能保留模板占位: {', '.join(placeholder_fields)}"

    bad_ref_fields = blocked_ref_fields(
        payload,
        REQUIRED_REFS,
        is_bad_ref=_bad_ref,
        allow_example_refs=allow_example_refs,
    )
    if bad_ref_fields:
        return False, blocked_ref_message(
            "local/prod/production/mock/fake/stub/example",
            bad_ref_fields,
        )

    internal_wms_ref_fields = _internal_wms_trace_ref_fields(payload)
    if internal_wms_ref_fields:
        return False, (
            "不能用 WMS 内部追溯码接口替代码上放心 evidence: "
            + ", ".join(internal_wms_ref_fields)
        )

    credential_ref = str(payload.get("credential_ref", ""))
    if not credential_ref.startswith("vault://"):
        return False, "credential_ref 必须是 vault:// 引用，不能写入明文凭证"

    missing_environment_refs = [
        key
        for key in REQUIRED_REFS
        if not _has_environment_token(str(payload.get(key, "")), environment)
    ]
    if missing_environment_refs:
        return False, f"证据引用必须包含 environment 标记 {environment}: {', '.join(missing_environment_refs)}"

    reported_events = payload.get("reported_events")
    if not isinstance(reported_events, int) or reported_events < 1:
        return False, "reported_events 必须 >= 1"

    failed_events = payload.get("failed_events_exercised")
    if not isinstance(failed_events, int) or failed_events < 1:
        return False, "failed_events_exercised 必须 >= 1"

    if payload.get("pending_replay_queue_verified") is not True:
        return False, "pending_replay_queue_verified 必须为 true"

    return True, "Wave 4 external dependency evidence 内容有效"


def validate_one(path: Path, *, allow_example_refs: bool) -> tuple[bool, str]:
    return _validate_one(
        path,
        allow_example_refs=allow_example_refs,
        validate=validate_wave4_external_dependency_payload,
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
