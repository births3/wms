"""Constants for Wave 3 PDA runtime readiness."""
from pathlib import Path

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
