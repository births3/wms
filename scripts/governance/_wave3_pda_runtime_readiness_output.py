"""Text and Markdown output for Wave 3 PDA runtime readiness."""
from typing import Any

from check_wave3_pda_runtime_readiness import *  # noqa: F403
from _wave3_pda_runtime_readiness_field import *  # noqa: F403

def print_materials_checklist_text(payload: dict[str, object]) -> None:
    print("Wave 3 PDA materials checklist")
    print("不会写入 runtime evidence；不能关闭 W6.D gate")
    for field in payload["fields"]:
        print(
            "{name}: owner={source_owner}; no_pda_stage={no_pda_stage}; "
            "requires_real_pda={requires_real_pda}".format(**field),
        )

def print_field_work_request_markdown(payload: dict[str, object]) -> None:
    print("# W6.D PDA Field Work Request")
    print()
    print("This request package is not runtime evidence JSON and cannot close W6.D.")
    print("It is a handoff sheet for field owners before real PDA execution.")
    print()
    print("writes_runtime_evidence=false")
    print("closes_gate=false")
    print()
    print("| Resource | Owner | Deliverable | Verification / variable |")
    print("|----------|-------|-------------|--------------------------|")
    for resource in payload["resources"]:
        print(
            "| {resource} | {owner} | {deliverable} | {verification} |".format(
                **resource,
            )
        )
    print()
    print("## 中文现场工单表")
    print()
    print("| 资源 | 负责人 | 交付物 | 验证变量 / 命令 |")
    print("|------|--------|--------|-----------------|")
    for resource in payload["resources"]:
        print(
            "| {resource_zh} | {owner_zh} | {deliverable_zh} | {verification_zh} |".format(
                **resource,
            )
        )
    print()
    print("## 中文执行顺序")
    print()
    for index, step in enumerate(payload["execution_order_zh"], start=1):
        print(f"{index}. {step}。")
    print()
    print("## Fast Troubleshooting")
    print()
    for item in payload["troubleshooting"]:
        print(f"- {item}")
    print()
    print("## Commands")
    print()
    print("```bash")
    print("just wave-3-pda-service-precheck --from-env --json")
    print("just wave-3-pda-trace-code-openapi-precheck --from-env --json")
    print("just wave-3-pda-runtime-readiness --from-env --json")
    print("just wave-3-pda-runtime-evidence-record --from-env --check-only --json")
    print("just wave-3-pda-runtime-evidence-record --from-env --json")
    print("just wave-3-pda-intake-check --json")
    print("just wave-3-pda-intake-record --json")
    print("just wave-3-pda-runtime-evidence-validate")
    print("```")

def print_field_execution_summary_markdown(payload: dict[str, object]) -> None:
    print("# W6.D PDA Field Execution Summary")
    print()
    print("This summary is read-only. It does not write runtime evidence and cannot close W6.D.")
    print()
    print("## 当前前置变量")
    print()
    current_env_status = payload["current_env_status"]
    print("set_now_env_vars:")
    for env_var in current_env_status["set_now_env_vars"]:
        print(f"- {env_var}")
    print("missing_now_env_vars:")
    for env_var in current_env_status["missing_now_env_vars"]:
        print(f"- {env_var}")
    if not current_env_status["missing_now_env_vars"]:
        print("- none")
    attachment = current_env_status.get("precheck_attachment")
    if attachment:
        print("satisfied_by_precheck_attachment_env_vars:")
        for env_var in current_env_status["satisfied_by_precheck_attachment_env_vars"]:
            print(f"- {env_var}")
        print(f"precheck_attachment_path={attachment['path']}")
    print()
    print("## 真 PDA 仍需字段")
    print()
    for env_var in payload["real_pda_missing_env_vars"]:
        print(f"- {env_var}")
    print()
    print("## 只读预检命令")
    print()
    print("```bash")
    for command in payload["no_pda_precheck_commands"]:
        print(command)
    print("```")
    print()
    print("## 只读预检通过后可置 true 的变量")
    print()
    for env_var in payload["no_pda_precheck_truth_flag_env_vars"]:
        print(f"- {env_var}")
    print()
    print("## 仍未置 true 的布尔变量")
    print()
    for env_var in payload["false_truth_flag_env_vars"]:
        print(f"- {env_var}")

def markdown_code_list(values: object) -> str:
    if not values:
        return "-"
    return ", ".join(f"`{value}`" for value in values)

def markdown_table_cell(value: object) -> str:
    return str(value).replace("|", "\\|")

