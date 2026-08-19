#!/usr/bin/env python3
"""Report Wave 6 gray release staging dry-run materials without writing evidence."""
from __future__ import annotations

import argparse
import json
import os
import sys
from uuid import UUID
from typing import Mapping

from validate_wave6_deploy_evidence import validate_wave6_deploy_payload

STRING_ENV_FIELDS = {
    "WAVE_6_ENVIRONMENT": "environment",
    "WAVE_6_DEPLOYMENT_MODE": "deployment_mode",
    "WAVE_6_RELEASE_VERSION": "release_version",
    "WAVE_6_RELEASE_PLAN_REF": "release_plan_ref",
    "WAVE_6_ARTIFACT_REF": "artifact_ref",
    "WAVE_6_CANARY_CONFIG_REF": "canary_config_ref",
    "WAVE_6_SMOKE_GATE_REF": "smoke_gate_ref",
    "WAVE_6_OBSERVABILITY_DASHBOARD_REF": "observability_dashboard_ref",
    "WAVE_6_ROLLBACK_DRILL_LOG_REF": "rollback_drill_log_ref",
    "WAVE_6_APPROVAL_RECORD_REF": "approval_record_ref",
    "WAVE_6_AUDIT_EVENT_QUERY_REF": "audit_event_query_ref",
}
COUNT_ENV_FIELDS = {
    "WAVE_6_CANARY_STAGES_EXERCISED": "canary_stages_exercised",
    "WAVE_6_SMOKE_CHECKS_PASSED": "smoke_checks_passed",
    "WAVE_6_ROLLBACK_DRILLS_EXERCISED": "rollback_drills_exercised",
}
FLAG_ENV_FIELDS = {
    "WAVE_6_CANARY_USED": "canary_used",
    "WAVE_6_FULL_RELEASE_BLOCKED": "full_release_blocked",
    "WAVE_6_ROLLBACK_VERIFIED": "rollback_verified",
    "WAVE_6_AUDIT_EVENT_VERIFIED": "audit_event_verified",
    "WAVE_6_DUAL_APPROVAL_RECORDED": "dual_approval_recorded",
}
DEPLOY_AUDIT_ENV_FIELDS = {
    "WAVE_6_DEPLOY_MODULE": "module",
    "WAVE_6_DEPLOY_ACTION": "action",
    "WAVE_6_DEPLOY_RESOURCE_TYPE": "resource_type",
    "WAVE_6_DEPLOY_RESOURCE_ID": "resource_id",
    "WAVE_6_DEPLOY_ACTOR_ID": "actor_id",
    "WAVE_6_DEPLOY_ACTOR_NAME": "actor_name",
    "WAVE_6_DEPLOY_OWNER_ID": "owner_id",
    "WAVE_6_DEPLOY_JTI": "jti",
}
DEPLOY_AUDIT_UUID_ENV_FIELDS = (
    "WAVE_6_DEPLOY_ACTOR_ID",
    "WAVE_6_DEPLOY_OWNER_ID",
)
REQUIRED_ENV_VARS = (
    "WAVE_6_SERVICE_URL",
    *STRING_ENV_FIELDS.keys(),
    *COUNT_ENV_FIELDS.keys(),
    *FLAG_ENV_FIELDS.keys(),
    *DEPLOY_AUDIT_ENV_FIELDS.keys(),
)
ENV_SOURCE_HINTS = {
    "WAVE_6_SERVICE_URL": "staging DNS service URL",
    "WAVE_6_ENVIRONMENT": "fixed value: staging",
    "WAVE_6_DEPLOYMENT_MODE": "release plan deployment mode",
    "WAVE_6_RELEASE_VERSION": "release plan version",
    "WAVE_6_RELEASE_PLAN_REF": "release ticket / operations evidence store",
    "WAVE_6_ARTIFACT_REF": "registry digest / CI artifact / traceable tag",
    "WAVE_6_CANARY_CONFIG_REF": "release platform / config center canary record",
    "WAVE_6_SMOKE_GATE_REF": "CI or release platform smoke gate",
    "WAVE_6_OBSERVABILITY_DASHBOARD_REF": "Grafana / Prometheus / log query archive",
    "WAVE_6_ROLLBACK_DRILL_LOG_REF": "release platform / CI rollback drill log",
    "WAVE_6_APPROVAL_RECORD_REF": "dual approval ticket without secrets",
    "WAVE_6_AUDIT_EVENT_QUERY_REF": "from wave-6-deploy-audit output",
    "WAVE_6_CANARY_STAGES_EXERCISED": "release plan canary stage count",
    "WAVE_6_SMOKE_CHECKS_PASSED": "smoke gate passed check count",
    "WAVE_6_ROLLBACK_DRILLS_EXERCISED": "rollback drill count",
    "WAVE_6_CANARY_USED": "true after canary was exercised",
    "WAVE_6_FULL_RELEASE_BLOCKED": "true when full release was blocked",
    "WAVE_6_ROLLBACK_VERIFIED": "true after rollback drill passed",
    "WAVE_6_AUDIT_EVENT_VERIFIED": "true after audit_event query is archived",
    "WAVE_6_DUAL_APPROVAL_RECORDED": "true after dual approval is archived",
    "WAVE_6_DEPLOY_MODULE": "release plan audit module",
    "WAVE_6_DEPLOY_ACTION": "release plan audit action",
    "WAVE_6_DEPLOY_RESOURCE_TYPE": "release plan audit resource type",
    "WAVE_6_DEPLOY_RESOURCE_ID": "release plan audit resource id",
    "WAVE_6_DEPLOY_ACTOR_ID": "H1 release actor UUID",
    "WAVE_6_DEPLOY_ACTOR_NAME": "H1 release actor display name",
    "WAVE_6_DEPLOY_OWNER_ID": "canary owner UUID or confirmed system owner UUID",
    "WAVE_6_DEPLOY_JTI": "unique deploy run id",
}
EXPORT_TEMPLATE_DEFAULTS = {
    "WAVE_6_ENVIRONMENT": "staging",
    "WAVE_6_DEPLOYMENT_MODE": "docker-compose",
    "WAVE_6_CANARY_STAGES_EXERCISED": "1",
    "WAVE_6_SMOKE_CHECKS_PASSED": "1",
    "WAVE_6_ROLLBACK_DRILLS_EXERCISED": "1",
    "WAVE_6_CANARY_USED": "true",
    "WAVE_6_FULL_RELEASE_BLOCKED": "true",
    "WAVE_6_ROLLBACK_VERIFIED": "true",
    "WAVE_6_AUDIT_EVENT_VERIFIED": "true",
    "WAVE_6_DUAL_APPROVAL_RECORDED": "true",
}
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


