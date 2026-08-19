"""Field handoff payloads for Wave 3 PDA runtime readiness."""
import json
import os
from pathlib import Path

from check_wave3_pda_runtime_readiness import *  # noqa: F403

def sanitized_facts(facts: dict[str, object]) -> dict[str, object]:
    sanitized = dict(facts)
    for key in ("service_url", "openapi_url"):
        if key in sanitized:
            sanitized[key] = sanitize_url_for_output(sanitized[key])
    return sanitized

def trace_code_openapi_payload(
    ok: bool,
    facts: dict[str, object],
    issues: list[str],
    missing_env_vars: list[str],
) -> dict[str, object]:
    payload: dict[str, object] = {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": ok,
        "schema_version": 1,
        "mode": "wave3-pda-trace-code-openapi-precheck",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "external_prerequisites": list(W6D_EXTERNAL_PREREQUISITES),
        "minimum_evidence_refs": list(W6D_MINIMUM_EVIDENCE_REFS),
        "facts": sanitized_facts(facts),
        "issues": issues,
        "troubleshooting": trace_code_openapi_troubleshooting(facts),
        "next_commands": W6D_NEXT_COMMANDS,
    }
    if missing_env_vars:
        payload["missing_env_vars"] = missing_env_vars
        payload["missing_env_var_owners"] = missing_trace_code_env_var_owner_details(
            missing_env_vars,
        )
    return payload

def trace_code_openapi_troubleshooting(
    facts: dict[str, object],
) -> list[str]:
    tips = list(TRACE_CODE_OPENAPI_TROUBLESHOOTING)
    if facts.get("status") == 502:
        tips.insert(
            0,
            "HTTP 502 is often produced by the proxy path for this endpoint; "
            "verify direct no-proxy access before escalating the OpenAPI service.",
        )
    return tips

def result_payload(
    ok: bool,
    facts: dict[str, object],
    issues: list[str],
    *,
    service_precheck_only: bool = False,
) -> dict[str, object]:
    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": ok,
        "schema_version": 1,
        "mode": (
            "wave3-pda-service-precheck"
            if service_precheck_only
            else "wave3-pda-runtime-readiness"
        ),
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "external_prerequisites": list(W6D_EXTERNAL_PREREQUISITES),
        "minimum_evidence_refs": list(W6D_MINIMUM_EVIDENCE_REFS),
        "facts": sanitized_facts(facts),
        "issues": issues,
        "next_commands": W6D_NEXT_COMMANDS,
    }

def materials_checklist_payload() -> dict[str, object]:
    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": True,
        "schema_version": 1,
        "mode": "wave3-pda-materials-checklist",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "external_prerequisites": list(W6D_EXTERNAL_PREREQUISITES),
        "minimum_evidence_refs": list(W6D_MINIMUM_EVIDENCE_REFS),
        "facts": {},
        "issues": [],
        "fields": [dict(field) for field in MATERIALS_CHECKLIST_FIELDS],
        "next_commands": W6D_NEXT_COMMANDS,
    }

def field_work_request_payload() -> dict[str, object]:
    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": True,
        "schema_version": 1,
        "mode": "wave3-pda-field-work-request",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "resources": [dict(resource) for resource in FIELD_WORK_RESOURCES],
        "execution_order_zh": list(FIELD_WORK_EXECUTION_ORDER_ZH),
        "troubleshooting": list(FIELD_WORK_TROUBLESHOOTING),
        "next_commands": W6D_NEXT_COMMANDS,
    }