def print_field_precheck_summary_markdown(payload: dict[str, object]) -> None:
    service_payload = payload["service_precheck"]
    trace_payload = payload["trace_code_openapi_precheck"]
    field_summary = payload["field_execution_summary"]
    service_facts = service_payload.get("facts", {})
    trace_facts = trace_payload.get("facts", {})
    current_env_status = field_summary["current_env_status"]
    required_paths = trace_facts.get("required_paths", [])
    required_paths_present = trace_facts.get("required_paths_present", [])
    missing_required_paths_count = len(required_paths) - len(required_paths_present)

    print("# W6.D PDA Field Precheck Summary")
    print()
    print("This summary is read-only and cannot close W6.D.")
    print()
    print(f"writes_runtime_evidence={str(payload['writes_runtime_evidence']).lower()}")
    print(f"closes_gate={str(payload['closes_gate']).lower()}")
    print(f"evidence_file={payload['evidence_file']}")
    print()
    print("## Service Precheck")
    print()
    print(f"service_precheck.ok={str(service_payload['ok']).lower()}")
    print(f"healthz_status={service_facts.get('healthz_status')}")
    print(f"healthz_payload_status={service_facts.get('healthz_payload_status')}")
    print(f"wave3_route_status={service_facts.get('wave3_route_status')}")
    print(f"wave3_route_error_code={service_facts.get('wave3_route_error_code')}")
    print()
    print("## Trace-code OpenAPI Precheck")
    print()
    print(f"trace_code_openapi_precheck.ok={str(trace_payload['ok']).lower()}")
    print(f"status={trace_facts.get('status')}")
    print(f"openapi={trace_facts.get('openapi')}")
    print(f"title={trace_facts.get('title')}")
    print(f"api_key_header_name={trace_facts.get('api_key_header_name')}")
    print(f"missing_required_paths_count={missing_required_paths_count}")
    print()
    print("## Field Gaps")
    print()
    print(
        "ready_for_record_from_env_vars="
        f"{str(field_summary['ready_for_record_from_env_vars']).lower()}",
    )
    print(
        "missing_now_env_vars_count="
        f"{len(current_env_status['missing_now_env_vars'])}",
    )
    print(
        "real_pda_missing_env_vars_count="
        f"{len(field_summary['real_pda_missing_env_vars'])}",
    )
    print(
        "false_truth_flag_env_vars_count="
        f"{len(field_summary['false_truth_flag_env_vars'])}",
    )
    print()
    print("## Precheck Verified Flags")
    print()
    verified_flags = payload["no_pda_precheck_verified_flag_env_vars"]
    if verified_flags:
        for env_var in verified_flags:
            print(f"- `{env_var}`")
    else:
        print("- none")
    print(
        "remaining_no_pda_precheck_false_flag_env_vars_count="
        f"{len(payload['remaining_no_pda_precheck_false_flag_env_vars'])}",
    )
    print(
        "remaining_real_evidence_false_flag_env_vars_count="
        f"{len(payload['remaining_real_evidence_false_flag_env_vars'])}",
    )
    print()
    print("## Missing Now Env Vars")
    print()
    missing_now_owners = current_env_status["missing_now_env_var_owners"]
    if missing_now_owners:
        for owner_detail in missing_now_owners:
            print(
                f"- `{owner_detail['env_var']}`: "
                f"{owner_detail['source_owner']}",
            )
    else:
        print("- none")
    print()
    print("## Issues")
    print()
    if payload["issues"]:
        for issue in payload["issues"]:
            print(f"- {issue}")
    else:
        print("- none")
    print()
    print("## Commands")
    print()
    print("```bash")
    print("just wave-3-pda-field-precheck-summary --from-env --json")
    print("just wave-3-pda-field-owner-gap-actions")
    for command in field_summary["record_commands"]:
        print(command)
    print("```")

