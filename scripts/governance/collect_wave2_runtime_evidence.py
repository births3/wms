#!/usr/bin/env python3
"""Collect Wave 2 config-center Feature Flag runtime evidence from dev/staging."""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from record_wave2_runtime_evidence import build_payload, write_payload
from report_wave2_completion import contains_blocked_runtime_ref_token

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_ACCESS_TOKEN_ENV = "WAVE_2_H1_TOKEN"
BUSINESS_SMOKE_PATH = "/api/v1/inventory/batches"
BUSINESS_SMOKE_FLAG = "m3_inventory_batches_config_center_smoke"
FAIL_CLOSED_CODE = "M1_CONFIG_FLAG_MISSING"
PLACEHOLDER_RE = re.compile(
    r"(yyyy|<|>|todo|tbd|your-|internal-domain|待填|待确认)",
    re.IGNORECASE,
)


class EvidenceError(Exception):
    pass


@dataclass(frozen=True)
class HttpJsonResult:
    status: int
    payload: Any


def contains_environment_token(value: str, environment: str) -> bool:
    return (
        re.search(
            rf"(^|[^0-9a-z]){re.escape(environment.lower())}([^0-9a-z]|$)",
            value.lower(),
        )
        is not None
    )


def validate_ref(label: str, value: str, environment: str) -> None:
    if not value:
        raise EvidenceError(f"{label} is required")
    if contains_blocked_runtime_ref_token(value):
        raise EvidenceError(
            f"{label} must not point to localhost/local/prod/production/stub/mock/fake/example boundaries"
        )
    if PLACEHOLDER_RE.search(value):
        raise EvidenceError(f"{label} must not contain template placeholders")
    if not contains_environment_token(value, environment):
        raise EvidenceError(f"{label} must include environment token: {environment}")


def access_token(env_name: str) -> str:
    token = os.environ.get(env_name, "").strip()
    if token:
        return token
    raise EvidenceError(f"{env_name} is required")


def join_url(base_url: str, path: str) -> str:
    return f"{base_url.rstrip('/')}/{path.lstrip('/')}"


def http_json(
    method: str,
    url: str,
    token: str,
    body: dict[str, Any] | None = None,
    timeout_seconds: int = 30,
) -> HttpJsonResult:
    headers = {
        "authorization": f"Bearer {token}",
        "accept": "application/json",
    }
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["content-type"] = "application/json"
    request = urllib.request.Request(url, data=data, headers=headers, method=method)

    try:
        with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
            payload = json.loads(response.read().decode("utf-8") or "{}")
            return HttpJsonResult(response.status, payload)
    except urllib.error.HTTPError as error:
        payload_text = error.read().decode("utf-8") or "{}"
        try:
            payload = json.loads(payload_text)
        except json.JSONDecodeError:
            payload = {"raw_body": payload_text}
        return HttpJsonResult(error.code, payload)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
        raise EvidenceError(f"HTTP request failed for {url}: {error}") from error


def require_status(label: str, result: HttpJsonResult, expected: int) -> None:
    if result.status != expected:
        raise EvidenceError(f"{label} expected HTTP {expected}, got {result.status}: {result.payload}")


def require_error_code(label: str, result: HttpJsonResult, expected: str) -> None:
    if result.status != 404 or not isinstance(result.payload, dict):
        raise EvidenceError(f"{label} expected fail-closed 404 {expected}, got {result.status}: {result.payload}")
    if result.payload.get("code") != expected:
        raise EvidenceError(f"{label} expected error code {expected}, got {result.payload.get('code')}")


def positive_int_from_payload(label: str, payload: Any, key: str) -> int:
    if not isinstance(payload, dict):
        raise EvidenceError(f"{label} response must be object")
    value = payload.get(key)
    if not isinstance(value, int) or value <= 0:
        raise EvidenceError(f"{label}.{key} must be > 0, got {value}")
    return value


def require_clean_reconcile(result: HttpJsonResult) -> int:
    require_status("reconcile", result, 200)
    if not isinstance(result.payload, dict):
        raise EvidenceError("reconcile response must be object")
    matched = result.payload.get("matched")
    if not isinstance(matched, int) or matched <= 0:
        raise EvidenceError(f"reconcile.matched must be > 0, got {matched}")
    if result.payload.get("missing_in_config_center") or result.payload.get("mismatched"):
        raise EvidenceError("reconcile still has missing_in_config_center or mismatched")
    return matched


def require_export_contains_smoke_flag(result: HttpJsonResult) -> None:
    require_status("export", result, 200)
    if not isinstance(result.payload, dict):
        raise EvidenceError("export response must be object")
    if result.payload.get("source") != "config_center":
        raise EvidenceError(f"export.source must be config_center, got {result.payload.get('source')}")
    flags = result.payload.get("flags")
    if not isinstance(flags, list):
        raise EvidenceError("export.flags must be array")
    has_smoke_flag = any(
        isinstance(flag, dict) and flag.get("key") == BUSINESS_SMOKE_FLAG
        for flag in flags
    )
    if not has_smoke_flag:
        raise EvidenceError(f"export.flags must include {BUSINESS_SMOKE_FLAG}")


def require_archive_ref(result: HttpJsonResult, expected_ref: str) -> None:
    require_status("archive file source", result, 200)
    if not isinstance(result.payload, dict):
        raise EvidenceError("archive file source response must be object")
    if result.payload.get("archive_ref") != expected_ref:
        raise EvidenceError(
            f"archive_ref echo mismatch: expected {expected_ref}, got {result.payload.get('archive_ref')}"
        )