def load_field_precheck_attachment(path_text: str | None) -> dict[str, object] | None:
    if not path_text:
        return None
    path = Path(path_text)
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("field precheck attachment must be a JSON object")
    if payload.get("kind") != FIELD_PRECHECK_ATTACHMENT_KIND:
        raise ValueError(
            "field precheck attachment kind must be "
            f"{FIELD_PRECHECK_ATTACHMENT_KIND}",
        )
    if payload.get("writes_runtime_evidence") is not False:
        raise ValueError("field precheck attachment must not write runtime evidence")
    if payload.get("closes_gate") is not False:
        raise ValueError("field precheck attachment must not close W6.D")
    if (
        payload.get("runtime_evidence_file")
        != FIELD_PRECHECK_ATTACHMENT_RUNTIME_EVIDENCE_FILE
    ):
        raise ValueError(
            "field precheck attachment runtime_evidence_file must be "
            f"{FIELD_PRECHECK_ATTACHMENT_RUNTIME_EVIDENCE_FILE}",
        )
    service_precheck = payload.get("service_precheck")
    trace_code_precheck = payload.get("trace_code_openapi_precheck")
    if not isinstance(service_precheck, dict):
        raise ValueError("field precheck attachment service_precheck must be an object")
    if not isinstance(trace_code_precheck, dict):
        raise ValueError(
            "field precheck attachment trace_code_openapi_precheck must be an object",
        )
    if bool(service_precheck.get("ok")):
        validate_field_precheck_attachment_service(service_precheck)
    if bool(trace_code_precheck.get("ok")):
        validate_field_precheck_attachment_trace_code(trace_code_precheck)
    return {
        "path": str(path),
        "kind": payload["kind"],
        "service_precheck_ok": bool(service_precheck.get("ok")),
        "trace_code_openapi_precheck_ok": bool(trace_code_precheck.get("ok")),
        "writes_runtime_evidence": False,
        "closes_gate": False,
    }

def validate_field_precheck_attachment_service(
    service_precheck: dict[str, object],
) -> None:
    if service_precheck.get("environment") not in {"dev", "staging"}:
        raise ValueError(
            "field precheck attachment service_precheck.environment must be dev or staging",
        )
    service_url = str(service_precheck.get("service_url", "")).strip()
    if not service_url:
        raise ValueError(
            "field precheck attachment service_precheck.service_url is required",
        )
    url_issues = sensitive_url_issues(
        service_url,
        field_name="service_precheck.service_url",
    )
    if url_issues:
        raise ValueError(f"field precheck attachment {url_issues[0]}")
    lowered = service_url.lower()
    if any(token in lowered for token in BLOCKED_SERVICE_URL_TOKENS):
        raise ValueError(
            "field precheck attachment service_precheck.service_url cannot point "
            "to local/prod/production/mock/fake/stub/example",
        )
    if service_precheck.get("healthz_status") != 200:
        raise ValueError(
            "field precheck attachment service_precheck.healthz_status must be 200",
        )
    if service_precheck.get("healthz_payload_status") != "ok":
        raise ValueError(
            "field precheck attachment service_precheck.healthz_payload_status must be ok",
        )
    if service_precheck.get("wave3_route_status") != 401:
        raise ValueError(
            "field precheck attachment service_precheck.wave3_route_status must be 401",
        )
    if service_precheck.get("wave3_route_error_code") != EXPECTED_WAVE3_UNAUTHORIZED_CODE:
        raise ValueError(
            "field precheck attachment service_precheck.wave3_route_error_code "
            f"must be {EXPECTED_WAVE3_UNAUTHORIZED_CODE}",
        )

