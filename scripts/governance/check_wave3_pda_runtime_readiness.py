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

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_HEALTH_PATH = "/healthz"
DEFAULT_WAVE3_ROUTE_PATH = "/api/v1/inventory/batches"
EXPECTED_WAVE3_UNAUTHORIZED_CODE = "AUTH-001"
BLOCKED_SERVICE_URL_TOKENS = (
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
SERVICE_URL_BOUNDARY_MESSAGE = (
    "service_url cannot point to local/prod/production/mock/fake/stub/example"
)
SENSITIVE_URL_QUERY_PARAMS = frozenset({
    "api_key",
    "token",
    "password",
    "secret",
    "signature",
})
W6D_EXTERNAL_PREREQUISITES = (
    "真 PDA",
    "实体扫码键",
    "dev/staging M2/M3 API",
    "离线 replay 条件",
    "幂等 replay 条件",
    "L7 执行环境",
    "人工易用性走查人",
)
W6D_MINIMUM_EVIDENCE_REFS = (
    "PDA 资产引用",
    "扫码日志",
    "离线 replay 日志",
    "idempotency replay 日志",
    "audit_event 查询",
    "L7 执行记录",
    "走查记录",
)
W6D_NEXT_COMMANDS = [
    "just wave-3-pda-preaudit-kit --json",
    "just wave-3-pda-materials-checklist --json",
    "just wave-3-pda-field-work-request",
    "just wave-3-pda-field-execution-summary --json",
    "just wave-3-pda-field-precheck-summary --from-env",
    "just wave-3-pda-field-precheck-summary --from-env --json",
    "just wave-3-pda-field-owner-gap-actions",
    "just wave-3-pda-field-owner-gap-actions --json",
    "just wave-3-pda-field-handoff-bundle --json",
    "just wave-3-pda-evidence-package-template",
    "just wave-3-pda-intake-template --json",
    "just wave-3-pda-intake-check --json",
    "just wave-3-pda-intake-record --json",
    "just wave-3-pda-service-precheck --from-env --json",
    "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
    "just wave-3-pda-runtime-evidence-record --export-template",
    "just wave-3-pda-runtime-readiness --from-env --json",
    "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
    "just wave-3-pda-runtime-evidence-record --from-env --json",
    "just wave-3-pda-runtime-evidence-validate",
]
TRACE_CODE_REQUIRED_PATHS = (
    "/api/codes/{code}",
    "/api/codes/{code}/children",
    "/api/codes/batch",
    "/api/codes/verify",
    "/api/wms-products",
)
TRACE_CODE_REQUIRED_OPERATIONS = (
    ("/api/codes/{code}", "get"),
    ("/api/codes/{code}/children", "get"),
    ("/api/codes/batch", "post"),
    ("/api/codes/verify", "post"),
    ("/api/wms-products", "post"),
)
TRACE_CODE_REQUIRED_OPERATION_LABELS = tuple(
    f"{method.upper()} {path}"
    for path, method in TRACE_CODE_REQUIRED_OPERATIONS
)
TRACE_CODE_ENV_FIELDS = {
    "trace_code_openapi_url": "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
    "trace_code_api_key": "WAVE_3_PDA_TRACE_CODE_API_KEY",
}
FIELD_PRECHECK_ATTACHMENT_KIND = "wave3-pda-field-precheck-attachment"
FIELD_PRECHECK_ATTACHMENT_RUNTIME_EVIDENCE_FILE = (
    "docs/retros/wave-3-pda-runtime-evidence.json"
)
TRACE_CODE_OPENAPI_TROUBLESHOOTING = (
    "If trace-code OpenAPI returns 502 through the current shell, rerun with "
    "NO_PROXY='*' no_proxy='*' or curl --noproxy '*' before treating it as a "
    "contract failure.",
    "Use the 43.128.77.47:9100 OpenAPI endpoint for the current W6.D precheck; "
    "do not switch to 9200 unless the interface owner confirms that port is open.",
    "Keep WAVE_3_PDA_TRACE_CODE_API_KEY in the environment or secret manager only; "
    "never paste the key into logs, screenshots, docs, or evidence JSON.",
)
TRACE_CODE_ENV_VAR_OWNER_DETAILS = {
    env_var: {
        "env_var": env_var,
        "source_owner": "追溯码接口负责人 / 运维",
        "no_pda_stage": "preparable",
        "requires_real_pda": False,
        "evidence_requirement": "追溯码 OpenAPI 合约",
    }
    for env_var in TRACE_CODE_ENV_FIELDS.values()
}
FIELD_WORK_RESOURCES: tuple[dict[str, str], ...] = (
    {
        "resource": "dev/staging service URL",
        "resource_zh": "dev/staging 服务地址",
        "owner": "Ops / deployment owner",
        "owner_zh": "运维 / 部署负责人",
        "deliverable": "Reachable wms-api URL for dev or staging",
        "deliverable_zh": "可访问的 dev 或 staging wms-api 地址",
        "verification": "`WAVE_3_PDA_SERVICE_URL`; just wave-3-pda-service-precheck",
        "verification_zh": (
            "WAVE_3_PDA_SERVICE_URL；运行 just wave-3-pda-service-precheck"
        ),
    },
    {
        "resource": "Trace-code OpenAPI contract",
        "resource_zh": "追溯码 OpenAPI 合约",
        "owner": "Trace-code interface owner / Ops",
        "owner_zh": "追溯码接口负责人 / 运维",
        "deliverable": "Read-only OpenAPI URL and API key stored outside the repo",
        "deliverable_zh": "只读 OpenAPI 地址和存放在仓库外的 API key",
        "verification": (
            "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL, WAVE_3_PDA_TRACE_CODE_API_KEY; "
            "just wave-3-pda-trace-code-openapi-precheck --from-env --json"
        ),
        "verification_zh": (
            "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL、"
            "WAVE_3_PDA_TRACE_CODE_API_KEY；运行 "
            "just wave-3-pda-trace-code-openapi-precheck --from-env --json；"
            "不得把真实 key 写入仓库或 evidence JSON"
        ),
    },
    {
        "resource": "At least one real PDA",
        "resource_zh": "至少一台真 PDA",
        "owner": "Business / asset owner / device vendor",
        "owner_zh": "业务方 / 资产负责人 / 设备方",
        "deliverable": "PDA model, Android version, photo or asset registration",
        "deliverable_zh": "PDA 型号、Android 版本、现场照片或资产登记",
        "verification": (
            "WAVE_3_PDA_PDA_MODEL, WAVE_3_PDA_ANDROID_VERSION, "
            "WAVE_3_PDA_PDA_DEVICE_REF=`asset://.../pda/...`"
        ),
        "verification_zh": (
            "WAVE_3_PDA_PDA_MODEL、WAVE_3_PDA_ANDROID_VERSION、"
            "WAVE_3_PDA_PDA_DEVICE_REF=asset://.../pda/..."
        ),
    },
    {
        "resource": "Physical scan key or vendor scan channel",
        "resource_zh": "实体扫码键 / 厂商扫码通道",
        "owner": "PDA technical verifier",
        "owner_zh": "PDA 技术验证负责人",
        "deliverable": "scan-key, KeyEvent, Intent, or DataWedge input record",
        "deliverable_zh": "scan-key、KeyEvent、Intent 或 DataWedge 输入记录",
        "verification": "WAVE_3_PDA_SCAN_INPUT_METHOD; do not use phone/camera/browser",
        "verification_zh": (
            "WAVE_3_PDA_SCAN_INPUT_METHOD；不能用 phone / camera / browser"
        ),
    },
    {
        "resource": "50 sanitized barcode samples",
        "resource_zh": "50 个脱敏条码样本",
        "owner": "Test owner / business data owner",
        "owner_zh": "测试负责人 / 业务数据负责人",
        "deliverable": "GS1, Code128 batch/carton, and QR task sample list",
        "deliverable_zh": "GS1、Code128 批号 / 箱码、二维码任务号样本清单",
        "verification": "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED=50",
        "verification_zh": "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED=50",
    },
    {
        "resource": "M2/M3 test data",
        "resource_zh": "M2/M3 测试数据",
        "owner": "Backend / test owner",
        "owner_zh": "后端 / 测试负责人",
        "deliverable": "Rebuildable M2 receiving/putaway and M3 query/state-change data",
        "deliverable_zh": "可重建的 M2 收货 / 上架与 M3 查询 / 状态变更数据",
        "verification": (
            "WAVE_3_PDA_M2_OPERATIONS_EXERCISED and "
            "WAVE_3_PDA_M3_OPERATIONS_EXERCISED"
        ),
        "verification_zh": (
            "WAVE_3_PDA_M2_OPERATIONS_EXERCISED 与 "
            "WAVE_3_PDA_M3_OPERATIONS_EXERCISED"
        ),
    },
    {
        "resource": "Offline replay condition",
        "resource_zh": "离线 replay 执行条件",
        "owner": "Test owner",
        "owner_zh": "测试负责人",
        "deliverable": "Disconnect/reconnect execution record and replay summary",
        "deliverable_zh": "断网、恢复网络、离线队列 replay 执行记录和摘要",
        "verification": (
            "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF and "
            "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED"
        ),
        "verification_zh": (
            "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF 与 "
            "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED"
        ),
    },
    {
        "resource": "Idempotency-Key replay condition",
        "resource_zh": "Idempotency-Key replay 条件",
        "owner": "Test owner / backend owner",
        "owner_zh": "测试负责人 / 后端负责人",
        "deliverable": "First request, replay request, same key, and response summary",
        "deliverable_zh": "首次请求、重放请求、相同 key 和响应摘要",
        "verification": (
            "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF and "
            "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED"
        ),
        "verification_zh": (
            "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF 与 "
            "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED"
        ),
    },
    {
        "resource": "H2 audit_event query",
        "resource_zh": "H2 audit_event 查询",
        "owner": "Backend / database operator",
        "owner_zh": "后端 / 数据库操作人",
        "deliverable": "Audit query refs for M2/M3 scan, offline replay, and idempotency replay",
        "deliverable_zh": "M2/M3 scan、offline replay、idempotency replay 审计查询引用",
        "verification": "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
        "verification_zh": "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
    },
    {
        "resource": "L7 runner",
        "resource_zh": "L7 执行人",
        "owner": "Test owner",
        "owner_zh": "测试负责人",
        "deliverable": "L7 measured-facts run record; no local threshold invented",
        "deliverable_zh": "L7 实测事实执行记录；不发明本地性能阈值",
        "verification": "WAVE_3_PDA_L7_RUN_REF",
        "verification_zh": "WAVE_3_PDA_L7_RUN_REF",
    },
    {
        "resource": "WebView/Capacitor native shell",
        "resource_zh": "WebView/Capacitor Android native shell",
        "owner": "PDA technical verifier",
        "owner_zh": "PDA 技术验证负责人",
        "deliverable": (
            "Android native shell runtime evidence when stack candidate is "
            "webview-capacitor"
        ),
        "deliverable_zh": (
            "当技术栈候选为 webview-capacitor 时，归档 Android native shell "
            "真机证据"
        ),
        "verification": "WAVE_3_PDA_NATIVE_SHELL_REF",
        "verification_zh": "WAVE_3_PDA_NATIVE_SHELL_REF",
    },
    {
        "resource": "WebView/Capacitor native scan plugin",
        "resource_zh": "WebView/Capacitor native scan plugin",
        "owner": "PDA technical verifier",
        "owner_zh": "PDA 技术验证负责人",
        "deliverable": (
            "Native scan plugin runtime evidence when stack candidate is "
            "webview-capacitor"
        ),
        "deliverable_zh": (
            "当技术栈候选为 webview-capacitor 时，归档 native scan plugin "
            "实体扫码键证据"
        ),
        "verification": "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
        "verification_zh": "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
    },
    {
        "resource": "Operator usability reviewer",
        "resource_zh": "人工易用性走查人",
        "owner": "Business reviewer / test owner",
        "owner_zh": "业务走查人 / 测试负责人",
        "deliverable": "Grip, scan-key reachability, feedback, offline/error prompts, reconnect path",
        "deliverable_zh": "设备握持、扫码键触达、反馈、离线 / 错误提示和恢复网络路径",
        "verification": "WAVE_3_PDA_USABILITY_REVIEW_REF",
        "verification_zh": "WAVE_3_PDA_USABILITY_REVIEW_REF",
    },
)
FIELD_WORK_EXECUTION_ORDER_ZH = (
    "运维 / 部署负责人提供 WAVE_3_PDA_SERVICE_URL 并运行 service-precheck",
    "追溯码接口负责人 / 运维提供 OpenAPI URL 和 API key 并运行 trace-code OpenAPI precheck",
    "业务方 / 资产负责人提供至少一台真 PDA 并登记资产引用",
    "PDA 技术验证负责人确认实体扫码键或厂商扫码通道",
    "测试负责人准备 50 个脱敏条码样本和 M2/M3 测试数据",
    "测试执行人用真 PDA 采集 M2/M3 scan、offline replay 和 Idempotency-Key replay 日志",
    "PDA 技术验证负责人归档 SPIKE-005 / SPIKE-005B 真机实测结果",
    "后端 / 数据库操作人归档 H2 audit_event 查询证据",
    "测试负责人选择 from-env 或 intake 路径执行 check-only、正式 record 和 validate",
)
FIELD_WORK_RECORD_COMMANDS = (
    "just wave-3-pda-runtime-readiness --from-env --json",
    "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
    "just wave-3-pda-runtime-evidence-record --from-env --json",
    "just wave-3-pda-intake-check --json",
    "just wave-3-pda-intake-record --json",
    "just wave-3-pda-runtime-evidence-validate",
)
FIELD_WORK_TROUBLESHOOTING = (
    "If the evidence JSON already exists, rerun formal record with --force only after "
    "backing up or confirming replacement.",
    "Normal closeout must not use --force; keep the original evidence ref before "
    "confirming any replacement.",
    "Service precheck expects /healthz HTTP 200 with status=ok and the Wave3 route to "
    "return 401/AUTH-001 without Authorization; service-url must not point to "
    "local/prod/production/mock/fake/stub/example.",
    "Use scan input values containing scan-key, keyevent, intent, or datawedge; "
    "do not write camera/phone/browser.",
    "Match stack candidate to Spike refs: react-native uses spike-005, "
    "webview-capacitor uses spike-005b.",
    "Every evidence ref must include the current environment token: dev or staging.",
    "Set WAVE_3_PDA_* boolean variables to true only after the matching evidence is present.",
)
PREAUDIT_AUDIENCES = (
    "运维 / 部署负责人",
    "业务方 / 资产负责人 / 设备方",
    "测试负责人 / 业务数据负责人",
    "PDA 技术验证负责人",
    "后端 / 数据库操作人",
    "业务走查人 / 测试负责人",
)
PREAUDIT_NOW_ACTIONS: tuple[dict[str, str], ...] = (
    {
        "owner": "运维 / 部署负责人",
        "action": "确认 dev/staging 环境和 WAVE_3_PDA_SERVICE_URL",
        "proof": "just wave-3-pda-service-precheck --from-env --json 输出",
    },
    {
        "owner": "追溯码接口负责人 / 运维",
        "action": "确认追溯码 OpenAPI URL 和 API key 已通过只读预检",
        "proof": "just wave-3-pda-trace-code-openapi-precheck --from-env --json 输出",
    },
    {
        "owner": "测试负责人 / 业务数据负责人",
        "action": "准备 50 个脱敏条码样本、M2/M3 测试数据和测试账号",
        "proof": "条码样本清单、可重建测试数据说明、m2.write / m3.write 账号引用",
    },
    {
        "owner": "测试负责人",
        "action": "导出 evidence package 模板并预建归档目录",
        "proof": "just wave-3-pda-evidence-package-template 输出归档引用",
    },
    {
        "owner": "业务方 / 资产负责人 / 设备方",
        "action": "借测或采购至少一台真 PDA",
        "proof": "PDA 到位后登记 asset://.../pda/... 设备资产引用",
    },
)
PREAUDIT_MUST_NOT_DO = (
    "不要创建或伪造 docs/retros/wave-3-pda-runtime-evidence.json",
    "不要用浏览器、模拟器、手机摄像头或本地脚本替代真 PDA 实体扫码键",
    "不要在对应真实证据引用缺失时把 WAVE_3_PDA_* 布尔变量设为 true",
    "不要把 readiness、preaudit-kit、field-work-request、field-execution-summary 或 field-precheck-summary 输出当作关闭 W6.D gate 的 evidence",
)
PREAUDIT_REQUIRED_NOW_ENV_VARS = (
    "WAVE_3_PDA_ENVIRONMENT",
    "WAVE_3_PDA_SERVICE_URL",
    "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
    "WAVE_3_PDA_TRACE_CODE_API_KEY",
)
MATERIALS_CHECKLIST_FIELDS: tuple[dict[str, object], ...] = (
    {
        "name": "WAVE_3_PDA_ENVIRONMENT",
        "source_owner": "运维 / 部署负责人",
        "evidence_source": "dev/staging 环境标识",
        "no_pda_stage": "preparable",
        "requires_real_pda": False,
        "evidence_requirement": "dev/staging M2/M3 API",
    },
    {
        "name": "WAVE_3_PDA_SERVICE_URL",
        "source_owner": "运维 / 部署负责人",
        "evidence_source": "dev/staging wms-api 地址，service-precheck 会检查 /healthz 和 Wave3 鉴权边界",
        "no_pda_stage": "preparable",
        "requires_real_pda": False,
        "evidence_requirement": "dev/staging M2/M3 API",
    },
    {
        "name": "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL",
        "source_owner": "追溯码接口负责人 / 运维",
        "evidence_source": "追溯码 OpenAPI 只读合约地址",
        "no_pda_stage": "preparable",
        "requires_real_pda": False,
        "evidence_requirement": "追溯码 OpenAPI 合约",
    },
    {
        "name": "WAVE_3_PDA_TRACE_CODE_API_KEY",
        "source_owner": "追溯码接口负责人 / 运维",
        "evidence_source": "追溯码 OpenAPI X-API-Key，必须来自环境变量或 secret 管理系统",
        "no_pda_stage": "preparable",
        "requires_real_pda": False,
        "evidence_requirement": "追溯码 OpenAPI 合约",
    },
    {
        "name": "WAVE_3_PDA_PDA_MODEL",
        "source_owner": "设备借测 / 资产负责人",
        "evidence_source": "真 PDA 设备资产登记或现场照片归档",
        "no_pda_stage": "blocked_until_device",
        "requires_real_pda": True,
        "evidence_requirement": "PDA 资产引用",
    },
    {
        "name": "WAVE_3_PDA_ANDROID_VERSION",
        "source_owner": "设备借测 / 资产负责人",
        "evidence_source": "真 PDA 系统版本记录",
        "no_pda_stage": "blocked_until_device",
        "requires_real_pda": True,
        "evidence_requirement": "PDA 资产引用",
    },
    {
        "name": "WAVE_3_PDA_SCAN_INPUT_METHOD",
        "source_owner": "PDA 技术验证负责人",
        "evidence_source": "实体扫码键或厂商扫码通道，如 scan-key / KeyEvent / Intent / DataWedge",
        "no_pda_stage": "blocked_until_device",
        "requires_real_pda": True,
        "evidence_requirement": "实体扫码键",
    },
    {
        "name": "WAVE_3_PDA_STACK_CANDIDATE",
        "source_owner": "PDA 技术验证负责人",
        "evidence_source": "SPIKE-005 或 SPIKE-005B 测试计划和真机实测结论",
        "no_pda_stage": "plan_only",
        "requires_real_pda": True,
        "evidence_requirement": "L7 执行环境",
    },
    {
        "name": "WAVE_3_PDA_PDA_DEVICE_REF",
        "source_owner": "资产负责人",
        "evidence_source": "asset://.../pda/... 设备资产引用",
        "no_pda_stage": "blocked_until_device",
        "requires_real_pda": True,
        "evidence_requirement": "PDA 资产引用",
    },
    {
        "name": "WAVE_3_PDA_SPIKE_RESULT_REF",
        "source_owner": "PDA 技术验证负责人",
        "evidence_source": "SPIKE-005 / SPIKE-005B 真机实测结果归档",
        "no_pda_stage": "plan_only",
        "requires_real_pda": True,
        "evidence_requirement": "L7 执行记录",
    },
    {
        "name": "WAVE_3_PDA_M2_SCAN_LOG_REF",
        "source_owner": "测试执行人",
        "evidence_source": "真 PDA 扫描 M2 收货 / 验收 / 上架流程的 dev/staging 日志引用",
        "no_pda_stage": "prepare_test_data_only",
        "requires_real_pda": True,
        "evidence_requirement": "扫码日志",
    },
    {
        "name": "WAVE_3_PDA_M3_SCAN_LOG_REF",
        "source_owner": "测试执行人",
        "evidence_source": "真 PDA 扫描 M3 库存查询 / 状态变更流程的 dev/staging 日志引用",
        "no_pda_stage": "prepare_test_data_only",
        "requires_real_pda": True,
        "evidence_requirement": "扫码日志",
    },
    {
        "name": "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF",
        "source_owner": "测试执行人",
        "evidence_source": "真 PDA 离线暂存后恢复网络 replay 的日志引用",
        "no_pda_stage": "prepare_steps_only",
        "requires_real_pda": True,
        "evidence_requirement": "离线 replay 日志",
    },
    {
        "name": "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF",
        "source_owner": "测试执行人 / 后端负责人",
        "evidence_source": "同一 Idempotency-Key replay 的真实日志引用",
        "no_pda_stage": "prepare_steps_only",
        "requires_real_pda": True,
        "evidence_requirement": "idempotency replay 日志",
    },
    {
        "name": "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF",
        "source_owner": "后端 / 数据库操作人",
        "evidence_source": "查询 H2 audit_event 中 M2/M3/PDA 操作落库的证据引用",
        "no_pda_stage": "blocked_until_real_scan",
        "requires_real_pda": True,
        "evidence_requirement": "audit_event 查询",
    },
    {
        "name": "WAVE_3_PDA_L7_RUN_REF",
        "source_owner": "测试负责人",
        "evidence_source": "真 PDA L7 执行记录，记录实测事实，不设本地性能阈值",
        "no_pda_stage": "prepare_template_only",
        "requires_real_pda": True,
        "evidence_requirement": "L7 执行记录",
    },
    {
        "name": "WAVE_3_PDA_USABILITY_REVIEW_REF",
        "source_owner": "测试负责人 / 业务走查人",
        "evidence_source": "操作员现场易用性走查记录",
        "no_pda_stage": "prepare_template_only",
        "requires_real_pda": True,
        "evidence_requirement": "走查记录",
    },
    {
        "name": "WAVE_3_PDA_NATIVE_SHELL_REF",
        "source_owner": "PDA 技术验证负责人",
        "evidence_source": "WebView/Capacitor 候选的 Android native shell 证据",
        "no_pda_stage": "plan_only",
        "requires_real_pda": True,
        "evidence_requirement": "L7 执行记录",
    },
    {
        "name": "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
        "source_owner": "PDA 技术验证负责人",
        "evidence_source": "WebView/Capacitor 候选的 native scan plugin 证据",
        "no_pda_stage": "plan_only",
        "requires_real_pda": True,
        "evidence_requirement": "实体扫码键",
    },
    {
        "name": "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED",
        "source_owner": "测试负责人",
        "evidence_source": "现场扫码样本计数，目标沿用 50 个脱敏条码样本",
        "no_pda_stage": "preparable",
        "requires_real_pda": True,
        "evidence_requirement": "扫码日志",
    },
    {
        "name": "WAVE_3_PDA_M2_OPERATIONS_EXERCISED",
        "source_owner": "后端 / 测试负责人",
        "evidence_source": "真 PDA 执行 M2 收货 / 验收 / 上架操作后的计数",
        "no_pda_stage": "prepare_test_data_only",
        "requires_real_pda": True,
        "evidence_requirement": "扫码日志",
    },
    {
        "name": "WAVE_3_PDA_M3_OPERATIONS_EXERCISED",
        "source_owner": "后端 / 测试负责人",
        "evidence_source": "真 PDA 执行 M3 库存查询 / 状态变更操作后的计数",
        "no_pda_stage": "prepare_test_data_only",
        "requires_real_pda": True,
        "evidence_requirement": "扫码日志",
    },
    {
        "name": "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED",
        "source_owner": "测试负责人",
        "evidence_source": "现场离线 replay 计数，目标沿用 50 次",
        "no_pda_stage": "prepare_steps_only",
        "requires_real_pda": True,
        "evidence_requirement": "离线 replay 日志",
    },
    {
        "name": "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED",
        "source_owner": "测试负责人 / 后端负责人",
        "evidence_source": "现场 Idempotency-Key replay 计数，目标沿用 50 次",
        "no_pda_stage": "prepare_steps_only",
        "requires_real_pda": True,
        "evidence_requirement": "idempotency replay 日志",
    },
    {
        "name": "WAVE_3_PDA_REAL_PDA_USED",
        "source_owner": "现场负责人",
        "evidence_source": "真 PDA、扫码日志和设备资产引用全部到位后确认",
        "no_pda_stage": "blocked_until_real_scan",
        "requires_real_pda": True,
        "evidence_requirement": "PDA 资产引用",
    },
    {
        "name": "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED",
        "source_owner": "现场负责人",
        "evidence_source": "实体扫码键或厂商扫码通道稳定触发后确认",
        "no_pda_stage": "blocked_until_real_scan",
        "requires_real_pda": True,
        "evidence_requirement": "实体扫码键",
    },
    {
        "name": "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
        "source_owner": "运维 / 部署负责人",
        "evidence_source": "dev/staging service-precheck 或 readiness 输出确认服务前置可达",
        "no_pda_stage": "preparable",
        "requires_real_pda": False,
        "evidence_requirement": "dev/staging M2/M3 API",
    },
    {
        "name": "WAVE_3_PDA_AUDIT_EVENT_VERIFIED",
        "source_owner": "后端 / 数据库操作人",
        "evidence_source": "真 PDA 执行后查询 H2 audit_event 并确认 M2/M3/PDA 事件落库",
        "no_pda_stage": "blocked_until_real_scan",
        "requires_real_pda": True,
        "evidence_requirement": "audit_event 查询",
    },
    {
        "name": "WAVE_3_PDA_L7_REVIEW_COMPLETED",
        "source_owner": "测试负责人",
        "evidence_source": "真 PDA L7 执行记录已归档并复核",
        "no_pda_stage": "prepare_template_only",
        "requires_real_pda": True,
        "evidence_requirement": "L7 执行记录",
    },
    {
        "name": "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED",
        "source_owner": "测试负责人 / 业务走查人",
        "evidence_source": "操作员易用性走查记录已归档并复核",
        "no_pda_stage": "prepare_template_only",
        "requires_real_pda": True,
        "evidence_requirement": "走查记录",
    },
)

STRING_FIELDS = (
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
WEBVIEW_CAPACITOR_FIELDS = (
    "native_shell_ref",
    "native_scan_plugin_ref",
)
COUNT_FIELDS = (
    "barcode_samples_scanned",
    "m2_operations_exercised",
    "m3_operations_exercised",
    "offline_replays_exercised",
    "idempotency_replays_exercised",
)
FLAG_FIELDS = (
    "real_pda_used",
    "physical_scan_key_verified",
    "dev_or_staging_service_verified",
    "audit_event_verified",
    "l7_review_completed",
    "usability_review_completed",
)
ENV_STRING_FIELDS = {
    "environment": "WAVE_3_PDA_ENVIRONMENT",
    "service_url": "WAVE_3_PDA_SERVICE_URL",
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
ENV_COUNT_FIELDS = {
    "barcode_samples_scanned": "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED",
    "m2_operations_exercised": "WAVE_3_PDA_M2_OPERATIONS_EXERCISED",
    "m3_operations_exercised": "WAVE_3_PDA_M3_OPERATIONS_EXERCISED",
    "offline_replays_exercised": "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED",
    "idempotency_replays_exercised": "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED",
}
ENV_FLAG_FIELDS = {
    "real_pda_used": "WAVE_3_PDA_REAL_PDA_USED",
    "physical_scan_key_verified": "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED",
    "dev_or_staging_service_verified": "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
    "audit_event_verified": "WAVE_3_PDA_AUDIT_EVENT_VERIFIED",
    "l7_review_completed": "WAVE_3_PDA_L7_REVIEW_COMPLETED",
    "usability_review_completed": "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED",
}
NO_PDA_PRECHECK_FLAG_ENV_VARS = (
    ENV_FLAG_FIELDS["dev_or_staging_service_verified"],
)
REAL_EVIDENCE_FLAG_ENV_VARS = tuple(
    env_var
    for field, env_var in ENV_FLAG_FIELDS.items()
    if field != "dev_or_staging_service_verified"
)
ENV_FIELDS = {
    **ENV_STRING_FIELDS,
    **ENV_COUNT_FIELDS,
    **ENV_FLAG_FIELDS,
}
ENV_VAR_OWNER_DETAILS = {
    str(field["name"]): {
        "env_var": str(field["name"]),
        "source_owner": str(field["source_owner"]),
        "no_pda_stage": str(field["no_pda_stage"]),
        "requires_real_pda": bool(field["requires_real_pda"]),
        "evidence_requirement": str(field["evidence_requirement"]),
    }
    for field in MATERIALS_CHECKLIST_FIELDS
}
TRUE_ENV_VALUES = {"true", "1", "yes", "on"}
FALSE_ENV_VALUES = {"false", "0", "no", "off", ""}


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