def print_field_owner_gap_actions_markdown(payload: dict[str, object]) -> None:
    field_summary = payload["field_execution_summary"]
    print("# W6.D PDA Owner Gap Actions")
    print()
    print("This handoff is read-only and cannot close W6.D.")
    print()
    print(f"writes_runtime_evidence={str(payload['writes_runtime_evidence']).lower()}")
    print(f"closes_gate={str(payload['closes_gate']).lower()}")
    print(f"evidence_file={payload['evidence_file']}")
    print()
    print("## Summary")
    print()
    print(
        "ready_for_record_from_env_vars="
        f"{str(payload['ready_for_record_from_env_vars']).lower()}",
    )
    print(f"gap_action_count={payload['gap_action_count']}")
    print(
        "real_pda_missing_env_vars_count="
        f"{len(field_summary['real_pda_missing_env_vars'])}",
    )
    print(
        "false_truth_flag_env_vars_count="
        f"{len(field_summary['false_truth_flag_env_vars'])}",
    )
    print()
    print(
        "| Owner | Action | Missing now | Real evidence vars | False flags | "
        "Evidence requirements | Stage | Real PDA? |",
    )
    print(
        "|-------|--------|-------------|--------------------|-------------|"
        "-----------------------|-------|-----------|",
    )
    for action in payload["field_owner_gap_actions"]:
        print(
            "| {owner} | {action_text} | {missing_now} | {missing_evidence} | "
            "{false_flags} | {evidence_requirements} | "
            "{stages} | {requires_real_pda} |".format(
                owner=markdown_table_cell(action["source_owner"]),
                action_text=markdown_table_cell(action["action"]),
                missing_now=markdown_code_list(action["missing_now_env_vars"]),
                missing_evidence=markdown_code_list(action["missing_env_vars"]),
                false_flags=markdown_code_list(action["false_truth_flag_env_vars"]),
                evidence_requirements=markdown_code_list(
                    action["evidence_requirements"],
                ),
                stages=markdown_code_list(action["no_pda_stages"]),
                requires_real_pda=str(action["requires_real_pda"]).lower(),
            ),
        )
    print()
    print("## Commands")
    print()
    print("```bash")
    print("just wave-3-pda-field-owner-gap-actions --json")
    for command in payload["field_execution_summary"]["record_commands"]:
        print(command)
    print("```")

def print_field_handoff_bundle_markdown(payload: dict[str, object]) -> None:
    print("# W6.D PDA Field Handoff Bundle")
    print()
    print("This bundle is read-only. It does not write runtime evidence and cannot close W6.D.")
    print()
    print(f"writes_runtime_evidence={str(payload['writes_runtime_evidence']).lower()}")
    print(f"closes_gate={str(payload['closes_gate']).lower()}")
    print(f"evidence_file={payload['evidence_file']}")
    print(f"include_precheck={str(payload['include_precheck']).lower()}")
    print()
    print("## Bundle Scope")
    print()
    for item in payload["bundle_scope"]:
        print(f"- `{item}`")
    print()
    print("## Summary")
    print()
    print(
        "ready_for_record_from_env_vars="
        f"{str(payload['ready_for_record_from_env_vars']).lower()}",
    )
    print(f"gap_action_count={payload['gap_action_count']}")
    print(f"real_pda_missing_env_vars_count={payload['real_pda_missing_env_vars_count']}")
    print(f"false_truth_flag_env_vars_count={payload['false_truth_flag_env_vars_count']}")
    print()
    print("## Current Env Status")
    print()
    current_env_status = payload["field_execution_summary"]["current_env_status"]
    print(
        "missing_now_env_vars="
        f"{markdown_code_list(current_env_status['missing_now_env_vars'])}",
    )
    print()
    print("## Owner Actions")
    print()
    print(
        "| Owner | Missing now | Real evidence vars | False flags | "
        "Evidence requirements | Real PDA? |",
    )
    print(
        "|-------|-------------|--------------------|-------------|"
        "-----------------------|-----------|",
    )
    for action in payload["field_owner_gap_actions"]["field_owner_gap_actions"]:
        print(
            "| {owner} | {missing_now} | {missing_evidence} | {false_flags} | "
            "{evidence_requirements} | {requires_real_pda} |".format(
                owner=markdown_table_cell(action["source_owner"]),
                missing_now=markdown_code_list(action["missing_now_env_vars"]),
                missing_evidence=markdown_code_list(action["missing_env_vars"]),
                false_flags=markdown_code_list(action["false_truth_flag_env_vars"]),
                evidence_requirements=markdown_code_list(
                    action["evidence_requirements"],
                ),
                requires_real_pda=str(action["requires_real_pda"]).lower(),
            ),
        )
    print()
    print("## Package Template")
    print()
    print(
        "section_count="
        f"{len(payload['evidence_package_template']['sections'])}",
    )
    print(
        "owner_action_count="
        f"{len(payload['evidence_package_template']['owner_actions'])}",
    )
    print()
    print("## Intake Template")
    print()
    print(f"intake_mode={payload['intake_template']['mode']}")
    print(f"intake_kind={payload['intake_template']['kind']}")
    print("intake_writes_runtime_evidence=false")
    print("intake_closes_gate=false")
    print()
    print("## Precheck")
    print()
    if payload["field_precheck_summary"] is None:
        print("- not run; use `--from-env` to include service and trace-code OpenAPI prechecks")
    else:
        precheck = payload["field_precheck_summary"]
        print(f"- ok={str(precheck['ok']).lower()}")
        print(f"- issues_count={len(precheck['issues'])}")
    print()
    print("## Must Not Do")
    print()
    for item in payload["must_not_do"]:
        print(f"- {item}")
    print()
    print("## Commands")
    print()
    print("```bash")
    print("just wave-3-pda-field-handoff-bundle --json")
    print("just wave-3-pda-field-handoff-bundle --from-env --json")
    print("just wave-3-pda-intake-template --json")
    print("just wave-3-pda-intake-check --json")
    print("just wave-3-pda-intake-record --json")
    for command in payload["field_execution_summary"]["record_commands"]:
        print(command)
    print("```")