def validate_field_precheck_attachment_trace_code(
    trace_code_precheck: dict[str, object],
) -> None:
    openapi_url = str(trace_code_precheck.get("openapi_url", "")).strip()
    if openapi_url:
        url_issues = sensitive_url_issues(
            openapi_url,
            field_name="trace_code_openapi_precheck.openapi_url",
        )
        if url_issues:
            raise ValueError(f"field precheck attachment {url_issues[0]}")
    if trace_code_precheck.get("status") != 200:
        raise ValueError(
            "field precheck attachment trace_code_openapi_precheck.status must be 200",
        )
    if trace_code_precheck.get("openapi") != "3.0.3":
        raise ValueError(
            "field precheck attachment trace_code_openapi_precheck.openapi must be 3.0.3",
        )
    if trace_code_precheck.get("api_key_header_name") != "X-API-Key":
        raise ValueError(
            "field precheck attachment trace_code_openapi_precheck.api_key_header_name "
            "must be X-API-Key",
        )
    present_paths = trace_code_precheck.get("required_paths_present")
    if not isinstance(present_paths, list):
        raise ValueError(
            "field precheck attachment trace_code_openapi_precheck."
            "required_paths_present must be a list",
        )
    missing_paths = [
        path
        for path in TRACE_CODE_REQUIRED_PATHS
        if path not in present_paths
    ]
    if missing_paths:
        raise ValueError(
            "field precheck attachment trace_code_openapi_precheck missing "
            f"required paths: {', '.join(missing_paths)}",
        )
    present_operations = trace_code_precheck.get("required_operations_present")
    if present_operations is not None:
        if not isinstance(present_operations, list):
            raise ValueError(
                "field precheck attachment trace_code_openapi_precheck."
                "required_operations_present must be a list",
            )
        missing_operations = [
            operation
            for operation in TRACE_CODE_REQUIRED_OPERATION_LABELS
            if operation not in present_operations
        ]
        if missing_operations:
            raise ValueError(
                "field precheck attachment trace_code_openapi_precheck missing "
                f"required operations: {', '.join(missing_operations)}",
            )

def precheck_attachment_satisfied_env_vars(
    attachment: dict[str, object] | None,
) -> list[str]:
    if attachment is None:
        return []
    satisfied: list[str] = []
    if bool(attachment["service_precheck_ok"]):
        satisfied.extend([
            "WAVE_3_PDA_ENVIRONMENT",
            "WAVE_3_PDA_SERVICE_URL",
        ])
    if bool(attachment["trace_code_openapi_precheck_ok"]):
        satisfied.extend([
            "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
            "WAVE_3_PDA_TRACE_CODE_API_KEY",
        ])
    return [
        env_var
        for env_var in PREAUDIT_REQUIRED_NOW_ENV_VARS
        if env_var in satisfied
    ]

def precheck_attachment_satisfied_truth_flag_env_vars(
    attachment: dict[str, object] | None,
) -> list[str]:
    if attachment is None:
        return []
    satisfied: list[str] = []
    if bool(attachment["service_precheck_ok"]):
        satisfied.append(ENV_FLAG_FIELDS["dev_or_staging_service_verified"])
    return [
        env_var
        for env_var in NO_PDA_PRECHECK_FLAG_ENV_VARS
        if env_var in satisfied
    ]

