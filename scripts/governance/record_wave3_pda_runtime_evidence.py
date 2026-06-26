#!/usr/bin/env python3
"""Record Wave 3 real PDA and L7 runtime evidence."""
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

from _wave_evidence_recorder import check_only_result
from check_wave3_pda_runtime_readiness import missing_env_var_owner_details
from validate_wave3_pda_runtime_evidence import (
    DEFAULT_EVIDENCE,
    validate_wave3_pda_runtime_payload,
)

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


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    payload = {
        "environment": args.environment,
        "pda_model": args.pda_model,
        "android_version": args.android_version,
        "scan_input_method": args.scan_input_method,
        "pda_stack_candidate": args.pda_stack_candidate,
        "pda_device_ref": args.pda_device_ref,
        "spike005_result_ref": args.spike005_result_ref,
        "m2_scan_log_ref": args.m2_scan_log_ref,
        "m3_scan_log_ref": args.m3_scan_log_ref,
        "offline_replay_log_ref": args.offline_replay_log_ref,
        "idempotency_replay_log_ref": args.idempotency_replay_log_ref,
        "audit_event_query_ref": args.audit_event_query_ref,
        "l7_run_ref": args.l7_run_ref,
        "usability_review_ref": args.usability_review_ref,
        "barcode_samples_scanned": args.barcode_samples_scanned,
        "m2_operations_exercised": args.m2_operations_exercised,
        "m3_operations_exercised": args.m3_operations_exercised,
        "offline_replays_exercised": args.offline_replays_exercised,
        "idempotency_replays_exercised": args.idempotency_replays_exercised,
        "real_pda_used": args.real_pda_used,
        "physical_scan_key_verified": args.physical_scan_key_verified,
        "dev_or_staging_service_verified": args.dev_or_staging_service_verified,
        "audit_event_verified": args.audit_event_verified,
        "l7_review_completed": args.l7_review_completed,
        "usability_review_completed": args.usability_review_completed,
    }
    if args.native_shell_ref:
        payload["native_shell_ref"] = args.native_shell_ref
    if args.native_scan_plugin_ref:
        payload["native_scan_plugin_ref"] = args.native_scan_plugin_ref
    return payload


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    ok, message = validate_wave3_pda_runtime_payload(payload)
    if not ok:
        return False, message

    if path.exists() and not force:
        return False, f"{path} already exists; pass --force to overwrite"

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return True, f"wrote {path}"


def check_payload(payload: dict[str, Any]) -> tuple[bool, str]:
    ok, message = validate_wave3_pda_runtime_payload(payload)
    if ok:
        return True, f"check-only passed: {message}"
    return False, message


def missing_required_args(args: argparse.Namespace) -> list[str]:
    missing: list[str] = []
    for field in STRING_ARGS:
        value = str(getattr(args, field, "") or "").strip()
        if not value:
            missing.append(f"--{field.replace('_', '-')}")
    for field in INT_ARGS:
        if getattr(args, field, None) is None:
            missing.append(f"--{field.replace('_', '-')}")
    if args.pda_stack_candidate == "webview-capacitor":
        if not (args.native_shell_ref or "").strip():
            missing.append("--native-shell-ref")
        if not (args.native_scan_plugin_ref or "").strip():
            missing.append("--native-scan-plugin-ref")
    return missing


def false_flag_env_vars(args: argparse.Namespace) -> list[str]:
    return [
        env_name
        for field, env_name in ENV_FLAG_ARGS.items()
        if getattr(args, field) is not True
    ]


def apply_env_args(args: argparse.Namespace) -> list[str]:
    issues: list[str] = []
    for field, env_name in ENV_STRING_ARGS.items():
        value = os.environ.get(env_name)
        if value is not None:
            setattr(args, field, value.strip())

    for field, env_name in ENV_INT_ARGS.items():
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

    for field, env_name in ENV_FLAG_ARGS.items():
        raw_value = os.environ.get(env_name, "")
        value = raw_value.strip().lower()
        if value in TRUE_ENV_VALUES:
            setattr(args, field, True)
        elif value in FALSE_ENV_VALUES:
            setattr(args, field, False)
        else:
            issues.append(f"{env_name} must be true or false")
    return issues


def print_export_template() -> None:
    print(EXPORT_TEMPLATE.rstrip())


