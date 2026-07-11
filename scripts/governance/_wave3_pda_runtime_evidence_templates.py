"""Templates and field mappings for Wave 3 PDA runtime evidence."""
DEFAULT_EVIDENCE_DISPLAY = "docs/retros/wave-3-pda-runtime-evidence.json"

STRING_ARGS = (
    "environment",
    "pda_model",
    "android_version",
    "scan_input_method",
    "pda_stack_candidate",
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
INT_ARGS = (
    "barcode_samples_scanned",
    "m2_operations_exercised",
    "m3_operations_exercised",
    "offline_replays_exercised",
    "idempotency_replays_exercised",
)
ENV_STRING_ARGS = {
    "environment": "WAVE_3_PDA_ENVIRONMENT",
    "pda_model": "WAVE_3_PDA_PDA_MODEL",
    "android_version": "WAVE_3_PDA_ANDROID_VERSION",
    "scan_input_method": "WAVE_3_PDA_SCAN_INPUT_METHOD",
    "pda_stack_candidate": "WAVE_3_PDA_STACK_CANDIDATE",
    "pda_device_ref": "WAVE_3_PDA_PDA_DEVICE_REF",
    "spike005_result_ref": "WAVE_3_PDA_SPIKE_RESULT_REF",
    "m2_scan_log_ref": "WAVE_3_PDA_M2_SCAN_LOG_REF",
    "m3_scan_log_ref": "WAVE_3_PDA_M3_SCAN_LOG_REF",
    "offline_replay_log_ref": "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
    "idempotency_replay_log_ref": "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF",
    "audit_event_query_ref": "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
    "l7_run_ref": "WAVE_3_PDA_L7_RUN_REF",
    "usability_review_ref": "WAVE_3_PDA_USABILITY_REVIEW_REF",
    "native_shell_ref": "WAVE_3_PDA_NATIVE_SHELL_REF",
    "native_scan_plugin_ref": "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
}
ENV_INT_ARGS = {
    "barcode_samples_scanned": "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED",
    "m2_operations_exercised": "WAVE_3_PDA_M2_OPERATIONS_EXERCISED",
    "m3_operations_exercised": "WAVE_3_PDA_M3_OPERATIONS_EXERCISED",
    "offline_replays_exercised": "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED",
    "idempotency_replays_exercised": "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED",
}
ENV_FLAG_ARGS = {
    "real_pda_used": "WAVE_3_PDA_REAL_PDA_USED",
    "physical_scan_key_verified": "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED",
    "dev_or_staging_service_verified": "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
    "audit_event_verified": "WAVE_3_PDA_AUDIT_EVENT_VERIFIED",
    "l7_review_completed": "WAVE_3_PDA_L7_REVIEW_COMPLETED",
    "usability_review_completed": "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED",
}
FIELD_TO_CLI_ARG = {
    **{
        field: f"--{field.replace('_', '-')}"
        for field in (
            *STRING_ARGS,
            *INT_ARGS,
            "native_shell_ref",
            "native_scan_plugin_ref",
        )
    },
}
CLI_ARG_TO_ENV = {
    **{
        FIELD_TO_CLI_ARG[field]: env_name
        for field, env_name in {
            **ENV_STRING_ARGS,
            **ENV_INT_ARGS,
            **ENV_FLAG_ARGS,
        }.items()
        if field in FIELD_TO_CLI_ARG
    },
}
TRUE_ENV_VALUES = {"true", "1", "yes", "on"}
FALSE_ENV_VALUES = {"false", "0", "no", "off", ""}
INTAKE_KIND = "wave3-pda-runtime-evidence-intake"
INTAKE_STRING_FIELDS = frozenset((*STRING_ARGS, "native_shell_ref", "native_scan_plugin_ref"))
INTAKE_INT_FIELDS = frozenset(INT_ARGS)
INTAKE_BOOL_FIELDS = frozenset(ENV_FLAG_ARGS)
INTAKE_EVIDENCE_FIELDS = INTAKE_STRING_FIELDS | INTAKE_INT_FIELDS | INTAKE_BOOL_FIELDS
EXPORT_TEMPLATE = """# Wave 3 PDA runtime evidence materials
# Fill with real dev/staging PDA evidence refs and flags. Do not use local/prod/production/mock/fake/stub/example refs.
# This template does not write runtime evidence and cannot close W6.D.
# Evidence refs must include environment, PDA asset, executed_at, test account or tenant, scenario, business IDs, and result summary.
# Save readiness --json output as a field precheck attachment; it cannot close W6.D.
# Do not invent local L7 thresholds; record measured facts only.
# Operator usability checklist belongs in WAVE_3_PDA_USABILITY_REVIEW_REF.
# The checklist must cover device grip, scan key reachability, scan feedback, offline prompts, error messages, reconnect confirmation, and conclusion.
# Use docs/runbooks/wave-3-pda-readiness.md W6.D L7 and usability templates.
# Set WAVE_3_PDA_* boolean variables to true only after the corresponding real PDA evidence is collected.
# Normal closeout must not use --force.
# Only use --force after backing up or confirming replacement of an existing evidence JSON.
export WAVE_3_PDA_ENVIRONMENT='staging'
export WAVE_3_PDA_SERVICE_URL=''
export WAVE_3_PDA_TRACE_CODE_OPENAPI_URL=''
export WAVE_3_PDA_TRACE_CODE_API_KEY=''
export WAVE_3_PDA_PDA_MODEL=''
export WAVE_3_PDA_ANDROID_VERSION=''
export WAVE_3_PDA_SCAN_INPUT_METHOD=''
export WAVE_3_PDA_STACK_CANDIDATE='react-native'
export WAVE_3_PDA_PDA_DEVICE_REF=''
export WAVE_3_PDA_SPIKE_RESULT_REF=''
export WAVE_3_PDA_M2_SCAN_LOG_REF=''
export WAVE_3_PDA_M3_SCAN_LOG_REF=''
export WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF=''
export WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF=''
export WAVE_3_PDA_AUDIT_EVENT_QUERY_REF=''
export WAVE_3_PDA_L7_RUN_REF=''
export WAVE_3_PDA_USABILITY_REVIEW_REF=''
export WAVE_3_PDA_BARCODE_SAMPLES_SCANNED='50'
export WAVE_3_PDA_M2_OPERATIONS_EXERCISED='1'
export WAVE_3_PDA_M3_OPERATIONS_EXERCISED='1'
export WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED='50'
export WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED='50'
export WAVE_3_PDA_REAL_PDA_USED='false'
export WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED='false'
export WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED='false'
export WAVE_3_PDA_AUDIT_EVENT_VERIFIED='false'
export WAVE_3_PDA_L7_REVIEW_COMPLETED='false'
export WAVE_3_PDA_USABILITY_REVIEW_COMPLETED='false'
export WAVE_3_PDA_NATIVE_SHELL_REF=''
export WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF=''

just wave-3-pda-preaudit-kit --json

just wave-3-pda-materials-checklist --json

just wave-3-pda-field-work-request

just wave-3-pda-field-execution-summary --json

just wave-3-pda-field-precheck-summary --from-env

just wave-3-pda-field-precheck-summary --from-env --json

just wave-3-pda-field-owner-gap-actions

just wave-3-pda-field-owner-gap-actions --json

just wave-3-pda-field-handoff-bundle --json

just wave-3-pda-evidence-package-template

just wave-3-pda-intake-template --json

just wave-3-pda-intake-check --json

just wave-3-pda-service-precheck --from-env --json

just wave-3-pda-trace-code-openapi-precheck --from-env --json

just wave-3-pda-runtime-readiness --from-env --json

just wave-3-pda-runtime-evidence-record --from-env --check-only --json

just wave-3-pda-runtime-evidence-record --from-env --json

just wave-3-pda-intake-record --json

just wave-3-pda-runtime-evidence-validate

# If pda-stack-candidate=webview-capacitor, readiness and record/check-only read native refs through --from-env.
#   --native-shell-ref "$WAVE_3_PDA_NATIVE_SHELL_REF" \\
#   --native-scan-plugin-ref "$WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF"
# real PDA flags are controlled by WAVE_3_PDA_* boolean variables.
"""
PACKAGE_TEMPLATE = """# W6.D PDA Runtime Evidence Package

This package template is not runtime evidence JSON and cannot close W6.D.
Fill it with real dev/staging + real PDA execution facts, then store the
final document in the evidence repository and reference it from the matching
WAVE_3_PDA_* variables.

## 1. Execution Metadata

| Field | Value |
|-------|-------|
| Environment |  |
| Executed at |  |
| Test account / tenant |  |
| PDA asset ref | asset:// |
| PDA model |  |
| Android version |  |
| Scan input method |  |
| PDA stack candidate | react-native / webview-capacitor |
| Barcode sample batch |  |

## 2. M2 Scan Evidence

| Field | Value |
|-------|-------|
| Scenario | M2 scan |
| Business document ID |  |
| Barcode sample IDs |  |
| Request trace ID |  |
| API response summary |  |
| audit_event resource ID |  |
| Evidence ref for WAVE_3_PDA_M2_SCAN_LOG_REF |  |

## 3. M3 Scan Evidence

| Field | Value |
|-------|-------|
| Scenario | M3 scan |
| Inventory batch / state change ID |  |
| Barcode sample IDs |  |
| Request trace ID |  |
| API response summary |  |
| audit_event resource ID |  |
| Evidence ref for WAVE_3_PDA_M3_SCAN_LOG_REF |  |

## 4. Offline Replay Evidence

| Field | Value |
|-------|-------|
| Offline started at |  |
| Network restored at |  |
| Offline queue count |  |
| Replay order summary |  |
| Success / failure summary |  |
| Conflict handling summary |  |
| Evidence ref for WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF |  |

## 5. Idempotency-Key Replay Evidence

| Field | Value |
|-------|-------|
| First request ID |  |
| Replay request ID |  |
| Idempotency-Key |  |
| Response consistency summary |  |
| Evidence ref for WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF |  |

## 6. H2 audit_event Query Evidence

| Field | Value |
|-------|-------|
| Query time |  |
| Query filter |  |
| M2 resource IDs found |  |
| M3 resource IDs found |  |
| Replay resource IDs found |  |
| Evidence ref for WAVE_3_PDA_AUDIT_EVENT_QUERY_REF |  |

## 7. L7 Run Record

Record measured facts only. Do not invent local thresholds.

| Field | Value |
|-------|-------|
| Device model |  |
| Network condition |  |
| M2 operation count |  |
| M3 operation count |  |
| Barcode samples scanned |  |
| Offline replays exercised |  |
| Idempotency replays exercised |  |
| Result summary |  |
| Evidence ref for WAVE_3_PDA_L7_RUN_REF |  |

## 8. Operator Usability Review

| Field | Value |
|-------|-------|
| Operator role |  |
| Device grip |  |
| Scan key reachability |  |
| Scan feedback |  |
| Offline prompt |  |
| Error prompt |  |
| Reconnect confirmation path |  |
| Review conclusion |  |
| Evidence ref for WAVE_3_PDA_USABILITY_REVIEW_REF |  |

## 9. Trace-code OpenAPI Precheck Attachment

This section is a preparation attachment only. It does not close W6.D and does
not replace real PDA scan, replay, audit, L7, or usability evidence.

| Field | Value |
|-------|-------|
| OpenAPI URL variable | WAVE_3_PDA_TRACE_CODE_OPENAPI_URL |
| Precheck command | just wave-3-pda-trace-code-openapi-precheck --from-env --json |
| Required operations summary |  |
| ApiKeyAuth header summary | X-API-Key |
| Precheck output attachment ref |  |

Do not paste WAVE_3_PDA_TRACE_CODE_API_KEY into this package, screenshots, or
runtime evidence JSON.

## 10. Evidence JSON Mapping

| Evidence JSON field / variable | Evidence ref |
|--------------------------------|--------------|
| WAVE_3_PDA_PDA_DEVICE_REF |  |
| WAVE_3_PDA_SPIKE_RESULT_REF |  |
| WAVE_3_PDA_M2_SCAN_LOG_REF |  |
| WAVE_3_PDA_M3_SCAN_LOG_REF |  |
| WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF |  |
| WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF |  |
| WAVE_3_PDA_AUDIT_EVENT_QUERY_REF |  |
| WAVE_3_PDA_L7_RUN_REF |  |
| WAVE_3_PDA_USABILITY_REVIEW_REF |  |
| WAVE_3_PDA_NATIVE_SHELL_REF | WebView/Capacitor only |
| WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF | WebView/Capacitor only |

Before all refs above are present and verified, do not set real_pda_used=true,
physical_scan_key_verified=true, audit_event_verified=true,
l7_review_completed=true, or usability_review_completed=true.

## 11. Owner Actions

| Owner | Action | Required env vars | Acceptance | Runtime write? |
|-------|--------|-------------------|------------|----------------|
| 业务方 / 资产负责人 / 设备方 | 提供真 PDA 设备资产信息 | WAVE_3_PDA_PDA_MODEL, WAVE_3_PDA_ANDROID_VERSION, WAVE_3_PDA_PDA_DEVICE_REF | PDA 资产引用必须是 asset://.../pda/...，并记录 Android 版本 | can_write_runtime_evidence=false |
| PDA 技术验证负责人 | 确认实体扫码键或厂商扫码通道 | WAVE_3_PDA_SCAN_INPUT_METHOD, WAVE_3_PDA_SPIKE_RESULT_REF | 扫码输入方式必须包含 scan-key / KeyEvent / Intent / DataWedge 之一 | can_write_runtime_evidence=false |
| 测试执行人 | 用真 PDA 采集 M2/M3 scan 与 offline replay 日志 | WAVE_3_PDA_M2_SCAN_LOG_REF, WAVE_3_PDA_M3_SCAN_LOG_REF, WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF | 日志引用必须包含 staging 或 dev、wave3-pda 场景名和 run ID | can_write_runtime_evidence=false |
| 测试执行人 / 后端负责人 | 归档 Idempotency-Key replay 日志 | WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF, WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED | 记录首次请求、重放请求、相同 Idempotency-Key 和响应一致性摘要 | can_write_runtime_evidence=false |
| 后端 / 数据库操作人 | 归档 H2 audit_event 查询证据 | WAVE_3_PDA_AUDIT_EVENT_QUERY_REF, WAVE_3_PDA_AUDIT_EVENT_VERIFIED | 查询引用必须能定位 M2/M3 scan、offline replay、idempotency replay 审计事件 | can_write_runtime_evidence=false |
| 测试负责人 | 归档 L7 实测事实记录 | WAVE_3_PDA_BARCODE_SAMPLES_SCANNED, WAVE_3_PDA_M2_OPERATIONS_EXERCISED, WAVE_3_PDA_M3_OPERATIONS_EXERCISED, WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED, WAVE_3_PDA_L7_RUN_REF, WAVE_3_PDA_L7_REVIEW_COMPLETED | 记录 50 个条码样本、M2/M3 操作、50 次 offline replay 和结果摘要 | can_write_runtime_evidence=false |
| 测试负责人 / 业务走查人 | 归档操作员易用性走查 | WAVE_3_PDA_USABILITY_REVIEW_REF, WAVE_3_PDA_USABILITY_REVIEW_COMPLETED | 覆盖握持、扫码键触达、扫码反馈、离线提示、错误提示和恢复网络确认 | can_write_runtime_evidence=false |
"""
PACKAGE_TEMPLATE_SECTIONS: tuple[dict[str, object], ...] = (
    {
        "id": "execution_metadata",
        "title": "Execution Metadata",
        "fields": [
            "Environment",
            "Executed at",
            "Test account / tenant",
            "PDA asset ref",
            "PDA model",
            "Android version",
            "Scan input method",
            "PDA stack candidate",
            "Barcode sample batch",
        ],
    },
    {
        "id": "m2_scan_evidence",
        "title": "M2 Scan Evidence",
        "fields": [
            "Scenario",
            "Business document ID",
            "Barcode sample IDs",
            "Request trace ID",
            "API response summary",
            "audit_event resource ID",
            "Evidence ref for WAVE_3_PDA_M2_SCAN_LOG_REF",
        ],
    },
    {
        "id": "m3_scan_evidence",
        "title": "M3 Scan Evidence",
        "fields": [
            "Scenario",
            "Inventory batch / state change ID",
            "Barcode sample IDs",
            "Request trace ID",
            "API response summary",
            "audit_event resource ID",
            "Evidence ref for WAVE_3_PDA_M3_SCAN_LOG_REF",
        ],
    },
    {
        "id": "offline_replay_evidence",
        "title": "Offline Replay Evidence",
        "fields": [
            "Offline started at",
            "Network restored at",
            "Offline queue count",
            "Replay order summary",
            "Success / failure summary",
            "Conflict handling summary",
            "Evidence ref for WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
        ],
    },
    {
        "id": "idempotency_key_replay_evidence",
        "title": "Idempotency-Key Replay Evidence",
        "fields": [
            "First request ID",
            "Replay request ID",
            "Idempotency-Key",
            "Response consistency summary",
            "Evidence ref for WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF",
        ],
    },
    {
        "id": "audit_event_query_evidence",
        "title": "H2 audit_event Query Evidence",
        "fields": [
            "Query time",
            "Query filter",
            "M2 resource IDs found",
            "M3 resource IDs found",
            "Replay resource IDs found",
            "Evidence ref for WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
        ],
    },
    {
        "id": "l7_run_record",
        "title": "L7 Run Record",
        "fields": [
            "Device model",
            "Network condition",
            "M2 operation count",
            "M3 operation count",
            "Barcode samples scanned",
            "Offline replays exercised",
            "Idempotency replays exercised",
            "Result summary",
            "Evidence ref for WAVE_3_PDA_L7_RUN_REF",
        ],
    },
    {
        "id": "operator_usability_review",
        "title": "Operator Usability Review",
        "fields": [
            "Operator role",
            "Device grip",
            "Scan key reachability",
            "Scan feedback",
            "Offline prompt",
            "Error prompt",
            "Reconnect confirmation path",
            "Review conclusion",
            "Evidence ref for WAVE_3_PDA_USABILITY_REVIEW_REF",
        ],
    },
    {
        "id": "trace_code_openapi_precheck_attachment",
        "title": "Trace-code OpenAPI Precheck Attachment",
        "fields": [
            "OpenAPI URL variable",
            "Precheck command",
            "Required operations summary",
            "ApiKeyAuth header summary",
            "Precheck output attachment ref",
        ],
    },
    {
        "id": "evidence_json_mapping",
        "title": "Evidence JSON Mapping",
        "fields": [
            "WAVE_3_PDA_PDA_DEVICE_REF",
            "WAVE_3_PDA_SPIKE_RESULT_REF",
            "WAVE_3_PDA_M2_SCAN_LOG_REF",
            "WAVE_3_PDA_M3_SCAN_LOG_REF",
            "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
            "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF",
            "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
            "WAVE_3_PDA_L7_RUN_REF",
            "WAVE_3_PDA_USABILITY_REVIEW_REF",
            "WAVE_3_PDA_NATIVE_SHELL_REF",
            "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
        ],
    },
    {
        "id": "owner_actions",
        "title": "Owner Actions",
        "fields": [
            "Owner",
            "Action",
            "Required env vars",
            "Acceptance",
            "Runtime write?",
        ],
    },
)
PACKAGE_OWNER_ACTIONS: tuple[dict[str, object], ...] = (
    {
        "owner": "业务方 / 资产负责人 / 设备方",
        "action": "提供真 PDA 设备资产信息",
        "required_env_vars": [
            "WAVE_3_PDA_PDA_MODEL",
            "WAVE_3_PDA_ANDROID_VERSION",
            "WAVE_3_PDA_PDA_DEVICE_REF",
        ],
        "acceptance": "PDA 资产引用必须是 asset://.../pda/...，并记录 Android 版本",
        "can_write_runtime_evidence": False,
    },
    {
        "owner": "PDA 技术验证负责人",
        "action": "确认实体扫码键或厂商扫码通道",
        "required_env_vars": [
            "WAVE_3_PDA_SCAN_INPUT_METHOD",
            "WAVE_3_PDA_SPIKE_RESULT_REF",
        ],
        "acceptance": "扫码输入方式必须包含 scan-key / KeyEvent / Intent / DataWedge 之一",
        "can_write_runtime_evidence": False,
    },
    {
        "owner": "测试执行人",
        "action": "用真 PDA 采集 M2/M3 scan 与 offline replay 日志",
        "required_env_vars": [
            "WAVE_3_PDA_M2_SCAN_LOG_REF",
            "WAVE_3_PDA_M3_SCAN_LOG_REF",
            "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
        ],
        "acceptance": "日志引用必须包含 staging 或 dev、wave3-pda 场景名和 run ID",
        "can_write_runtime_evidence": False,
    },
    {
        "owner": "测试执行人 / 后端负责人",
        "action": "归档 Idempotency-Key replay 日志",
        "required_env_vars": [
            "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF",
            "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED",
        ],
        "acceptance": "记录首次请求、重放请求、相同 Idempotency-Key 和响应一致性摘要",
        "can_write_runtime_evidence": False,
    },
    {
        "owner": "后端 / 数据库操作人",
        "action": "归档 H2 audit_event 查询证据",
        "required_env_vars": [
            "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
            "WAVE_3_PDA_AUDIT_EVENT_VERIFIED",
        ],
        "acceptance": "查询引用必须能定位 M2/M3 scan、offline replay、idempotency replay 审计事件",
        "can_write_runtime_evidence": False,
    },
    {
        "owner": "测试负责人",
        "action": "归档 L7 实测事实记录",
        "required_env_vars": [
            "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED",
            "WAVE_3_PDA_M2_OPERATIONS_EXERCISED",
            "WAVE_3_PDA_M3_OPERATIONS_EXERCISED",
            "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED",
            "WAVE_3_PDA_L7_RUN_REF",
            "WAVE_3_PDA_L7_REVIEW_COMPLETED",
        ],
        "acceptance": "记录 50 个条码样本、M2/M3 操作、50 次 offline replay 和结果摘要",
        "can_write_runtime_evidence": False,
    },
    {
        "owner": "测试负责人 / 业务走查人",
        "action": "归档操作员易用性走查",
        "required_env_vars": [
            "WAVE_3_PDA_USABILITY_REVIEW_REF",
            "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED",
        ],
        "acceptance": "覆盖握持、扫码键触达、扫码反馈、离线提示、错误提示和恢复网络确认",
        "can_write_runtime_evidence": False,
    },
)
PACKAGE_MAPPING_VARIABLES = (
    "WAVE_3_PDA_PDA_DEVICE_REF",
    "WAVE_3_PDA_SPIKE_RESULT_REF",
    "WAVE_3_PDA_M2_SCAN_LOG_REF",
    "WAVE_3_PDA_M3_SCAN_LOG_REF",
    "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
    "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF",
    "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
    "WAVE_3_PDA_L7_RUN_REF",
    "WAVE_3_PDA_USABILITY_REVIEW_REF",
    "WAVE_3_PDA_NATIVE_SHELL_REF",
    "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
)
PACKAGE_BLOCKED_FLAGS = (
    "real_pda_used",
    "physical_scan_key_verified",
    "audit_event_verified",
    "l7_review_completed",
    "usability_review_completed",
)
PACKAGE_RECORD_GATE_AFTER_OWNER_ACTIONS = (
    "just wave-3-pda-runtime-readiness --from-env --json",
    "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
    "just wave-3-pda-runtime-evidence-record --from-env --json",
    "just wave-3-pda-intake-check --json",
    "just wave-3-pda-intake-record --json",
    "just wave-3-pda-runtime-evidence-validate",
)
INTAKE_RECORD_GATE_AFTER_INTAKE = (
    "just wave-3-pda-intake-check --json",
    "just wave-3-pda-intake-record --json",
    "just wave-3-pda-runtime-evidence-validate",
)
PACKAGE_TEMPLATE_WARNINGS = (
    "This package template is not runtime evidence JSON and cannot close W6.D.",
    "readiness --json output is only a field precheck attachment",
    "trace-code OpenAPI precheck output is only a preparation attachment",
    "Do not set truth flags before the matching real PDA evidence refs are present.",
    "Do not invent local L7 thresholds; record measured facts only.",
    "Do not paste WAVE_3_PDA_TRACE_CODE_API_KEY into evidence packages.",
)