def field_execution_summary_payload(
    field_precheck_attachment: dict[str, object] | None = None,
) -> dict[str, object]:
    stack_candidate = os.environ.get(
        ENV_STRING_FIELDS["pda_stack_candidate"],
        "",
    ).strip()
    webview_only_env_vars = {
        ENV_STRING_FIELDS[field]
        for field in WEBVIEW_CAPACITOR_FIELDS
    }
    real_pda_required_env_vars = [
        str(field["name"])
        for field in MATERIALS_CHECKLIST_FIELDS
        if bool(field["requires_real_pda"])
        and (
            str(field["name"]) not in webview_only_env_vars
            or stack_candidate == "webview-capacitor"
        )
    ]
    real_pda_missing_env_vars = [
        env_var
        for env_var in real_pda_required_env_vars
        if not os.environ.get(env_var, "").strip()
    ]
    truth_flag_env_vars = list(ENV_FLAG_FIELDS.values())
    no_pda_precheck_truth_flag_env_vars = list(NO_PDA_PRECHECK_FLAG_ENV_VARS)
    real_evidence_truth_flag_env_vars = list(REAL_EVIDENCE_FLAG_ENV_VARS)
    satisfied_truth_flag_env_vars = precheck_attachment_satisfied_truth_flag_env_vars(
        field_precheck_attachment,
    )
    false_truth_flag_env_vars = [
        env_var
        for env_var in truth_flag_env_vars
        if os.environ.get(env_var, "").strip().lower() not in TRUE_ENV_VALUES
        and env_var not in satisfied_truth_flag_env_vars
    ]
    false_no_pda_precheck_truth_flag_env_vars = [
        env_var
        for env_var in no_pda_precheck_truth_flag_env_vars
        if env_var in false_truth_flag_env_vars
    ]
    false_real_evidence_truth_flag_env_vars = [
        env_var
        for env_var in real_evidence_truth_flag_env_vars
        if env_var in false_truth_flag_env_vars
    ]
    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": True,
        "schema_version": 1,
        "mode": "wave3-pda-field-execution-summary",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "external_prerequisites": list(W6D_EXTERNAL_PREREQUISITES),
        "minimum_evidence_refs": list(W6D_MINIMUM_EVIDENCE_REFS),
        "current_env_status": preaudit_current_env_status(
            field_precheck_attachment,
        ),
        "no_pda_precheck_commands": [
            "just wave-3-pda-service-precheck --from-env --json",
            "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
            "just wave-3-pda-field-precheck-summary --from-env --json",
        ],
        "field_package_commands": [
            "just wave-3-pda-preaudit-kit --json",
            "just wave-3-pda-materials-checklist --json",
            "just wave-3-pda-field-work-request",
            "just wave-3-pda-evidence-package-template",
            "just wave-3-pda-runtime-evidence-record --export-template",
        ],
        "real_pda_required_env_vars": real_pda_required_env_vars,
        "real_pda_missing_env_vars": real_pda_missing_env_vars,
        "real_pda_missing_env_var_owners": missing_env_var_owner_details(
            real_pda_missing_env_vars,
        ),
        "truth_flag_env_vars": truth_flag_env_vars,
        "no_pda_precheck_truth_flag_env_vars": no_pda_precheck_truth_flag_env_vars,
        "truth_flags_must_remain_false_until_refs_present": real_evidence_truth_flag_env_vars,
        "satisfied_by_precheck_attachment_truth_flag_env_vars": (
            satisfied_truth_flag_env_vars
        ),
        "false_truth_flag_env_vars": false_truth_flag_env_vars,
        "false_truth_flag_env_var_owners": missing_env_var_owner_details(
            false_truth_flag_env_vars,
        ),
        "false_no_pda_precheck_truth_flag_env_vars": false_no_pda_precheck_truth_flag_env_vars,
        "false_no_pda_precheck_truth_flag_env_var_owners": (
            missing_env_var_owner_details(false_no_pda_precheck_truth_flag_env_vars)
        ),
        "false_real_evidence_truth_flag_env_vars": false_real_evidence_truth_flag_env_vars,
        "false_real_evidence_truth_flag_env_var_owners": (
            missing_env_var_owner_details(false_real_evidence_truth_flag_env_vars)
        ),
        "ready_for_record_from_env_vars": not real_pda_missing_env_vars
        and all(os.environ.get(env_var, "").strip().lower() in TRUE_ENV_VALUES for env_var in truth_flag_env_vars),
        "record_commands": list(FIELD_WORK_RECORD_COMMANDS),
        "record_command_note": (
            "from-env record and intake-record are alternative formal write paths; "
            "run one formal record path, then validate."
        ),
        "must_not_do": list(PREAUDIT_MUST_NOT_DO),
        "next_commands": W6D_NEXT_COMMANDS,
    }