def env_value(env: Mapping[str, str], name: str) -> str:
    return str(env.get(name, "")).strip()


def missing_env_vars(env: Mapping[str, str]) -> list[str]:
    return [name for name in REQUIRED_ENV_VARS if not env_value(env, name)]


def parse_counts(env: Mapping[str, str]) -> tuple[dict[str, int], list[str]]:
    counts: dict[str, int] = {}
    invalid: list[str] = []
    for env_name, field in COUNT_ENV_FIELDS.items():
        value = env_value(env, env_name)
        if not value:
            continue
        try:
            parsed = int(value)
        except ValueError:
            invalid.append(f"{env_name} must be an integer")
            continue
        if parsed < 1:
            invalid.append(f"{env_name} must be >= 1")
            continue
        counts[field] = parsed
    return counts, invalid


def parse_flags(env: Mapping[str, str]) -> tuple[dict[str, bool], list[str]]:
    flags: dict[str, bool] = {}
    invalid: list[str] = []
    for env_name, field in FLAG_ENV_FIELDS.items():
        value = env_value(env, env_name).lower()
        if not value:
            continue
        if value != "true":
            invalid.append(f"{env_name} must be true")
            flags[field] = False
            continue
        flags[field] = True
    return flags, invalid


def validate_deploy_audit_env(env: Mapping[str, str]) -> list[str]:
    invalid: list[str] = []
    for env_name in DEPLOY_AUDIT_UUID_ENV_FIELDS:
        value = env_value(env, env_name)
        if not value:
            continue
        try:
            UUID(value)
        except ValueError:
            invalid.append(f"{env_name} must be a UUID")
    return invalid


def build_payload(env: Mapping[str, str]) -> tuple[dict[str, object], list[str]]:
    invalid: list[str] = []
    payload: dict[str, object] = {}
    for env_name, field in STRING_ENV_FIELDS.items():
        value = env_value(env, env_name)
        if value:
            payload[field] = value

    counts, count_errors = parse_counts(env)
    flags, flag_errors = parse_flags(env)
    invalid.extend(count_errors)
    invalid.extend(flag_errors)
    invalid.extend(validate_deploy_audit_env(env))
    payload.update(counts)
    payload.update(flags)
    return payload, invalid


def _has_environment_token(value: str, environment: str) -> bool:
    import re

    return re.search(
        rf"(^|[^0-9a-z]){re.escape(environment)}([^0-9a-z]|$)",
        value.lower(),
    ) is not None