def print_package_template() -> None:
    print(PACKAGE_TEMPLATE.rstrip())


def display_evidence_file(path: Path) -> str:
    if path.resolve() == DEFAULT_EVIDENCE.resolve():
        return DEFAULT_EVIDENCE_DISPLAY
    return str(path)


def package_template_payload(evidence_file: Path) -> dict[str, object]:
    return {
        "ok": True,
        "mode": "wave3-pda-evidence-package-template",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": display_evidence_file(evidence_file),
        "sections": [
            {
                "id": section["id"],
                "title": section["title"],
                "fields": list(section["fields"]),
            }
            for section in PACKAGE_TEMPLATE_SECTIONS
        ],
        "mapping_variables": list(PACKAGE_MAPPING_VARIABLES),
        "blocked_flags_until_refs_present": list(PACKAGE_BLOCKED_FLAGS),
        "owner_actions": [
            {
                "owner": str(action["owner"]),
                "action": str(action["action"]),
                "required_env_vars": list(action["required_env_vars"]),
                "acceptance": str(action["acceptance"]),
                "can_write_runtime_evidence": bool(action["can_write_runtime_evidence"]),
            }
            for action in PACKAGE_OWNER_ACTIONS
        ],
        "record_gate_after_owner_actions": list(PACKAGE_RECORD_GATE_AFTER_OWNER_ACTIONS),
        "warnings": list(PACKAGE_TEMPLATE_WARNINGS),
    }


def intake_template_evidence() -> dict[str, object]:
    payload: dict[str, object] = {
        "environment": "staging",
        "pda_model": "",
        "android_version": "",
        "scan_input_method": "",
        "pda_stack_candidate": "react-native",
        "pda_device_ref": "",
        "spike005_result_ref": "",
        "m2_scan_log_ref": "",
        "m3_scan_log_ref": "",
        "offline_replay_log_ref": "",
        "idempotency_replay_log_ref": "",
        "audit_event_query_ref": "",
        "l7_run_ref": "",
        "usability_review_ref": "",
        "barcode_samples_scanned": 50,
        "m2_operations_exercised": 1,
        "m3_operations_exercised": 1,
        "offline_replays_exercised": 50,
        "idempotency_replays_exercised": 50,
        "real_pda_used": False,
        "physical_scan_key_verified": False,
        "dev_or_staging_service_verified": False,
        "audit_event_verified": False,
        "l7_review_completed": False,
        "usability_review_completed": False,
        "native_shell_ref": "",
        "native_scan_plugin_ref": "",
    }
    return payload


def intake_template_payload(evidence_file: Path) -> dict[str, object]:
    return {
        "ok": True,
        "mode": "wave3-pda-runtime-evidence-intake-template",
        "schema_version": 1,
        "kind": INTAKE_KIND,
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": display_evidence_file(evidence_file),
        "instructions": [
            "Fill evidence with real dev/staging + real PDA refs.",
            "Run with --from-intake-file <path> --check-only --json before record.",
            "Do not paste trace-code API keys into this intake file.",
            "Set truth flags to true only after matching real evidence refs are present.",
            (
                "Empty string values and false truth flags mean field evidence is still "
                "missing and must be filled by the assigned owner."
            ),
        ],
        "required_evidence_fields": list(STRING_ARGS) + list(INT_ARGS) + list(ENV_FLAG_ARGS),
        "webview_capacitor_evidence_fields": [
            "native_shell_ref",
            "native_scan_plugin_ref",
        ],
        "evidence": intake_template_evidence(),
        "record_gate_after_intake": list(INTAKE_RECORD_GATE_AFTER_INTAKE),
    }


def write_intake_template(
    path: Path,
    payload: dict[str, object],
    *,
    force: bool = False,
) -> tuple[bool, str]:
    if path.exists() and not force:
        return False, f"{path} already exists; pass --intake-template-force to overwrite"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return True, f"wrote {path}"