def owner_gap_actions_from_summary(
    field_summary_payload: dict[str, object],
) -> list[dict[str, object]]:
    grouped: dict[str, dict[str, object]] = {}

    def ensure_action(owner_detail: dict[str, object]) -> dict[str, object]:
        source_owner = str(owner_detail["source_owner"])
        action = grouped.get(source_owner)
        if action is None:
            action = {
                "source_owner": source_owner,
                "action": "补齐缺失环境变量或真实 evidence 引用",
                "next_action": "补齐缺失环境变量或真实 evidence 引用",
                "env_vars": [],
                "missing_now_env_vars": [],
                "missing_env_vars": [],
                "false_truth_flag_env_vars": [],
                "evidence_requirements": [],
                "no_pda_stages": [],
                "requires_real_pda": False,
            }
            grouped[source_owner] = action
        return action

    def append_unique(target: list[str], value: object) -> None:
        text = str(value)
        if text not in target:
            target.append(text)

    def add_detail(owner_detail: dict[str, object], bucket: str) -> None:
        action = ensure_action(owner_detail)
        env_var = owner_detail["env_var"]
        append_unique(action["env_vars"], env_var)
        append_unique(action[bucket], env_var)
        append_unique(
            action["evidence_requirements"],
            owner_detail["evidence_requirement"],
        )
        append_unique(action["no_pda_stages"], owner_detail["no_pda_stage"])
        if bool(owner_detail["requires_real_pda"]):
            action["requires_real_pda"] = True

    current_env_status = field_summary_payload.get("current_env_status", {})
    for owner_detail in current_env_status.get("missing_now_env_var_owners", []):
        add_detail(owner_detail, "missing_now_env_vars")
    for owner_detail in field_summary_payload.get("real_pda_missing_env_var_owners", []):
        add_detail(owner_detail, "missing_env_vars")
    for owner_detail in field_summary_payload.get("false_truth_flag_env_var_owners", []):
        add_detail(owner_detail, "false_truth_flag_env_vars")

    return sorted(
        grouped.values(),
        key=lambda item: str(item["source_owner"]),
    )

def field_owner_gap_actions_payload(
    field_precheck_attachment: dict[str, object] | None = None,
) -> dict[str, object]:
    field_summary = field_execution_summary_payload(field_precheck_attachment)
    actions = owner_gap_actions_from_summary(field_summary)
    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": True,
        "schema_version": 1,
        "mode": "wave3-pda-field-owner-gap-actions",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "external_prerequisites": list(W6D_EXTERNAL_PREREQUISITES),
        "minimum_evidence_refs": list(W6D_MINIMUM_EVIDENCE_REFS),
        "field_execution_summary": field_summary,
        "field_owner_gap_actions": actions,
        "gap_action_count": len(actions),
        "ready_for_record_from_env_vars": field_summary[
            "ready_for_record_from_env_vars"
        ],
        "must_not_do": list(PREAUDIT_MUST_NOT_DO),
        "next_commands": W6D_NEXT_COMMANDS,
    }

def service_precheck_payload_from_args(
    args: argparse.Namespace,
) -> dict[str, object]:
    original_service_precheck_only = args.service_precheck_only
    args.service_precheck_only = True
    try:
        ok, facts, issues = check_readiness(args)
    except (ReadinessError, OSError, ValueError) as error:
        ok = False
        facts = {
            "environment": args.environment,
            "service_url": args.service_url,
            "health_path": args.health_path,
            "wave3_route_path": args.wave3_route_path,
            "service_precheck_only": True,
        }
        issues = [str(error)]
    finally:
        args.service_precheck_only = original_service_precheck_only

    payload = result_payload(ok, facts, issues, service_precheck_only=True)
    missing_env_vars = missing_env_vars_for_issues(issues)
    if missing_env_vars:
        payload["missing_env_vars"] = missing_env_vars
        payload["missing_env_var_owners"] = missing_env_var_owner_details(
            missing_env_vars,
        )
    return payload

def trace_code_openapi_precheck_payload_from_args(
    args: argparse.Namespace,
) -> dict[str, object]:
    try:
        ok, facts, issues, missing_env_vars = check_trace_code_openapi(args)
    except (ReadinessError, OSError, ValueError) as error:
        ok = False
        facts = {
            "openapi_url": getattr(args, "trace_code_openapi_url", None),
            "required_paths": list(TRACE_CODE_REQUIRED_PATHS),
        }
        issues = [str(error)]
        missing_env_vars = []
    return trace_code_openapi_payload(ok, facts, issues, missing_env_vars)

def no_pda_precheck_verified_flag_env_vars(
    service_precheck_payload: dict[str, object],
) -> list[str]:
    if bool(service_precheck_payload["ok"]):
        return list(NO_PDA_PRECHECK_FLAG_ENV_VARS)
    return []