def validate_staging_service_url(service_url: str) -> str | None:
    value = service_url.strip()
    if not value:
        return "WAVE_6_SERVICE_URL is required"

    lowered = value.lower()
    if not lowered.startswith(("http://", "https://")):
        return "WAVE_6_SERVICE_URL must use http:// or https://"

    without_scheme = lowered.split("://", 1)[1]
    host_port_path = without_scheme.rsplit("@", 1)[-1]
    host = host_port_path.split("/", 1)[0].split("?", 1)[0].split("#", 1)[0]
    host = host.split(":", 1)[0]
    if not host:
        return "WAVE_6_SERVICE_URL host is required"
    if all(ch.isdigit() or ch == "." for ch in host):
        return "WAVE_6_SERVICE_URL must use a staging DNS name, not a raw IP"
    if any(token in host_port_path for token in BLOCKED_SERVICE_URL_TOKENS):
        return "WAVE_6_SERVICE_URL contains blocked boundary token"
    if not _has_environment_token(host_port_path, "staging"):
        return "WAVE_6_SERVICE_URL must contain staging boundary token"
    return None


def command_block(command: str, flags: tuple[tuple[str, str], ...]) -> str:
    def render(flag: str, value: str) -> str:
        return f"{flag} {value}" if value else flag

    lines = [f"{command} \\"]
    for flag, value in flags[:-1]:
        lines.append(f"  {render(flag, value)} \\")
    last_flag, last_value = flags[-1]
    lines.append(f"  {render(last_flag, last_value)}")
    return "\n".join(lines)


def readiness_command() -> str:
    return "just wave-6-deploy-readiness --from-env --json"


def deploy_audit_flags(*, check_only: bool) -> tuple[tuple[str, str], ...]:
    flags: list[tuple[str, str]] = []
    if check_only:
        flags.append(("--check-only", ""))
    flags.extend([
        ("--environment", "$WAVE_6_ENVIRONMENT"),
        ("--deployment-mode", "$WAVE_6_DEPLOYMENT_MODE"),
        ("--module", '"$WAVE_6_DEPLOY_MODULE"'),
        ("--action", '"$WAVE_6_DEPLOY_ACTION"'),
        ("--resource-type", '"$WAVE_6_DEPLOY_RESOURCE_TYPE"'),
        ("--resource-id", '"$WAVE_6_DEPLOY_RESOURCE_ID"'),
        ("--release-version", '"$WAVE_6_RELEASE_VERSION"'),
        ("--release-plan-ref", '"$WAVE_6_RELEASE_PLAN_REF"'),
        ("--artifact-ref", '"$WAVE_6_ARTIFACT_REF"'),
        ("--canary-config-ref", '"$WAVE_6_CANARY_CONFIG_REF"'),
        ("--smoke-gate-ref", '"$WAVE_6_SMOKE_GATE_REF"'),
        ("--observability-dashboard-ref", '"$WAVE_6_OBSERVABILITY_DASHBOARD_REF"'),
        ("--rollback-drill-log-ref", '"$WAVE_6_ROLLBACK_DRILL_LOG_REF"'),
        ("--approval-record-ref", '"$WAVE_6_APPROVAL_RECORD_REF"'),
        ("--canary-stages-exercised", "$WAVE_6_CANARY_STAGES_EXERCISED"),
        ("--smoke-checks-passed", "$WAVE_6_SMOKE_CHECKS_PASSED"),
        ("--rollback-drills-exercised", "$WAVE_6_ROLLBACK_DRILLS_EXERCISED"),
        ("--actor-id", '"$WAVE_6_DEPLOY_ACTOR_ID"'),
        ("--actor-name", '"$WAVE_6_DEPLOY_ACTOR_NAME"'),
        ("--owner-id", '"$WAVE_6_DEPLOY_OWNER_ID"'),
        ("--jti", '"$WAVE_6_DEPLOY_JTI"'),
    ])
    return tuple(flags)


def deploy_audit_check_only_command() -> str:
    return "just wave-6-deploy-audit --from-env --check-only"


def deploy_audit_record_command() -> str:
    return "just wave-6-deploy-audit --from-env"


def record_command() -> str:
    return "just wave-6-deploy-evidence-record --from-env --json"


def record_check_only_command() -> str:
    return "just wave-6-deploy-evidence-record --from-env --check-only --json"


def unique_env_vars(names: list[str]) -> list[str]:
    return list(dict.fromkeys(names))


