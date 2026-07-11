#!/usr/bin/env python3
"""Check Wave 3 real PDA and L7 readiness before recording runtime evidence."""
from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

from validate_wave3_pda_runtime_evidence import (
    validate_wave3_pda_runtime_payload,
)

from _wave3_pda_runtime_readiness_constants import *  # noqa: F403


class ReadinessError(Exception):
    """Expected readiness failure for external state or malformed input."""


@dataclass(frozen=True)
class HttpJsonResult:
    status: int
    payload: Any


@dataclass(frozen=True)
class HttpTextResult:
    status: int
    text: str


def join_url(base_url: str, path: str) -> str:
    return f"{base_url.rstrip('/')}/{path.lstrip('/')}"


def http_json(url: str, timeout_seconds: int = 10) -> HttpJsonResult:
    request = urllib.request.Request(
        url,
        headers={"accept": "application/json"},
        method="GET",
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(request, timeout=timeout_seconds) as response:
            payload_text = response.read().decode("utf-8") or "{}"
            return HttpJsonResult(response.status, json.loads(payload_text))
    except urllib.error.HTTPError as error:
        payload_text = error.read().decode("utf-8") or "{}"
        try:
            payload = json.loads(payload_text)
        except json.JSONDecodeError:
            payload = {"raw_body": payload_text}
        return HttpJsonResult(error.code, payload)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
        raise ReadinessError(f"HTTP request failed for {url}: {error}") from error


def http_text_with_api_key(
    url: str,
    api_key: str,
    timeout_seconds: int = 10,
) -> HttpTextResult:
    request = urllib.request.Request(
        url,
        headers={
            "accept": "application/yaml, text/yaml, application/json",
            "X-API-Key": api_key,
        },
        method="GET",
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(request, timeout=timeout_seconds) as response:
            return HttpTextResult(
                response.status,
                response.read().decode("utf-8", errors="replace"),
            )
    except urllib.error.HTTPError as error:
        return HttpTextResult(
            error.code,
            error.read().decode("utf-8", errors="replace"),
        )
    except (urllib.error.URLError, TimeoutError) as error:
        raise ReadinessError(f"HTTP request failed for {url}: {error}") from error


def parse_positive_int(value: str) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be an integer") from error
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be > 0")
    return parsed


def parse_timeout_seconds(value: str) -> int:
    return parse_positive_int(value)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--environment", choices=["dev", "staging"])
    parser.add_argument("--service-url")
    parser.add_argument("--health-path", default=DEFAULT_HEALTH_PATH)
    parser.add_argument("--wave3-route-path", default=DEFAULT_WAVE3_ROUTE_PATH)
    parser.add_argument("--timeout-seconds", type=parse_timeout_seconds, default=10)
    parser.add_argument("--trace-code-openapi-url")
    parser.add_argument("--pda-model")
    parser.add_argument("--android-version")
    parser.add_argument("--scan-input-method")
    parser.add_argument(
        "--pda-stack-candidate",
        choices=["react-native", "webview-capacitor"],
    )
    parser.add_argument("--pda-device-ref")
    parser.add_argument(
        "--spike005-result-ref",
        "--spike-result-ref",
        dest="spike005_result_ref",
    )
    parser.add_argument("--m2-scan-log-ref")
    parser.add_argument("--m3-scan-log-ref")
    parser.add_argument("--offline-replay-log-ref")
    parser.add_argument("--idempotency-replay-log-ref")
    parser.add_argument("--audit-event-query-ref")
    parser.add_argument("--l7-run-ref")
    parser.add_argument("--usability-review-ref")
    parser.add_argument("--native-shell-ref")
    parser.add_argument("--native-scan-plugin-ref")
    parser.add_argument("--barcode-samples-scanned", type=parse_positive_int)
    parser.add_argument("--m2-operations-exercised", type=parse_positive_int)
    parser.add_argument("--m3-operations-exercised", type=parse_positive_int)
    parser.add_argument("--offline-replays-exercised", type=parse_positive_int)
    parser.add_argument("--idempotency-replays-exercised", type=parse_positive_int)
    parser.add_argument("--real-pda-used", action="store_true")
    parser.add_argument("--physical-scan-key-verified", action="store_true")
    parser.add_argument("--dev-or-staging-service-verified", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    parser.add_argument("--l7-review-completed", action="store_true")
    parser.add_argument("--usability-review-completed", action="store_true")
    parser.add_argument(
        "--service-precheck-only",
        action="store_true",
        help=(
            "Only probe dev/staging health and Wave3 auth boundary. "
            "Does not validate PDA evidence fields and cannot close W6.D."
        ),
    )
    parser.add_argument(
        "--materials-checklist",
        action="store_true",
        help=(
            "Print the W6.D PDA field ownership checklist. "
            "Does not probe services, write evidence, or close W6.D."
        ),
    )
    parser.add_argument(
        "--field-work-request",
        action="store_true",
        help=(
            "Print the W6.D PDA field work request package. "
            "Does not probe services, write evidence, or close W6.D."
        ),
    )
    parser.add_argument(
        "--field-execution-summary",
        action="store_true",
        help=(
            "Print the W6.D field execution gap summary. "
            "Does not probe services, write evidence, or close W6.D."
        ),
    )
    parser.add_argument(
        "--field-precheck-summary",
        action="store_true",
        help=(
            "Run the read-only field precheck bundle: service precheck, trace-code "
            "OpenAPI precheck, and field execution summary. Does not write evidence "
            "or close W6.D."
        ),
    )
    parser.add_argument(
        "--field-precheck-attachment",
        help=(
            "Read a sanitized wave3-pda-field-precheck attachment to mark "
            "already verified no-PDA precheck env vars as satisfied. Does not "
            "write runtime evidence or close W6.D."
        ),
    )
    parser.add_argument(
        "--field-owner-gap-actions",
        action="store_true",
        help=(
            "Print current W6.D gaps grouped by source owner for field assignment. "
            "Does not probe services, write evidence, or close W6.D."
        ),
    )
    parser.add_argument(
        "--field-handoff-bundle",
        action="store_true",
        help=(
            "Print one read-only W6.D field handoff bundle combining the preaudit kit, "
            "materials checklist, owner gaps, package template metadata, and optional "
            "from-env prechecks. Does not write evidence or close W6.D."
        ),
    )
    parser.add_argument(
        "--field-handoff-output",
        type=Path,
        help=(
            "Write the sanitized field handoff bundle JSON to this path. "
            "Only valid with --field-handoff-bundle; does not write runtime evidence."
        ),
    )
    parser.add_argument(
        "--field-handoff-force",
        action="store_true",
        help=(
            "Overwrite an existing --field-handoff-output file. Only valid with "
            "--field-handoff-bundle; does not write runtime evidence."
        ),
    )
    parser.add_argument(
        "--preaudit-kit",
        action="store_true",
        help=(
            "Print the W6.D PDA pre-audit kit for project and field owners. "
            "Does not probe services, write evidence, or close W6.D."
        ),
    )
    parser.add_argument(
        "--trace-code-openapi-precheck",
        action="store_true",
        help=(
            "Read-only probe for trace-code OpenAPI contract using "
            "WAVE_3_PDA_TRACE_CODE_* env vars. Does not write evidence or close W6.D."
        ),
    )
    parser.add_argument(
        "--from-env",
        action="store_true",
        help="Read WAVE_3_PDA_* variables from the exported evidence template.",
    )
    parser.add_argument("--json", action="store_true")
    return parser


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    return build_parser().parse_args(argv)


def apply_env_args(args: argparse.Namespace, *, service_precheck_only: bool = False) -> list[str]:
    issues: list[str] = []
    string_fields = ENV_STRING_FIELDS
    if service_precheck_only:
        string_fields = {
            "environment": ENV_STRING_FIELDS["environment"],
            "service_url": ENV_STRING_FIELDS["service_url"],
        }
    for field, env_name in string_fields.items():
        value = os.environ.get(env_name)
        if value is not None:
            setattr(args, field, value.strip())

    if service_precheck_only:
        return issues

    for field, env_name in ENV_COUNT_FIELDS.items():
        value = os.environ.get(env_name)
        if value is None:
            continue
        try:
            parsed = int(value.strip())
        except ValueError:
            issues.append(f"{env_name} must be an integer")
            continue
        if parsed <= 0:
            issues.append(f"{env_name} must be > 0")
            continue
        setattr(args, field, parsed)

    for field, env_name in ENV_FLAG_FIELDS.items():
        raw_value = os.environ.get(env_name, "")
        value = raw_value.strip().lower()
        if value in TRUE_ENV_VALUES:
            setattr(args, field, True)
        elif value in FALSE_ENV_VALUES:
            setattr(args, field, False)
        else:
            issues.append(f"{env_name} must be true or false")
    return issues


def apply_trace_code_env_args(args: argparse.Namespace) -> None:
    for field, env_name in TRACE_CODE_ENV_FIELDS.items():
        value = os.environ.get(env_name)
        if value is not None:
            setattr(args, field, value.strip())


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    payload: dict[str, Any] = {}
    for field in (*STRING_FIELDS, *WEBVIEW_CAPACITOR_FIELDS):
        value = getattr(args, field, None)
        if value is not None and value != "":
            payload[field] = value
    for field in COUNT_FIELDS:
        value = getattr(args, field, None)
        if value is not None:
            payload[field] = value
    for field in FLAG_FIELDS:
        payload[field] = getattr(args, field)
    return payload


def missing_input_issues(args: argparse.Namespace) -> list[str]:
    issues: list[str] = []
    for field in ("environment", "service_url"):
        if not getattr(args, field, None):
            issues.append(f"{field} is required")
    issues.extend(service_url_boundary_issues(args))

    payload = build_payload(args)
    for field in STRING_FIELDS:
        if not str(payload.get(field, "")).strip():
            issues.append(f"{field} is required")

    if args.pda_stack_candidate == "webview-capacitor":
        for field in WEBVIEW_CAPACITOR_FIELDS:
            if not str(payload.get(field, "")).strip():
                issues.append(f"{field} is required")

    for field in COUNT_FIELDS:
        if field not in payload:
            issues.append(f"{field} is required")

    for field in FLAG_FIELDS:
        if payload.get(field) is not True:
            issues.append(f"{field} must be true")
    return list(dict.fromkeys(issues))


def missing_service_precheck_issues(args: argparse.Namespace) -> list[str]:
    issues: list[str] = []
    for field in ("environment", "service_url"):
        if not getattr(args, field, None):
            issues.append(f"{field} is required")
    issues.extend(service_url_boundary_issues(args))
    return issues


def service_url_boundary_issues(args: argparse.Namespace) -> list[str]:
    raw_service_url = str(getattr(args, "service_url", "") or "")
    service_url = raw_service_url.lower()
    if not service_url:
        return []
    issues = sensitive_url_issues(raw_service_url, field_name="service_url")
    if any(token in service_url for token in BLOCKED_SERVICE_URL_TOKENS):
        issues.append(SERVICE_URL_BOUNDARY_MESSAGE)
    return list(dict.fromkeys(issues))


def sensitive_url_issues(url: str, *, field_name: str) -> list[str]:
    text = str(url or "").strip()
    if not text:
        return []
    parsed = urllib.parse.urlsplit(text)
    issues: list[str] = []
    if parsed.username or parsed.password:
        issues.append(f"{field_name} cannot contain userinfo credentials")
    query_params = urllib.parse.parse_qsl(parsed.query, keep_blank_values=True)
    for key, _value in query_params:
        lowered_key = key.lower()
        if lowered_key in SENSITIVE_URL_QUERY_PARAMS:
            issues.append(
                f"{field_name} query cannot contain sensitive parameter: {lowered_key}",
            )
    return list(dict.fromkeys(issues))


def sanitize_url_for_output(url: object) -> object:
    if not isinstance(url, str) or not url:
        return url
    parsed = urllib.parse.urlsplit(url)
    netloc = parsed.netloc.rsplit("@", maxsplit=1)[-1]
    query_params = urllib.parse.parse_qsl(parsed.query, keep_blank_values=True)
    sanitized_query = urllib.parse.urlencode([
        (
            key,
            "REDACTED"
            if key.lower() in SENSITIVE_URL_QUERY_PARAMS
            else value,
        )
        for key, value in query_params
    ])
    return urllib.parse.urlunsplit((
        parsed.scheme,
        netloc,
        parsed.path,
        sanitized_query,
        parsed.fragment,
    ))


def missing_env_vars_for_issues(issues: list[str]) -> list[str]:
    missing: list[str] = []
    for issue in issues:
        if not issue.endswith(" is required"):
            continue
        field = issue.removesuffix(" is required")
        env_name = ENV_FIELDS.get(field)
        if env_name:
            missing.append(env_name)
    return list(dict.fromkeys(missing))


def missing_env_var_owner_details(env_vars: list[str]) -> list[dict[str, object]]:
    return [
        dict(ENV_VAR_OWNER_DETAILS[env_var])
        for env_var in env_vars
        if env_var in ENV_VAR_OWNER_DETAILS
    ]


def missing_trace_code_env_var_owner_details(env_vars: list[str]) -> list[dict[str, object]]:
    return [
        dict(TRACE_CODE_ENV_VAR_OWNER_DETAILS[env_var])
        for env_var in env_vars
        if env_var in TRACE_CODE_ENV_VAR_OWNER_DETAILS
    ]


def check_payload_contract(args: argparse.Namespace) -> list[str]:
    issues = missing_input_issues(args)
    if issues:
        return issues

    ok, message = validate_wave3_pda_runtime_payload(
        build_payload(args),
        allow_example_refs=False,
    )
    if ok:
        return []
    return [message]


def check_staging_service(args: argparse.Namespace, facts: dict[str, object]) -> list[str]:
    issues: list[str] = []
    if not args.service_url:
        return ["service_url is required"]

    health = http_json(join_url(args.service_url, args.health_path), args.timeout_seconds)
    facts["healthz_status"] = health.status
    if health.status != 200:
        issues.append(f"healthz expected HTTP 200, got {health.status}")
    elif isinstance(health.payload, dict):
        status = health.payload.get("status")
        facts["healthz_payload_status"] = status
        if status != "ok":
            issues.append(f"healthz payload.status expected ok, got {status}")

    wave3_route = http_json(
        join_url(args.service_url, args.wave3_route_path),
        args.timeout_seconds,
    )
    facts["wave3_route_status"] = wave3_route.status
    if isinstance(wave3_route.payload, dict):
        facts["wave3_route_error_code"] = wave3_route.payload.get("code")
    if (
        wave3_route.status != 401
        or not isinstance(wave3_route.payload, dict)
        or wave3_route.payload.get("code") != EXPECTED_WAVE3_UNAUTHORIZED_CODE
    ):
        issues.append(
            "Wave3 route expected 401 AUTH-001 without Authorization header, "
            f"got {wave3_route.status}: {wave3_route.payload}"
        )
    return issues


def check_trace_code_openapi(
    args: argparse.Namespace,
) -> tuple[bool, dict[str, object], list[str], list[str]]:
    facts: dict[str, object] = {
        "openapi_url": sanitize_url_for_output(
            getattr(args, "trace_code_openapi_url", None),
        ),
        "required_paths": list(TRACE_CODE_REQUIRED_PATHS),
    }
    missing_env_vars: list[str] = []
    for field, env_name in TRACE_CODE_ENV_FIELDS.items():
        if not str(getattr(args, field, "") or "").strip():
            missing_env_vars.append(env_name)

    if missing_env_vars:
        return (
            False,
            facts,
            [f"{env_var} is required" for env_var in missing_env_vars],
            missing_env_vars,
        )

    url_issues = sensitive_url_issues(
        args.trace_code_openapi_url,
        field_name="trace_code_openapi_url",
    )
    if url_issues:
        return False, facts, url_issues, []

    response = http_text_with_api_key(
        args.trace_code_openapi_url,
        args.trace_code_api_key,
        args.timeout_seconds,
    )
    facts["status"] = response.status
    if response.status != 200:
        return (
            False,
            facts,
            [f"trace-code OpenAPI expected HTTP 200, got {response.status}"],
            [],
        )

    try:
        document = yaml.safe_load(response.text) or {}
    except yaml.YAMLError as error:
        raise ReadinessError(f"trace-code OpenAPI YAML parse failed: {error}") from error

    if not isinstance(document, dict):
        return False, facts, ["trace-code OpenAPI document must be an object"], []

    info = document.get("info") if isinstance(document.get("info"), dict) else {}
    paths = document.get("paths") if isinstance(document.get("paths"), dict) else {}
    components = (
        document.get("components")
        if isinstance(document.get("components"), dict)
        else {}
    )
    security_schemes = (
        components.get("securitySchemes")
        if isinstance(components.get("securitySchemes"), dict)
        else {}
    )
    api_key_auth = security_schemes.get("ApiKeyAuth")
    if not isinstance(api_key_auth, dict):
        api_key_auth = {}

    facts["openapi"] = document.get("openapi")
    facts["title"] = info.get("title")
    facts["required_paths_present"] = [
        path for path in TRACE_CODE_REQUIRED_PATHS if path in paths
    ]
    facts["required_operations_present"] = [
        label
        for (path, method), label in zip(
            TRACE_CODE_REQUIRED_OPERATIONS,
            TRACE_CODE_REQUIRED_OPERATION_LABELS,
            strict=True,
        )
        if isinstance(paths.get(path), dict) and method in paths[path]
    ]
    facts["api_key_header_name"] = api_key_auth.get("name")

    issues: list[str] = []
    if document.get("openapi") != "3.0.3":
        issues.append("OpenAPI version 3.0.3 is required")
    for path in TRACE_CODE_REQUIRED_PATHS:
        if path not in paths:
            issues.append(f"{path} path is required")
    for path, method in TRACE_CODE_REQUIRED_OPERATIONS:
        path_item = paths.get(path)
        if not isinstance(path_item, dict) or method not in path_item:
            issues.append(f"{method.upper()} {path} operation is required")
    if not (
        api_key_auth.get("type") == "apiKey"
        and api_key_auth.get("in") == "header"
        and api_key_auth.get("name") == "X-API-Key"
    ):
        issues.append("ApiKeyAuth header X-API-Key is required")
    return not issues, facts, issues, []


def check_readiness(args: argparse.Namespace) -> tuple[bool, dict[str, object], list[str]]:
    facts: dict[str, object] = {
        "environment": args.environment,
        "service_url": sanitize_url_for_output(args.service_url),
        "health_path": args.health_path,
        "wave3_route_path": args.wave3_route_path,
        "service_precheck_only": args.service_precheck_only,
    }
    if args.service_precheck_only:
        issues = missing_service_precheck_issues(args)
    else:
        issues = check_payload_contract(args)
    if args.service_url and not any(
        issue == SERVICE_URL_BOUNDARY_MESSAGE or issue.startswith("service_url ")
        for issue in issues
    ):
        issues.extend(check_staging_service(args, facts))
    return not issues, facts, issues


from _wave3_pda_runtime_readiness_field import *  # noqa: F403

















































from _wave3_pda_runtime_readiness_output import *  # noqa: F403























def main(argv: list[str] | None = None) -> int:
    requested_argv = sys.argv[1:] if argv is None else argv
    try:
        args = parse_args(argv)
        field_precheck_attachment = load_field_precheck_attachment(
            args.field_precheck_attachment,
        )
        if args.materials_checklist:
            payload = materials_checklist_payload()
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_materials_checklist_text(payload)
            return 0

        if args.preaudit_kit:
            payload = preaudit_kit_payload()
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_preaudit_kit_markdown(payload)
            return 0

        if args.field_work_request:
            payload = field_work_request_payload()
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_field_work_request_markdown(payload)
            return 0

        if args.field_execution_summary:
            payload = field_execution_summary_payload(field_precheck_attachment)
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_field_execution_summary_markdown(payload)
            return 0

        if args.field_precheck_summary:
            if args.from_env:
                apply_env_args(args, service_precheck_only=True)
                apply_trace_code_env_args(args)
            service_payload = service_precheck_payload_from_args(args)
            trace_code_payload = trace_code_openapi_precheck_payload_from_args(args)
            field_summary = field_execution_summary_payload(field_precheck_attachment)
            payload = field_precheck_summary_payload(
                service_payload,
                trace_code_payload,
                field_summary,
            )
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_field_precheck_summary_markdown(payload)
            return 0 if payload["ok"] else 1

        if args.field_owner_gap_actions:
            payload = field_owner_gap_actions_payload(field_precheck_attachment)
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_field_owner_gap_actions_markdown(payload)
            return 0

        if args.field_handoff_bundle:
            payload = field_handoff_bundle_payload(
                args,
                include_precheck=args.from_env,
                field_precheck_attachment=field_precheck_attachment,
            )
            if args.field_handoff_output:
                payload["field_handoff_output"] = str(args.field_handoff_output)
                ok_to_write, write_message = write_field_handoff_bundle(
                    args.field_handoff_output,
                    {
                        **payload,
                        "field_handoff_output": str(args.field_handoff_output),
                        "writes_field_handoff_bundle": True,
                    },
                    force=args.field_handoff_force,
                )
                payload["writes_field_handoff_bundle"] = ok_to_write
                payload["message"] = write_message
                if not ok_to_write:
                    payload["ok"] = False
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_field_handoff_bundle_markdown(payload)
            return 0 if payload["ok"] else 1

        if args.trace_code_openapi_precheck:
            if args.from_env:
                apply_trace_code_env_args(args)
            ok, facts, issues, missing_env_vars = check_trace_code_openapi(args)
            payload = trace_code_openapi_payload(
                ok,
                facts,
                issues,
                missing_env_vars,
            )
            if args.json:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print_trace_code_openapi_text(ok, facts, issues)
            return 0 if ok else 1

        if args.from_env:
            env_issues = apply_env_args(
                args,
                service_precheck_only=args.service_precheck_only,
            )
            if env_issues:
                raise ValueError("; ".join(env_issues))

        ok, facts, issues = check_readiness(args)
    except (ReadinessError, OSError, ValueError) as error:
        if "--json" in requested_argv:
            print(json.dumps({
                "check": "check_wave3_pda_runtime_readiness",
                "tier": "T1",
                "category": "流程治理",
                "ok": False,
                "schema_version": 1,
                "mode": "wave3-pda-runtime-readiness",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "error": str(error),
            }, ensure_ascii=False, indent=2))
        else:
            print(f"wave3 pda runtime readiness error: {error}", file=sys.stderr)
        return 2

    if args.json:
        payload = result_payload(
            ok,
            facts,
            issues,
            service_precheck_only=args.service_precheck_only,
        )
        if args.from_env and not ok:
            missing_env_vars = missing_env_vars_for_issues(issues)
            if missing_env_vars:
                payload["missing_env_vars"] = missing_env_vars
                payload["missing_env_var_owners"] = missing_env_var_owner_details(
                    missing_env_vars,
                )
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print_text(ok, facts, issues)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