def field_precheck_summary_payload(
    service_precheck_payload: dict[str, object],
    trace_code_precheck_payload: dict[str, object],
    field_summary_payload: dict[str, object],
) -> dict[str, object]:
    issues = [
        f"service: {issue}"
        for issue in service_precheck_payload.get("issues", [])
    ]
    issues.extend(
        f"trace-code-openapi: {issue}"
        for issue in trace_code_precheck_payload.get("issues", [])
    )
    ok = bool(service_precheck_payload["ok"]) and bool(
        trace_code_precheck_payload["ok"],
    )
    verified_flag_env_vars = no_pda_precheck_verified_flag_env_vars(
        service_precheck_payload,
    )
    false_no_pda_flag_env_vars = field_summary_payload.get(
        "false_no_pda_precheck_truth_flag_env_vars",
        [],
    )
    false_real_evidence_flag_env_vars = field_summary_payload.get(
        "false_real_evidence_truth_flag_env_vars",
        [],
    )
    remaining_no_pda_flag_env_vars = [
        env_var
        for env_var in false_no_pda_flag_env_vars
        if env_var not in verified_flag_env_vars
    ]
    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": ok,
        "schema_version": 1,
        "mode": "wave3-pda-field-precheck-summary",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "external_prerequisites": list(W6D_EXTERNAL_PREREQUISITES),
        "minimum_evidence_refs": list(W6D_MINIMUM_EVIDENCE_REFS),
        "service_precheck": service_precheck_payload,
        "trace_code_openapi_precheck": trace_code_precheck_payload,
        "field_execution_summary": field_summary_payload,
        "no_pda_precheck_verified_flag_env_vars": verified_flag_env_vars,
        "no_pda_precheck_verified_flag_env_var_owners": (
            missing_env_var_owner_details(verified_flag_env_vars)
        ),
        "remaining_no_pda_precheck_false_flag_env_vars": remaining_no_pda_flag_env_vars,
        "remaining_no_pda_precheck_false_flag_env_var_owners": (
            missing_env_var_owner_details(remaining_no_pda_flag_env_vars)
        ),
        "remaining_real_evidence_false_flag_env_vars": list(
            false_real_evidence_flag_env_vars,
        ),
        "remaining_real_evidence_false_flag_env_var_owners": (
            missing_env_var_owner_details(false_real_evidence_flag_env_vars)
        ),
        "issues": issues,
        "next_commands": W6D_NEXT_COMMANDS,
    }

def evidence_package_template_payload_for_handoff() -> dict[str, object]:
    from record_wave3_pda_runtime_evidence import package_template_payload

    return package_template_payload(
        Path("docs/retros/wave-3-pda-runtime-evidence.json"),
    )

def intake_template_payload_for_handoff() -> dict[str, object]:
    from record_wave3_pda_runtime_evidence import intake_template_payload

    return intake_template_payload(
        Path("docs/retros/wave-3-pda-runtime-evidence.json"),
    )