def collect(args: argparse.Namespace, token: str) -> dict[str, Any]:
    base_url = args.service_url.rstrip("/")
    timeout = args.timeout_seconds

    switch_before = http_json(
        "POST",
        join_url(base_url, "/api/v1/config-center/feature-flags/source"),
        token,
        {"source": "config_center"},
        timeout,
    )
    require_status("switch source before fail-closed smoke", switch_before, 200)

    fail_closed = http_json("GET", join_url(base_url, BUSINESS_SMOKE_PATH), token, None, timeout)
    require_error_code("business fail-closed smoke", fail_closed, FAIL_CLOSED_CODE)

    migrated = http_json(
        "POST",
        join_url(base_url, "/api/v1/config-center/feature-flags/migrate"),
        token,
        None,
        timeout,
    )
    require_status("migrate", migrated, 200)
    migrated_count = positive_int_from_payload("migrate", migrated.payload, "migrated_count")

    reconcile = http_json(
        "GET",
        join_url(base_url, "/api/v1/config-center/feature-flags/reconcile"),
        token,
        None,
        timeout,
    )
    reconcile_matched = require_clean_reconcile(reconcile)

    exported = http_json(
        "GET",
        join_url(base_url, "/api/v1/config-center/feature-flags/export"),
        token,
        None,
        timeout,
    )
    require_export_contains_smoke_flag(exported)

    switch_after = http_json(
        "POST",
        join_url(base_url, "/api/v1/config-center/feature-flags/source"),
        token,
        {"source": "config_center"},
        timeout,
    )
    require_status("switch source after migration", switch_after, 200)

    success = http_json("GET", join_url(base_url, BUSINESS_SMOKE_PATH), token, None, timeout)
    require_status("business success smoke", success, 200)

    archived = http_json(
        "POST",
        join_url(base_url, "/api/v1/config-center/feature-flags/archive-file-source"),
        token,
        {"archive_ref": args.archive_ref},
        timeout,
    )
    require_archive_ref(archived, args.archive_ref)

    return build_payload(
        argparse.Namespace(
            environment=args.environment,
            service_url=args.service_url,
            migrated_count=migrated_count,
            reconcile_matched=reconcile_matched,
            business_smoke_path=BUSINESS_SMOKE_PATH,
            business_smoke_enabled_flag=BUSINESS_SMOKE_FLAG,
            business_smoke_success_status=200,
            business_smoke_fail_closed_error_code=FAIL_CLOSED_CODE,
            smoke_log_ref=args.smoke_log_ref,
            reconcile_log_ref=args.reconcile_log_ref,
            archive_ref=args.archive_ref,
        )
    )


def validate_inputs(args: argparse.Namespace, *, require_output_writable: bool) -> None:
    if args.environment not in {"dev", "staging"}:
        raise EvidenceError("environment must be dev or staging")
    for label, value in (
        ("service-url", args.service_url),
        ("smoke-log-ref", args.smoke_log_ref),
        ("reconcile-log-ref", args.reconcile_log_ref),
        ("archive-ref", args.archive_ref),
    ):
        validate_ref(label, value, args.environment)
    if require_output_writable and args.output.exists() and not args.force:
        raise EvidenceError(f"{args.output} already exists; pass --force to overwrite")


def default_env(name: str, fallback: str = "") -> str:
    return os.environ.get(name, fallback).strip()


def timeout_seconds_from_env() -> int:
    value = default_env("WAVE_2_CURL_TIMEOUT_SECONDS", "30")
    try:
        timeout_seconds = int(value)
    except ValueError as error:
        raise EvidenceError(f"WAVE_2_CURL_TIMEOUT_SECONDS must be an integer, got {value!r}") from error
    if timeout_seconds <= 0:
        raise EvidenceError(f"WAVE_2_CURL_TIMEOUT_SECONDS must be > 0, got {timeout_seconds}")
    return timeout_seconds


def parse_timeout_seconds(value: str) -> int:
    try:
        timeout_seconds = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be an integer") from error
    if timeout_seconds <= 0:
        raise argparse.ArgumentTypeError("must be > 0")
    return timeout_seconds


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=REPO_ROOT / "docs/retros/wave-2-runtime-evidence.json")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument(
        "--environment",
        default=default_env("WAVE_2_ENVIRONMENT"),
    )
    parser.add_argument("--service-url", default=default_env("WAVE_2_SERVICE_URL"))
    parser.add_argument("--token-env", default=DEFAULT_ACCESS_TOKEN_ENV)
    parser.add_argument("--smoke-log-ref", default=default_env("WAVE_2_SMOKE_LOG_REF"))
    parser.add_argument("--reconcile-log-ref", default=default_env("WAVE_2_RECONCILE_LOG_REF"))
    parser.add_argument("--archive-ref", default=default_env("WAVE_2_ARCHIVE_REF"))
    parser.add_argument("--timeout-seconds", type=parse_timeout_seconds)
    args = parser.parse_args(argv)

    try:
        if args.timeout_seconds is None:
            args.timeout_seconds = timeout_seconds_from_env()
        if not args.environment:
            raise EvidenceError("environment is required")
        validate_inputs(args, require_output_writable=not args.check_only)
        token = access_token(args.token_env)
        if args.check_only:
            print(f"wave2 runtime evidence readiness ok: environment={args.environment}")
            return 0
        payload = collect(args, token)
        ok, message = write_payload(args.output, payload, force=args.force)
        if not ok:
            raise EvidenceError(message)
        print(f"✓ {message}")
        return 0
    except EvidenceError as error:
        print(f"wave2 runtime evidence rejected: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