def export_template() -> str:
    lines = [
        "# Wave 6 deploy materials export template",
        "# This template does not contain secrets and does not write runtime evidence.",
        "# Fill refs from real staging release systems before running readiness or record.",
    ]
    for name in unique_env_vars(list(REQUIRED_ENV_VARS)):
        lines.append("")
        lines.append(f"# source: {ENV_SOURCE_HINTS[name]}")
        if name == "WAVE_6_AUDIT_EVENT_QUERY_REF":
            lines.append("# leave blank until deploy_audit_record succeeds")
        lines.append(f"export {name}=\"{EXPORT_TEMPLATE_DEFAULTS.get(name, '')}\"")
    lines.extend([
        "",
        "just wave-6-deploy-materials --from-env --json",
        "just wave-6-deploy-audit --from-env --check-only",
        "just wave-6-deploy-audit --from-env",
        "just wave-6-deploy-readiness --from-env --json",
        "just wave-6-deploy-evidence-record --from-env --check-only --json",
        "just wave-6-deploy-evidence-record --from-env --json",
        "just wave-6-deploy-evidence-validate",
    ])
    return "\n".join(lines) + "\n"


def execution_plan() -> list[dict[str, object]]:
    service_vars = [
        "WAVE_6_ENVIRONMENT",
        "WAVE_6_DEPLOYMENT_MODE",
        "WAVE_6_RELEASE_VERSION",
        "WAVE_6_SERVICE_URL",
    ]
    release_vars = [
        "WAVE_6_ENVIRONMENT",
        "WAVE_6_DEPLOYMENT_MODE",
        "WAVE_6_RELEASE_VERSION",
    ]
    evidence_ref_vars = list(STRING_ENV_FIELDS.keys())
    deploy_audit_vars = list(DEPLOY_AUDIT_ENV_FIELDS.keys())
    count_vars = list(COUNT_ENV_FIELDS.keys())
    flag_vars = list(FLAG_ENV_FIELDS.keys())
    return [
        {
            "step": "materials",
            "command": "just wave-6-deploy-materials --from-env --json",
            "requires_env": [],
            "checks_env": unique_env_vars(list(REQUIRED_ENV_VARS)),
            "writes_audit_event": False,
            "writes_runtime_evidence": False,
            "closes_gate": False,
        },
        {
            "step": "deploy_audit_check_only",
            "command": deploy_audit_check_only_command(),
            "requires_env": unique_env_vars([
                *release_vars,
                *deploy_audit_vars,
                *[name for name in evidence_ref_vars if name != "WAVE_6_AUDIT_EVENT_QUERY_REF"],
                *count_vars,
            ]),
            "writes_audit_event": False,
            "writes_runtime_evidence": False,
            "closes_gate": False,
        },
        {
            "step": "deploy_audit_record",
            "command": deploy_audit_record_command(),
            "requires_env": unique_env_vars([
                *release_vars,
                *deploy_audit_vars,
                *[name for name in evidence_ref_vars if name != "WAVE_6_AUDIT_EVENT_QUERY_REF"],
                *count_vars,
            ]),
            "writes_audit_event": True,
            "writes_runtime_evidence": False,
            "closes_gate": False,
        },
        {
            "step": "readiness",
            "command": readiness_command(),
            "requires_env": unique_env_vars([
                *service_vars,
                *evidence_ref_vars,
                *count_vars,
                *flag_vars,
            ]),
            "writes_audit_event": False,
            "writes_runtime_evidence": False,
            "closes_gate": False,
        },
        {
            "step": "evidence_record_check_only",
            "command": record_check_only_command(),
            "requires_env": unique_env_vars([
                *release_vars,
                *evidence_ref_vars,
                *count_vars,
                *flag_vars,
            ]),
            "writes_audit_event": False,
            "writes_runtime_evidence": False,
            "closes_gate": False,
        },
        {
            "step": "evidence_record",
            "command": record_command(),
            "requires_env": unique_env_vars([
                *release_vars,
                *evidence_ref_vars,
                *count_vars,
                *flag_vars,
            ]),
            "writes_audit_event": False,
            "writes_runtime_evidence": True,
            "closes_gate": False,
        },
        {
            "step": "validate",
            "command": "just wave-6-deploy-evidence-validate",
            "requires_env": [],
            "writes_audit_event": False,
            "writes_runtime_evidence": False,
            "closes_gate": False,
        },
    ]


def required_env_for_step(step_name: str) -> list[str]:
    for step in execution_plan():
        if step["step"] == step_name:
            return list(step["requires_env"])
    return []


def missing_for(env: Mapping[str, str], names: list[str]) -> list[str]:
    return [name for name in names if not env_value(env, name)]