def load_intake_evidence(path: Path) -> dict[str, object]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ValueError(f"failed to read intake file {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"intake file must be valid JSON: {error}") from error

    if not isinstance(raw, dict):
        raise ValueError("intake file must contain a JSON object")
    if type(raw.get("schema_version")) is not int or raw["schema_version"] != 1:
        raise ValueError("intake schema_version is required and must be 1")
    if raw.get("kind") != INTAKE_KIND:
        raise ValueError(f"intake kind is required and must be {INTAKE_KIND}")
    if raw.get("writes_runtime_evidence") is not False:
        raise ValueError("intake writes_runtime_evidence is required and must be false")
    if raw.get("closes_gate") is not False:
        raise ValueError("intake closes_gate is required and must be false")
    if "evidence" not in raw:
        raise ValueError("intake evidence is required")
    evidence = raw["evidence"]
    if not isinstance(evidence, dict):
        raise ValueError("intake evidence must be a JSON object")
    unknown_fields = sorted(set(evidence) - INTAKE_EVIDENCE_FIELDS)
    if unknown_fields:
        raise ValueError(
            "intake evidence contains unknown fields: "
            f"{', '.join(unknown_fields)}",
        )
    non_string_fields = [
        field
        for field in sorted(INTAKE_STRING_FIELDS & set(evidence))
        if not isinstance(evidence[field], str)
    ]
    if non_string_fields:
        raise ValueError(
            "intake evidence string fields must be JSON strings: "
            + "; ".join(f"{field} must be a JSON string" for field in non_string_fields),
        )
    non_int_fields = [
        field
        for field in sorted(INTAKE_INT_FIELDS & set(evidence))
        if type(evidence[field]) is not int
    ]
    if non_int_fields:
        raise ValueError(
            "intake evidence integer fields must be JSON integers: "
            + "; ".join(f"{field} must be a JSON integer" for field in non_int_fields),
        )
    non_bool_fields = [
        field
        for field in sorted(INTAKE_BOOL_FIELDS & set(evidence))
        if not isinstance(evidence[field], bool)
    ]
    if non_bool_fields:
        raise ValueError(
            "intake evidence boolean fields must be JSON booleans: "
            + "; ".join(f"{field} must be a JSON boolean" for field in non_bool_fields),
        )
    return evidence


def apply_intake_args(args: argparse.Namespace) -> list[str]:
    issues: list[str] = []
    evidence = load_intake_evidence(args.from_intake_file)

    for field in (*STRING_ARGS, "native_shell_ref", "native_scan_plugin_ref"):
        if field not in evidence:
            continue
        value = evidence[field]
        setattr(args, field, value.strip())

    for field in INT_ARGS:
        if field not in evidence:
            continue
        parsed = evidence[field]
        if parsed <= 0:
            issues.append(f"{field} must be > 0")
            continue
        setattr(args, field, parsed)

    for field in ENV_FLAG_ARGS:
        if field not in evidence:
            continue
        setattr(args, field, evidence[field])

    return issues


def report_input_error(
    parser: argparse.ArgumentParser,
    args: argparse.Namespace,
    message: str,
    *,
    missing_args: list[str] | None = None,
) -> int:
    if args.json:
        payload = check_only_result(False, message, args.output)
        payload["check_only"] = bool(args.check_only)
        if missing_args:
            payload["missing_args"] = missing_args
            missing_env_vars = [
                CLI_ARG_TO_ENV[arg]
                for arg in missing_args
                if arg in CLI_ARG_TO_ENV
            ]
            payload["missing_env_vars"] = missing_env_vars
            payload["missing_env_var_owners"] = missing_env_var_owner_details(
                missing_env_vars,
            )
            false_flags = false_flag_env_vars(args)
            if false_flags:
                payload["false_flag_env_vars"] = false_flags
                payload["false_flag_env_var_owners"] = missing_env_var_owner_details(
                    false_flags,
                )
        print(
            json.dumps(
                payload,
                ensure_ascii=False,
                indent=2,
            ),
        )
        return 2
    parser.error(message)
    return 2


def record_result(ok: bool, message: str, evidence_file: Path) -> dict[str, object]:
    return {
        "ok": ok,
        "check_only": False,
        "writes_runtime_evidence": ok,
        "closes_gate": False,
        "evidence_file": str(evidence_file),
        "message": message,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--force", action="store_true")
    parser.add_argument(
        "--export-template",
        action="store_true",
        help="Print a shell template for collecting real Wave 3 PDA evidence refs.",
    )
    parser.add_argument(
        "--export-package-template",
        action="store_true",
        help="Print a Markdown evidence package template without writing evidence.",
    )
    parser.add_argument(
        "--export-intake-template",
        action="store_true",
        help="Print a JSON intake template for field evidence without writing evidence.",
    )
    parser.add_argument(
        "--intake-template-output",
        type=Path,
        help=(
            "Write the JSON intake template to this path. Only valid with "
            "--export-intake-template; does not write runtime evidence."
        ),
    )
    parser.add_argument(
        "--intake-template-force",
        action="store_true",
        help=(
            "Overwrite an existing --intake-template-output file. Only valid with "
            "--export-intake-template; does not write runtime evidence."
        ),
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Validate fields, refs, and boundaries without writing evidence.",
    )
    parser.add_argument(
        "--from-env",
        action="store_true",
        help="Read WAVE_3_PDA_* variables from the exported evidence template.",
    )
    parser.add_argument(
        "--from-intake-file",
        type=Path,
        help="Read Wave 3 PDA evidence fields from a JSON field intake file.",
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--environment", choices=["dev", "staging"])
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
    parser.add_argument("--barcode-samples-scanned", type=int)
    parser.add_argument("--m2-operations-exercised", type=int)
    parser.add_argument("--m3-operations-exercised", type=int)
    parser.add_argument("--offline-replays-exercised", type=int)
    parser.add_argument("--idempotency-replays-exercised", type=int)
    parser.add_argument("--real-pda-used", action="store_true")
    parser.add_argument("--physical-scan-key-verified", action="store_true")
    parser.add_argument("--dev-or-staging-service-verified", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    parser.add_argument("--l7-review-completed", action="store_true")
    parser.add_argument("--usability-review-completed", action="store_true")
    args = parser.parse_args(argv)

    if args.intake_template_output and not args.export_intake_template:
        return report_input_error(
            parser,
            args,
            "--intake-template-output requires --export-intake-template",
        )
    if args.intake_template_force and not args.intake_template_output:
        return report_input_error(
            parser,
            args,
            "--intake-template-force requires --intake-template-output",
        )

    if args.export_template:
        print_export_template()
        return 0
    if args.export_package_template:
        if args.json:
            print(json.dumps(
                package_template_payload(args.output),
                ensure_ascii=False,
                indent=2,
            ))
        else:
            print_package_template()
        return 0
    if args.export_intake_template:
        payload = intake_template_payload(args.output)
        payload["writes_intake_template"] = False
        if args.intake_template_output:
            payload["intake_template_output"] = str(args.intake_template_output)
            file_payload = {
                **payload,
                "writes_intake_template": True,
            }
            ok_to_write, write_message = write_intake_template(
                args.intake_template_output,
                file_payload,
                force=args.intake_template_force,
            )
            payload["writes_intake_template"] = ok_to_write
            payload["message"] = write_message
            if not ok_to_write:
                payload["ok"] = False
            else:
                payload = file_payload | {
                    "message": write_message,
                }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return 0 if payload["ok"] else 1

    if args.from_env and args.from_intake_file:
        return report_input_error(
            parser,
            args,
            "--from-env and --from-intake-file cannot be used together",
        )

    if args.from_intake_file:
        try:
            intake_issues = apply_intake_args(args)
        except ValueError as error:
            return report_input_error(parser, args, str(error))
        if intake_issues:
            return report_input_error(parser, args, "; ".join(intake_issues))

    if args.from_env:
        env_issues = apply_env_args(args)
        if env_issues:
            return report_input_error(parser, args, "; ".join(env_issues))

    missing = missing_required_args(args)
    if missing:
        return report_input_error(
            parser,
            args,
            f"the following arguments are required: {', '.join(missing)}",
            missing_args=missing,
        )

    payload = build_payload(args)
    if args.check_only:
        ok, message = check_payload(payload)
        if ok:
            message = (
                f"{message}; no PDA runtime evidence JSON written; "
                "W6.D gate remains open"
            )
        false_flags = false_flag_env_vars(args)
    else:
        ok, message = write_payload(args.output, payload, force=args.force)
        false_flags = false_flag_env_vars(args)
    if args.json:
        payload = (
            check_only_result(ok, message, args.output)
            if args.check_only
            else record_result(ok, message, args.output)
        )
        if not ok and false_flags:
            payload["false_flag_env_vars"] = false_flags
            payload["false_flag_env_var_owners"] = missing_env_var_owner_details(
                false_flags,
            )
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        mark = "✓" if ok else "✘"
        stream = sys.stdout if ok else sys.stderr
        print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