def field_handoff_bundle_payload(
    args: argparse.Namespace,
    *,
    include_precheck: bool = False,
    field_precheck_attachment: dict[str, object] | None = None,
) -> dict[str, object]:
    field_summary = field_execution_summary_payload(field_precheck_attachment)
    owner_gap_payload = field_owner_gap_actions_payload(field_precheck_attachment)
    field_precheck_payload = None
    if include_precheck:
        apply_env_args(args, service_precheck_only=True)
        apply_trace_code_env_args(args)
        field_precheck_payload = field_precheck_summary_payload(
            service_precheck_payload_from_args(args),
            trace_code_openapi_precheck_payload_from_args(args),
            field_summary,
        )

    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": True if field_precheck_payload is None else bool(field_precheck_payload["ok"]),
        "schema_version": 1,
        "mode": "wave3-pda-field-handoff-bundle",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "bundle_scope": [
            "preaudit_kit",
            "materials_checklist",
            "field_work_request",
            "field_execution_summary",
            "field_owner_gap_actions",
            "evidence_package_template",
            "intake_template",
            "field_precheck_summary_from_env" if include_precheck else "field_precheck_summary_not_run",
        ],
        "preaudit_kit": preaudit_kit_payload(field_precheck_attachment),
        "materials_checklist": materials_checklist_payload(),
        "field_work_request": field_work_request_payload(),
        "field_execution_summary": field_summary,
        "field_owner_gap_actions": owner_gap_payload,
        "evidence_package_template": evidence_package_template_payload_for_handoff(),
        "intake_template": intake_template_payload_for_handoff(),
        "field_precheck_summary": field_precheck_payload,
        "ready_for_record_from_env_vars": field_summary[
            "ready_for_record_from_env_vars"
        ],
        "gap_action_count": owner_gap_payload["gap_action_count"],
        "real_pda_missing_env_vars_count": len(
            field_summary["real_pda_missing_env_vars"],
        ),
        "false_truth_flag_env_vars_count": len(
            field_summary["false_truth_flag_env_vars"],
        ),
        "include_precheck": include_precheck,
        "must_not_do": list(PREAUDIT_MUST_NOT_DO),
        "next_commands": W6D_NEXT_COMMANDS,
    }

def write_field_handoff_bundle(
    path: Path,
    payload: dict[str, object],
    *,
    force: bool = False,
) -> tuple[bool, str]:
    if path.exists() and not force:
        return False, f"{path} already exists; pass --field-handoff-force to overwrite"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return True, f"wrote {path}"

def preaudit_current_env_status(
    field_precheck_attachment: dict[str, object] | None = None,
) -> dict[str, object]:
    set_env_vars = [
        env_var
        for env_var in PREAUDIT_REQUIRED_NOW_ENV_VARS
        if os.environ.get(env_var, "").strip()
    ]
    satisfied_by_attachment = precheck_attachment_satisfied_env_vars(
        field_precheck_attachment,
    )
    missing_env_vars = [
        env_var
        for env_var in PREAUDIT_REQUIRED_NOW_ENV_VARS
        if env_var not in set_env_vars and env_var not in satisfied_by_attachment
    ]
    status = {
        "required_now_env_vars": list(PREAUDIT_REQUIRED_NOW_ENV_VARS),
        "set_now_env_vars": set_env_vars,
        "missing_now_env_vars": missing_env_vars,
        "missing_now_env_var_owners": missing_env_var_owner_details(missing_env_vars),
    }
    if field_precheck_attachment is not None:
        status["satisfied_by_precheck_attachment_env_vars"] = satisfied_by_attachment
        status["precheck_attachment"] = field_precheck_attachment
    return status

def preaudit_kit_payload(
    field_precheck_attachment: dict[str, object] | None = None,
) -> dict[str, object]:
    return {
        "check": "check_wave3_pda_runtime_readiness",
        "tier": "T1",
        "category": "流程治理",
        "ok": True,
        "schema_version": 1,
        "mode": "wave3-pda-preaudit-kit",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
        "preaudit_stage": "before_real_pda_execution",
        "audiences": list(PREAUDIT_AUDIENCES),
        "external_prerequisites": list(W6D_EXTERNAL_PREREQUISITES),
        "minimum_evidence_refs": list(W6D_MINIMUM_EVIDENCE_REFS),
        "current_env_status": preaudit_current_env_status(
            field_precheck_attachment,
        ),
        "now_actions": [dict(action) for action in PREAUDIT_NOW_ACTIONS],
        "blocked_until_real_pda": [
            {"env_var": str(field["name"]), **dict(field)}
            for field in MATERIALS_CHECKLIST_FIELDS
            if bool(field["requires_real_pda"])
        ],
        "must_not_do": list(PREAUDIT_MUST_NOT_DO),
        "resources": [dict(resource) for resource in FIELD_WORK_RESOURCES],
        "execution_order_zh": list(FIELD_WORK_EXECUTION_ORDER_ZH),
        "next_commands": W6D_NEXT_COMMANDS,
    }
