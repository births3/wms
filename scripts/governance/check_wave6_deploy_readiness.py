#!/usr/bin/env python3
"""Check Wave 6 gray release readiness before recording deployment evidence."""
from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from validate_wave6_deploy_evidence import validate_wave6_deploy_payload

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_HEALTH_PATH = "/healthz"
BLOCKED_SERVICE_URL_TOKENS = (
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

STRING_FIELDS = (
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
COUNT_FIELDS = (
    "canary_stages_exercised",
    "smoke_checks_passed",
    "rollback_drills_exercised",
)
FLAG_FIELDS = (
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
    "service_url": "WAVE_6_SERVICE_URL",
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


class ReadinessError(Exception):
    """Expected readiness failure for external state or malformed input."""


@dataclass(frozen=True)
class HttpJsonResult:
    status: int
    payload: Any


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
    parser.add_argument(
        "--from-env",
        action="store_true",
        help="Read W6.H fields from WAVE_6_* environment variables.",
    )
    parser.add_argument("--environment", choices=["dev", "staging"])
    parser.add_argument("--deployment-mode", choices=["docker-compose", "kubernetes"])
    parser.add_argument("--release-version")
    parser.add_argument("--service-url")
    parser.add_argument("--health-path", default=DEFAULT_HEALTH_PATH)
    parser.add_argument("--timeout-seconds", type=parse_timeout_seconds, default=10)
    parser.add_argument("--release-plan-ref")
    parser.add_argument("--artifact-ref")
    parser.add_argument("--canary-config-ref")
    parser.add_argument("--smoke-gate-ref")
    parser.add_argument("--observability-dashboard-ref")
    parser.add_argument("--rollback-drill-log-ref")
    parser.add_argument("--approval-record-ref")
    parser.add_argument("--audit-event-query-ref")
    parser.add_argument("--canary-stages-exercised", type=parse_positive_int)
    parser.add_argument("--smoke-checks-passed", type=parse_positive_int)
    parser.add_argument("--rollback-drills-exercised", type=parse_positive_int)
    parser.add_argument("--canary-used", action="store_true")
    parser.add_argument("--full-release-blocked", action="store_true")
    parser.add_argument("--rollback-verified", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    parser.add_argument("--dual-approval-recorded", action="store_true")
    parser.add_argument("--json", action="store_true")
    return parser


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    return build_parser().parse_args(argv)


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    payload: dict[str, Any] = {}
    for field in STRING_FIELDS:
        value = getattr(args, field, None)
        if value is not None:
            payload[field] = value
    for field in COUNT_FIELDS:
        value = getattr(args, field, None)
        if value is not None:
            payload[field] = value
    for field in FLAG_FIELDS:
        payload[field] = getattr(args, field)
    return payload


def bool_from_env(value: str) -> bool:
    return value.strip().lower() in {"1", "true", "yes", "y", "on"}


def apply_from_env(args: argparse.Namespace) -> list[str]:
    missing: list[str] = []
    for field, env_var in ENV_VARS.items():
        raw_value = os.environ.get(env_var)
        if raw_value is None or raw_value.strip() == "":
            missing.append(env_var)
            continue
        if field in COUNT_FIELDS:
            try:
                setattr(args, field, int(raw_value))
            except ValueError:
                setattr(args, field, None)
            continue
        if field in FLAG_FIELDS:
            setattr(args, field, bool_from_env(raw_value))
            continue
        setattr(args, field, raw_value)
    return missing


def missing_from_env_payload(args: argparse.Namespace, missing: list[str]) -> dict[str, object]:
    return {
        "check": "check_wave6_deploy_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": False,
        "schema_version": 1,
        "mode": "wave6-deploy-readiness",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-6-deploy-evidence.json",
        "facts": {
            "environment": args.environment,
            "deployment_mode": args.deployment_mode,
            "release_version": args.release_version,
            "service_url": args.service_url,
            "health_path": args.health_path,
        },
        "issues": [
            "缺少 W6.H 灰度发布 readiness 环境变量；不会写 runtime evidence，W6.H gate remains open",
        ],
        "missing_env_vars": missing,
        "next_commands": [
            "just wave-6-deploy-materials --from-env --json",
            "just wave-6-deploy-audit --from-env --check-only",
            "just wave-6-deploy-audit --from-env",
            "just wave-6-deploy-evidence-record --from-env --check-only --json",
            "just wave-6-deploy-evidence-record --from-env --json",
            "just wave-6-deploy-evidence-validate",
        ],
    }


def missing_input_issues(args: argparse.Namespace) -> list[str]:
    issues: list[str] = []
    if not args.service_url:
        issues.append("service_url is required")

    payload = build_payload(args)
    for field in STRING_FIELDS:
        if not str(payload.get(field, "")).strip():
            issues.append(f"{field} is required")

    for field in COUNT_FIELDS:
        if field not in payload:
            issues.append(f"{field} is required")

    for field in FLAG_FIELDS:
        if payload.get(field) is not True:
            issues.append(f"{field} must be true")
    return list(dict.fromkeys(issues))


def check_payload_contract(args: argparse.Namespace) -> list[str]:
    issues = missing_input_issues(args)
    if issues:
        return issues

    ok, message = validate_wave6_deploy_payload(
        build_payload(args),
        allow_example_refs=False,
    )
    if ok:
        return []
    return [message]


def _has_environment_token(value: str, environment: str) -> bool:
    import re

    return re.search(
        rf"(^|[^0-9a-z]){re.escape(environment)}([^0-9a-z]|$)",
        value.lower(),
    ) is not None


def validate_staging_service_url(service_url: str) -> str | None:
    value = service_url.strip()
    if not value:
        return "service_url is required"

    lowered = value.lower()
    if not lowered.startswith(("http://", "https://")):
        return "service_url must use http:// or https://"

    without_scheme = lowered.split("://", 1)[1]
    host_port_path = without_scheme.rsplit("@", 1)[-1]
    host = host_port_path.split("/", 1)[0].split("?", 1)[0].split("#", 1)[0]
    host = host.split(":", 1)[0]
    if not host:
        return "service_url host is required"
    if all(ch.isdigit() or ch == "." for ch in host):
        return "service_url must use a staging DNS name, not a raw IP"
    if any(token in host_port_path for token in BLOCKED_SERVICE_URL_TOKENS):
        return "service_url contains blocked boundary token"
    if not _has_environment_token(host_port_path, "staging"):
        return "service_url must contain staging boundary token"
    return None


def check_staging_service(args: argparse.Namespace, facts: dict[str, object]) -> list[str]:
    if not args.service_url:
        return ["service_url is required"]

    service_url_issue = validate_staging_service_url(args.service_url)
    if service_url_issue:
        return [service_url_issue]

    health = http_json(join_url(args.service_url, args.health_path), args.timeout_seconds)
    facts["healthz_status"] = health.status
    if health.status != 200:
        return [f"healthz expected HTTP 200, got {health.status}"]

    if isinstance(health.payload, dict):
        status = health.payload.get("status")
        facts["healthz_payload_status"] = status
        if status != "ok":
            return [f"healthz payload.status expected ok, got {status}"]
    return []


def check_readiness(args: argparse.Namespace) -> tuple[bool, dict[str, object], list[str]]:
    facts: dict[str, object] = {
        "environment": args.environment,
        "deployment_mode": args.deployment_mode,
        "release_version": args.release_version,
        "service_url": args.service_url,
        "health_path": args.health_path,
    }
    issues = check_payload_contract(args)
    if args.service_url:
        issues.extend(check_staging_service(args, facts))
    return not issues, facts, issues


def result_payload(ok: bool, facts: dict[str, object], issues: list[str]) -> dict[str, object]:
    return {
        "check": "check_wave6_deploy_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": ok,
        "schema_version": 1,
        "mode": "wave6-deploy-readiness",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-6-deploy-evidence.json",
        "facts": facts,
        "issues": issues,
        "next_commands": [
            "just wave-6-deploy-evidence-record ...",
            "just wave-6-deploy-evidence-validate",
        ],
    }


def print_text(ok: bool, facts: dict[str, object], issues: list[str]) -> None:
    if ok:
        print("✓ Wave 6 deploy readiness passed")
        print("PASS payload_contract: Wave 6 deploy evidence 内容有效")
        print(f"PASS service_health: {facts.get('health_path')} reachable")
        print("不会写入 runtime evidence；不能关闭 W6.H gate")
        return

    print("✘ Wave 6 deploy readiness failed", file=sys.stderr)
    print("不会写入 runtime evidence；不能关闭 W6.H gate", file=sys.stderr)
    for issue in issues:
        print(f"FAIL readiness: {issue}", file=sys.stderr)


def main(argv: list[str] | None = None) -> int:
    try:
        args = parse_args(argv)
        if args.from_env:
            missing_env_vars = apply_from_env(args)
            if missing_env_vars:
                payload = missing_from_env_payload(args, missing_env_vars)
                if args.json:
                    print(json.dumps(payload, ensure_ascii=False, indent=2))
                else:
                    print(f"✘ {payload['issues'][0]}", file=sys.stderr)
                    for env_var in missing_env_vars:
                        print(f"missing env: {env_var}", file=sys.stderr)
                return 1
        ok, facts, issues = check_readiness(args)
    except (ReadinessError, OSError, ValueError) as error:
        if argv is not None and "--json" in argv:
            print(json.dumps({
                "check": "check_wave6_deploy_readiness",
                "tier": "T1",
                "category": "流程治理",
                "ok": False,
                "schema_version": 1,
                "mode": "wave6-deploy-readiness",
                "writes_runtime_evidence": False,
                "closes_gate": False,
                "error": str(error),
            }, ensure_ascii=False, indent=2))
        else:
            print(f"wave6 deploy readiness error: {error}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps(
            result_payload(ok, facts, issues),
            ensure_ascii=False,
            indent=2,
        ))
    else:
        print_text(ok, facts, issues)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