def missing_env_by_stage(env: Mapping[str, str]) -> dict[str, list[str]]:
    pre_audit_required = required_env_for_step("deploy_audit_check_only")
    post_audit_required = required_env_for_step("readiness")
    evidence_record_required = required_env_for_step("evidence_record_check_only")
    return {
        "pre_audit": missing_for(env, pre_audit_required),
        "post_audit": missing_for(env, post_audit_required),
        "evidence_record": missing_for(env, evidence_record_required),
    }


def next_blocking_stage(grouped_missing: dict[str, list[str]]) -> str | None:
    for stage in ("pre_audit", "post_audit", "evidence_record"):
        if grouped_missing.get(stage):
            return stage
    return None


def collect(env: Mapping[str, str]) -> dict[str, object]:
    missing = missing_env_vars(env)
    staged_missing = missing_env_by_stage(env)
    payload, invalid = build_payload(env)
    environment = str(payload.get("environment", "")).strip().lower()
    if environment and environment != "staging":
        invalid.append(
            "WAVE_6_ENVIRONMENT must be staging; W6.H cannot be closed by dev evidence",
        )
    service_url = env_value(env, "WAVE_6_SERVICE_URL")
    if service_url:
        service_url_issue = validate_staging_service_url(service_url)
        if service_url_issue:
            invalid.append(service_url_issue)
    validator_ok = False
    validator_message = "not run: missing or invalid environment variables"
    if not missing and not invalid:
        validator_ok, validator_message = validate_wave6_deploy_payload(payload)
        if not validator_ok:
            invalid.append(validator_message)

    ok = not missing and not invalid and validator_ok
    return {
        "check": "report_wave6_deploy_materials",
        "tier": "T1",
        "category": "流程治理",
        "ok": ok,
        "schema_version": 1,
        "mode": "wave6-deploy-materials",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-6-deploy-evidence.json",
        "missing_env_vars": missing,
        "missing_env_by_stage": staged_missing,
        "next_blocking_stage": next_blocking_stage(staged_missing),
        "stage_guidance": (
            "fix only next_blocking_stage first; post_audit waits for "
            "deploy_audit_record output"
        ),
        "invalid_env_vars": invalid,
        "validator_message": validator_message,
        "deploy_audit_check_only_command": deploy_audit_check_only_command(),
        "deploy_audit_record_command": deploy_audit_record_command(),
        "readiness_command": readiness_command(),
        "record_check_only_command": record_check_only_command(),
        "record_command": record_command(),
        "validate_command": "just wave-6-deploy-evidence-validate",
        "execution_plan": execution_plan(),
    }


def print_text(payload: dict[str, object]) -> None:
    if payload["ok"]:
        print("✓ Wave 6 deploy materials complete")
    else:
        print("✘ Wave 6 deploy materials incomplete", file=sys.stderr)
    print("不会写入 runtime evidence；不能关闭 W6.H gate", file=sys.stderr)
    print(
        f"next blocking stage: {payload['next_blocking_stage'] or 'none'}",
        file=sys.stderr,
    )
    print(f"stage guidance: {payload['stage_guidance']}", file=sys.stderr)
    grouped = payload["missing_env_by_stage"]
    assert isinstance(grouped, dict)
    for stage in ("pre_audit", "post_audit", "evidence_record"):
        missing = grouped.get(stage, [])
        assert isinstance(missing, list)
        summary = ", ".join(str(name) for name in missing) if missing else "none"
        print(f"{stage} missing: {summary}", file=sys.stderr)
    print(
        "do not fake WAVE_6_AUDIT_EVENT_QUERY_REF before deploy audit",
        file=sys.stderr,
    )
    for name in payload["missing_env_vars"]:
        print(f"missing: {name}", file=sys.stderr)
    for item in payload["invalid_env_vars"]:
        print(f"invalid: {item}", file=sys.stderr)
    print("deploy audit check-only:")
    print(payload["deploy_audit_check_only_command"])
    print("deploy audit record:")
    print(payload["deploy_audit_record_command"])
    print("readiness:")
    print(payload["readiness_command"])
    print("record check-only:")
    print(payload["record_check_only_command"])
    print("record:")
    print(payload["record_command"])
    print(f"validate: {payload['validate_command']}")


def main(
    argv: list[str] | None = None,
    *,
    env: Mapping[str, str] | None = None,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "--export-template",
        action="store_true",
        help="Print non-secret W6.H export template; does not read env or write evidence.",
    )
    args = parser.parse_args(argv)

    if args.export_template:
        print(export_template(), end="")
        return 0

    payload = collect(os.environ if env is None else env)
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print_text(payload)
    return 0 if payload["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