def print_preaudit_kit_markdown(payload: dict[str, object]) -> None:
    print("# W6.D PDA Pre-Audit Kit")
    print()
    print("这不是 runtime evidence JSON，不能关闭 W6.D gate。")
    print("用途是在真 PDA 实测前，把可推进事项、阻塞字段和禁止事项一次性交给现场负责人。")
    print()
    print(f"writes_runtime_evidence={str(payload['writes_runtime_evidence']).lower()}")
    print(f"closes_gate={str(payload['closes_gate']).lower()}")
    print()
    print("## 适用负责人")
    print()
    for audience in payload["audiences"]:
        print(f"- {audience}")
    print()
    print("## 现在就能推进")
    print()
    current_env_status = payload["current_env_status"]
    missing_now_env_vars = current_env_status["missing_now_env_vars"]
    if missing_now_env_vars:
        print("当前缺少前置变量：")
        for owner in current_env_status["missing_now_env_var_owners"]:
            print(f"- {owner['env_var']}：{owner['source_owner']}")
        print()
    else:
        print("当前前置变量已设置：WAVE_3_PDA_ENVIRONMENT、WAVE_3_PDA_SERVICE_URL。")
        print()
    print("| 负责人 | 动作 | 可交付证明 |")
    print("|--------|------|------------|")
    for action in payload["now_actions"]:
        print("| {owner} | {action} | {proof} |".format(**action))
    print()
    print("## 必须等真 PDA 实扫后才能填写")
    print()
    print("| 变量 | 负责人 | 证据要求 |")
    print("|------|--------|----------|")
    for field in payload["blocked_until_real_pda"]:
        print(
            "| {name} | {source_owner} | {evidence_requirement} |".format(
                **field,
            )
        )
    print()
    print("## 禁止事项")
    print()
    for item in payload["must_not_do"]:
        print(f"- {item}")
    print()
    print("## 下一步命令")
    print()
    print("```bash")
    for command in payload["next_commands"]:
        print(command)
    print("```")

def print_trace_code_openapi_text(
    ok: bool,
    facts: dict[str, object],
    issues: list[str],
) -> None:
    if ok:
        print("✓ Wave 3 PDA trace-code OpenAPI precheck passed")
        print(f"PASS openapi: {facts.get('openapi')}")
        print(f"PASS api_key_header: {facts.get('api_key_header_name')}")
        print("不会写入 runtime evidence；不能关闭 W6.D gate")
        return

    print("✘ Wave 3 PDA trace-code OpenAPI precheck failed", file=sys.stderr)
    print("不会写入 runtime evidence；不能关闭 W6.D gate", file=sys.stderr)
    for issue in issues:
        print(f"FAIL trace-code-openapi: {issue}", file=sys.stderr)
    for tip in trace_code_openapi_troubleshooting(facts):
        print(f"TIP trace-code-openapi: {tip}", file=sys.stderr)

def print_text(ok: bool, facts: dict[str, object], issues: list[str]) -> None:
    if ok:
        print("✓ Wave 3 PDA runtime readiness passed")
        print("PASS payload_contract: Wave 3 PDA runtime evidence 内容有效")
        print(f"PASS service_health: {facts.get('health_path')} reachable")
        print(f"PASS wave3_route_auth: {facts.get('wave3_route_path')} protected by auth")
        print("不会写入 runtime evidence；不能关闭 W6.D gate")
        return

    print("✘ Wave 3 PDA runtime readiness failed", file=sys.stderr)
    print("不会写入 runtime evidence；不能关闭 W6.D gate", file=sys.stderr)
    for issue in issues:
        print(f"FAIL readiness: {issue}", file=sys.stderr)
