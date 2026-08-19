"""Wave 3 PDA readiness 文档边界治理测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave3_pda_field_precheck_attachment_is_sanitized_and_non_closing():
    """真实前置预检附件必须不含密钥，且不能被当作 W6.D runtime evidence。"""
    attachment_path = Path("docs/retros/wave-3-pda-field-precheck-2026-06-14.json")
    text = attachment_path.read_text(encoding="utf-8")
    payload = json.loads(text)

    assert payload["kind"] == "wave3-pda-field-precheck-attachment"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["runtime_evidence_file"] == (
        "docs/retros/wave-3-pda-runtime-evidence.json"
    )
    assert payload["service_precheck"]["ok"] is True
    assert payload["service_precheck"]["environment"] == "staging"
    assert payload["service_precheck"]["healthz_status"] == 200
    assert payload["service_precheck"]["wave3_route_error_code"] == "AUTH-001"
    assert payload["trace_code_openapi_precheck"]["ok"] is True
    assert payload["trace_code_openapi_precheck"]["openapi"] == "3.0.3"
    assert payload["trace_code_openapi_precheck"]["api_key_header_name"] == "X-API-Key"
    assert payload["trace_code_openapi_precheck"]["required_paths_present"] == [
        "/api/codes/{code}",
        "/api/codes/{code}/children",
        "/api/codes/batch",
        "/api/codes/verify",
        "/api/wms-products",
    ]
    assert payload["trace_code_network_diagnostics"] == {
        "captured_at": "2026-06-14T13:00:36+08:00",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "direct_no_proxy_status": 200,
        "default_proxy_path_status": 502,
        "alternate_proxy_192_168_124_5_7890_status": "timeout",
        "remote_9200_status": "timeout",
        "recommended_curl_option": "--noproxy '*'",
        "recommended_env_override": "NO_PROXY='*' no_proxy='*'",
        "note": (
            "Trace-code OpenAPI is reachable on 43.128.77.47:9100 by direct "
            "no-proxy access; proxy paths may return 502 or timeout and must "
            "not be treated as OpenAPI contract failure."
        ),
    }
    assert payload["field_status"]["ready_for_record_from_env_vars"] is False
    assert payload["field_status"]["missing_now_env_vars"] == []
    assert payload["field_status"]["real_pda_missing_env_vars_count"] == 23
    assert payload["field_status"]["false_truth_flag_env_vars_count"] == 5
    assert payload["field_status"]["no_pda_precheck_verified_flag_env_vars"] == [
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
    ]
    assert payload["field_status"]["remaining_no_pda_precheck_false_flag_env_vars"] == []
    assert payload["field_status"]["remaining_real_evidence_false_flag_env_vars"] == [
        "WAVE_3_PDA_REAL_PDA_USED",
        "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED",
        "WAVE_3_PDA_AUDIT_EVENT_VERIFIED",
        "WAVE_3_PDA_L7_REVIEW_COMPLETED",
        "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED",
    ]
    assert payload["owner_actions"] == [
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
    ]
    assert payload["record_gate_after_owner_actions"] == [
        "just wave-3-pda-runtime-readiness --from-env --json",
        "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
        "just wave-3-pda-runtime-evidence-record --from-env --json",
        "just wave-3-pda-intake-check --json",
        "just wave-3-pda-intake-record --json",
        "just wave-3-pda-runtime-evidence-validate",
    ]
    assert "wms_" not in text
    assert "sk-" not in text
    assert "<secret-from-secret-manager>" in text


def test_wave3_pda_readiness_documents_trace_code_proxy_diagnostics():
    """追溯码 OpenAPI 预检必须说明代理 502 与直连 200 的排障路径。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "网络排障补充" in text
    assert "当前可用入口是 `43.128.77.47:9100`" in text
    assert "远端 `9200` 当前连接超时" in text
    assert "`NO_PROXY='*' no_proxy='*'`" in text
    assert "`curl --noproxy '*'`" in text
    assert "代理 `502`" in text
    assert "OpenAPI 合约失败" in text
    assert "`192.168.124.5:7890` 代理路径当前连接超时" in text
    assert "trace_code_network_diagnostics.direct_no_proxy_status=200" in text
    assert "trace_code_network_diagnostics.default_proxy_path_status=502" in text
    assert (
        "trace_code_network_diagnostics.alternate_proxy_192_168_124_5_7890_status=timeout"
        in text
    )
    assert "trace_code_network_diagnostics.remote_9200_status=timeout" in text


def test_wave3_pda_readiness_records_rn_and_webview_candidates(monkeypatch):
    """PDA readiness 必须同时记录 RN 与 WebView/Capacitor 候选边界。"""
    import report_wave3_completion as report

    expected = {
        "docs/runbooks/wave-3-pda-readiness.md": (
            "SPIKE-005B",
            "不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖",
            "设备清单",
            "蓝牙打印",
            "Wave 5",
            "docs/retros/wave-5-hardware-evidence.json",
        ),
        "docs/spikes/spike-005-rn-scanner.md": (
            "7.7 Wave 3 readiness 决策",
            "先落 readiness/runbook",
            "真 PDA",
            "手机摄像头不能作为 SPIKE-005 evidence",
        ),
        "docs/spikes/spike-005b-webview-capacitor-pda.md": (
            "7.1 用户确认",
            "WebView/Capacitor native shell",
            "不直接替换 ADR-0001",
        ),
        "docs/adr/0027-pda-offline-model.md": (
            "PDA 离线模型与技术栈定版框架",
            "react-native",
            "webview-capacitor",
            "本 ADR 进入 Accepted 的前置条件",
            "apps/pda-mobile",
        ),
        "docs/domain/clarifications.md": (
            "PDA 端推进方式",
            "SPIKE-005 / SPIKE-005B readiness",
            "PDA Web 打包方案边界",
            "ADR-0027 定版",
            "不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖",
        ),
    }

    seen_paths = []
    seen_needles = {}

    def fake_file_contains(path, *needles):
        seen_paths.append(path)
        seen_needles[path] = needles
        return path in expected and all(needle in expected[path] for needle in needles)

    monkeypatch.setattr(report, "file_contains", fake_file_contains)

    assert report.pda_readiness_recorded() is True
    assert set(expected).issubset(set(seen_paths))
    readiness_needles = seen_needles["docs/runbooks/wave-3-pda-readiness.md"]
    assert "蓝牙打印" in readiness_needles
    assert "docs/retros/wave-5-hardware-evidence.json" in readiness_needles
    rn_needles = seen_needles["docs/spikes/spike-005-rn-scanner.md"]
    assert "真 PDA" in rn_needles
    assert "手机摄像头不能作为 SPIKE-005 evidence" in rn_needles
    clarification_needles = seen_needles["docs/domain/clarifications.md"]
    assert "不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖" in clarification_needles


def test_spike005_rn_scanner_keeps_spike_package_out_of_production_workspace():
    """SPIKE-005 PoC 不能再被文档描述成 pnpm production workspace member。"""
    text = Path("docs/spikes/spike-005-rn-scanner.md").read_text(encoding="utf-8")

    assert "workspace member" not in text
    assert "spike-local package" in text


def test_wave05_retro_records_adr0027_as_proposed_not_placeholder():
    """Wave 0.5 retro 的 ADR 状态表不能把已建立的 ADR-0027 继续写成占位。"""
    text = Path("docs/retros/wave-0.5-retro.md").read_text(encoding="utf-8")
    spike005_line = next(line for line in text.splitlines() if "| SPIKE-005" in line)
    adr0027_line = next(line for line in text.splitlines() if "| 0027" in line)

    assert "ADR-0027（Proposed）" in spike005_line
    assert "拟产出 ADR-0027" not in spike005_line
    assert "Proposed" in adr0027_line
    assert "已建" in adr0027_line
    assert "WebView/Capacitor" in adr0027_line
    assert "真 PDA" in adr0027_line
    assert "占位" not in adr0027_line
    assert "SPIKE-005 accept 后写" not in adr0027_line


def test_wave3_pda_readiness_rejects_missing_printing_handoff(monkeypatch):
    """PDA readiness 缺少蓝牙打印到 Wave 5 evidence 的交接说明时不能通过。"""
    import report_wave3_completion as report

    docs = {
        "docs/runbooks/wave-3-pda-readiness.md": (
            "SPIKE-005B\n"
            "不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖\n"
            "设备清单\n"
        ),
        "docs/spikes/spike-005-rn-scanner.md": (
            "7.7 Wave 3 readiness 决策\n"
            "先落 readiness/runbook\n"
            "真 PDA\n"
            "手机摄像头不能作为 SPIKE-005 evidence\n"
        ),
        "docs/spikes/spike-005b-webview-capacitor-pda.md": (
            "7.1 用户确认\n"
            "WebView/Capacitor native shell\n"
            "不直接替换 ADR-0001\n"
        ),
        "docs/adr/0027-pda-offline-model.md": (
            "PDA 离线模型与技术栈定版框架\n"
            "react-native\n"
            "webview-capacitor\n"
            "本 ADR 进入 Accepted 的前置条件\n"
            "apps/pda-mobile\n"
        ),
        "docs/domain/clarifications.md": (
            "PDA 端推进方式\n"
            "SPIKE-005 / SPIKE-005B readiness\n"
            "PDA Web 打包方案边界\n"
            "ADR-0027 定版\n"
            "不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖\n"
        ),
    }

    monkeypatch.setattr(
        report,
        "file_contains",
        lambda path, *needles: all(needle in docs.get(path, "") for needle in needles),
    )

    assert report.pda_readiness_recorded() is False


def test_wave3_pda_readiness_documents_runtime_readiness_entry():
    """PDA runbook 必须说明 runtime readiness 只读入口和边界。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "just wave-3-pda-runtime-readiness" in text
    assert "just wave-3-pda-service-precheck" in text
    assert "just wave-3-pda-materials-checklist --json" in text
    assert "just wave-3-pda-field-work-request" in text
    assert "just wave-3-pda-field-execution-summary --json" in text
    assert "just wave-3-pda-field-precheck-summary --from-env\n" in text
    assert "just wave-3-pda-field-precheck-summary --from-env --json" in text
    assert "just wave-3-pda-field-owner-gap-actions\n" in text
    assert "just wave-3-pda-field-owner-gap-actions --json" in text
    assert "just wave-3-pda-evidence-package-template" in text
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in text
    assert "只输出现场字段分工" in text
    assert "只输出可转发资源申请包" in text
    assert "只输出 Markdown 证据包模板" in text
    assert "现场前置一键预检" in text
    assert "只读组合 `service_precheck`、`trace_code_openapi_precheck` 和 `field_execution_summary`" in text
    assert "`Missing Now Env Vars`" in text
    assert "列出变量名和负责人" in text
    assert "owner 缺口动作单" in text
    assert "`Missing now`" in text
    assert "`Real evidence vars`" in text
    assert "`False flags`" in text
    assert "`field_owner_gap_actions`" in text
    assert "`source_owner`" in text
    assert "`env_vars`" in text
    assert "`evidence_requirements`" in text
    assert "WAVE_3_PDA_SERVICE_URL" in text
    assert "WAVE_3_PDA_TRACE_CODE_OPENAPI_URL" in text
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY" in text
    assert "不打印 key" in text
    assert "追溯码接口负责人 / 运维" in text
    assert "追溯码 OpenAPI" in text
    assert "GET /api/codes/{code}" in text
    assert "POST /api/codes/batch" in text
    assert "X-API-Key" in text
    assert "docs/retros/wave-3-pda-field-precheck-2026-06-14.json" in text
    assert "`owner_actions` 现场采集动作" in text
    assert "正式 record gate 命令顺序" in text
    assert "<secret-from-secret-manager>" in text
    assert "不保存真实 API key" in text
    assert "模板中的命令先运行 `just wave-3-pda-service-precheck`" in text
    assert "just wave-3-pda-service-precheck --from-env --json" in text
    assert "模板中的完整命令先运行 `just wave-3-pda-runtime-readiness`" in text
    assert "just wave-3-pda-runtime-evidence-record --from-env --check-only --json" in text
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in text
    assert "`just wave-3-pda-service-precheck --json`" in text
    assert "`readiness --from-env --json` 遇到缺失变量时会输出 `missing_env_vars`" in text
    assert "`missing_env_var_owners`" in text
    assert "`missing_args`" in text
    assert "`missing_env_vars`" in text
    assert "填写 PDA 型号、Android 版本、证据引用等值时保留单引号" in text
    assert "避免 `Honeywell EDA52` / `Android 11` 这类带空格值被 shell 拆词" in text
    assert "不会写入 `docs/retros/wave-3-pda-runtime-evidence.json`" in text
    assert "不能关闭 W6.D gate" in text


def test_wave3_pda_readiness_documents_intake_file_flow():
    """W6.D runbook 必须说明 JSON intake 文件的导出、校验和只读边界。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## W6.D JSON intake 采集流程" in text
    assert "just wave-3-pda-intake-template --json" in text
    assert "wave3-pda-runtime-evidence-intake-template" in text
    assert "wave3-pda-runtime-evidence-intake" in text
    assert "--intake-template-output docs/retros/wave-3-pda-intake-template-2026-06-14.json" in text
    assert "--intake-template-force" in text
    assert "该文件仍只是待填 intake 模板，不是 runtime evidence" in text
    assert "空字符串表示对应证据字段仍未填写" in text
    assert "`false_flag_env_vars`" in text
    assert "just wave-3-pda-intake-check --json" in text
    assert "WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE" in text
    assert "wave-3-pda-runtime-evidence-intake.staging.json" not in text
    assert "--check-only --json" in text
    assert "只读校验现场 JSON" in text
    assert "不写 `docs/retros/wave-3-pda-runtime-evidence.json`" in text
    assert "不能关闭 W6.D gate" in text
    assert "不得把 trace-code API key 写入 intake 文件" in text
    assert "truth flags" in text
    assert "`real_pda_used`" in text
    assert "`physical_scan_key_verified`" in text
    assert "`audit_event_verified`" in text
    assert "`l7_review_completed`" in text
    assert "`usability_review_completed`" in text
    assert "`pda_stack_candidate=webview-capacitor`" in text
    assert "`native_shell_ref`" in text
    assert "`native_scan_plugin_ref`" in text
    assert "record check-only 通过后" in text
    assert "正式 record 仍必须使用同一份真实材料" in text


def test_wave3_pda_readiness_command_emits_json_attachment():
    """runbook 的 readiness 命令必须实际输出 JSON，便于现场归档预检附件。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    start = text.index("just wave-3-pda-runtime-readiness --from-env --json")
    end = text.index("```", start)
    readiness_command = text[start:end]

    assert "--from-env" in readiness_command
    assert "--json" in readiness_command


def test_wave3_pda_readiness_records_staging_service_dry_run_without_closing_gate():
    """PDA runbook 可以记录 staging 服务前置，但必须继续阻断真 PDA evidence。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## Staging 服务前置 Dry Run - 2026-06-08" in text
    assert "healthz_status=200" in text
    assert "wave3_route_status=401" in text
    assert "wave3_route_error_code=AUTH-001" in text
    assert "audit_event count=0" not in text
    assert "pda audit_event count=0" not in text
    assert "不能写入 `docs/retros/wave-3-pda-runtime-evidence.json`" in text
    assert "不能关闭 W6.D gate" in text
    assert "真 PDA、实体扫码键、离线 replay、幂等 replay、L7 和易用性 evidence" in text


def test_wave3_pda_readiness_documents_no_pda_progress_boundary():
    """W6.D 必须说明没有真 PDA 时能推进什么、不能关闭什么。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## 当前无 PDA 时的推进口径" in text
    assert "先运行 `just wave-3-pda-materials-checklist --json`" in text
    assert "不写 `docs/retros/wave-3-pda-runtime-evidence.json`" in text
    assert "不把 `real_pda_used` 预填为 `true`" in text
    assert "不把 `physical_scan_key_verified` 预填为 `true`" in text
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in text
    assert "追溯码查询 OpenAPI 前置" in text
    assert "汇总当前变量缺口" in text
    assert "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED" in text
    assert "service precheck 输出归档后设为 `true`" in text
    assert "无 PDA 阶段不要把这些真实 evidence 变量预填为 `true`" in text
    assert "真 PDA 到位后才执行" in text
    assert "50 个脱敏条码样本" in text


def test_wave3_pda_readiness_documents_operator_materials_table():
    """W6.D runbook 必须把现场采集字段映射到来源和无 PDA 推进边界。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## W6.D 现场采集字段表" in text
    assert "字段 / 变量" in text
    assert "来源 / 负责人" in text
    assert "无 PDA 阶段能否提前准备" in text
    assert "`WAVE_3_PDA_SERVICE_URL`" in text
    assert "`WAVE_3_PDA_TRACE_CODE_OPENAPI_URL` / `WAVE_3_PDA_TRACE_CODE_API_KEY`" in text
    assert "追溯码接口负责人 / 运维" in text
    assert "不得把真实 key 写入仓库、截图或 evidence JSON" in text
    assert "`WAVE_3_PDA_PDA_MODEL` / `WAVE_3_PDA_ANDROID_VERSION`" in text
    assert "`WAVE_3_PDA_SCAN_INPUT_METHOD`" in text
    assert "`WAVE_3_PDA_M2_SCAN_LOG_REF` / `WAVE_3_PDA_M3_SCAN_LOG_REF`" in text
    assert "`WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF`" in text
    assert "`WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF`" in text
    assert "`WAVE_3_PDA_AUDIT_EVENT_QUERY_REF`" in text
    assert "`WAVE_3_PDA_L7_RUN_REF` / `WAVE_3_PDA_USABILITY_REVIEW_REF`" in text
    assert "`WAVE_3_PDA_REAL_PDA_USED` / `WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED`" in text
    assert "`WAVE_3_PDA_AUDIT_EVENT_VERIFIED` / `WAVE_3_PDA_L7_REVIEW_COMPLETED`" in text
    assert "只能在真 PDA 实扫后置为 `true`" in text
    assert "可以提前准备条码清单和 M2/M3 测试数据" in text
    assert "操作员现场走查清单" in text
    assert "归档到 `WAVE_3_PDA_USABILITY_REVIEW_REF`" in text


def test_wave3_pda_readiness_documents_field_glossary_for_site_operators():
    """W6.D runbook 必须给非技术现场人员解释关键术语。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## 术语速查" in text
    assert "真 PDA" in text
    assert "实体扫码键 / scan-key" in text
    assert "`scan_input_method`" in text
    assert "L7" in text
    assert "offline replay" in text
    assert "Idempotency-Key replay" in text
    assert "H2 `audit_event`" in text
    assert "evidence ref" in text
    assert "trace-code OpenAPI precheck" in text
    assert "不能用普通手机、浏览器、模拟器、摄像头扫码替代" in text
    assert "只记录实测事实，不发明本地性能阈值" in text
    assert "必须记录首次请求、重放请求和响应一致性" in text


def test_wave3_pda_readiness_documents_evidence_naming_and_archive_rules():
    """W6.D runbook 必须说明证据命名和归档规则，降低现场材料不一致风险。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## 证据命名与归档规则" in text
    assert "Evidence JSON 只保存引用" in text
    assert "`asset://wms-staging/pda/honeywell-eda52-01`" in text
    assert "`ci/staging/wave3-pda-m2-scan/run-20260614-01`" in text
    assert "`ci/staging/wave3-pda-audit-event/query-20260614-01`" in text
    assert "`s3://wms-staging-evidence/wave3/pda/usability-review-20260614-01.md`" in text
    assert "`ci/staging/wave3-pda-trace-code-openapi-precheck/run-20260614-01`" in text
    assert "场景名建议固定为" in text
    assert "`idempotency-replay`" in text
    assert "`trace-code-openapi-precheck`" in text
    assert "所有引用必须显式包含 `dev` 或 `staging`" in text
    assert "API key、token、密码不得出现在文件名、截图、日志正文、证据包或 evidence JSON 中" in text


def test_wave3_pda_readiness_documents_role_command_expected_output_table():
    """W6.D runbook 必须给现场角色、命令、预期输出和失败排查入口。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )
    table_start = text.index("## W6.D 角色-命令-预期输出表")
    table_end = text.index("现场准备顺序：", table_start)
    table_text = text[table_start:table_end]

    assert "## W6.D 角色-命令-预期输出表" in text
    assert "除正式 record 外，其余命令都是只读准备或预检" in text
    assert "命令 / 动作" in text
    assert "预期输出" in text
    assert "失败时先看" in text
    assert "`just wave-3-pda-preaudit-kit --json`" in text
    assert "`just wave-3-pda-field-work-request --json`" in text
    assert "`just wave-3-pda-field-execution-summary --json`" in text
    assert "`just wave-3-pda-field-precheck-summary --from-env`；结构化附件用" in text
    assert "`just wave-3-pda-field-precheck-summary --from-env --json`" in text
    assert "`just wave-3-pda-field-owner-gap-actions`；结构化附件用" in text
    assert "`just wave-3-pda-field-owner-gap-actions --json`" in text
    assert "`just wave-3-pda-field-handoff-bundle --json`" in text
    assert "`just wave-3-pda-field-handoff-bundle --from-env --json`" in text
    assert "`just wave-3-pda-service-precheck --from-env --json`" in text
    assert "`just wave-3-pda-trace-code-openapi-precheck --from-env --json`" in text
    assert "`just wave-3-pda-runtime-readiness --from-env --json`" in text
    assert "`just wave-3-pda-runtime-evidence-record --from-env --check-only --json`" in text
    assert "`just wave-3-pda-runtime-evidence-record --from-env --json`" in text
    assert "`just wave-3-pda-intake-check --json`" in table_text
    assert "`just wave-3-pda-intake-record --json`" in table_text
    assert "`just wave-3-pda-runtime-evidence-validate`" in text
    assert "`false_flag_env_vars`" in text
    assert "`false_flag_env_var_owners`" in text
    assert "`/healthz` 为 200" in text
    assert "5 个 required GET/POST operations 存在" in text
    assert "`service_precheck` 与 `trace_code_openapi_precheck` 都通过" in text
    assert "`writes_runtime_evidence=false`、`closes_gate=false`" in text
    assert "聚合 `preaudit_kit`、`materials_checklist`、`field_work_request`" in text
    assert "按 `source_owner` 聚合" in text
    assert "`current_env_status`" in text
    assert "`real_pda_missing_env_vars`" in text
    assert "`real_pda_missing_env_var_owners`" in text
    assert "`truth_flag_env_vars`" in text
    assert "`no_pda_precheck_truth_flag_env_vars`" in text
    assert "`false_truth_flag_env_vars`" in text
    assert "`false_truth_flag_env_var_owners`" in text
    assert "`false_no_pda_precheck_truth_flag_env_vars`" in text
    assert "`false_no_pda_precheck_truth_flag_env_var_owners`" in text
    assert "`false_real_evidence_truth_flag_env_vars`" in text
    assert "`false_real_evidence_truth_flag_env_var_owners`" in text
    assert "no_pda_precheck_verified_flag_env_vars" in text
    assert "remaining_no_pda_precheck_false_flag_env_vars" in text
    assert "remaining_real_evidence_false_flag_env_vars_count" in text
    assert "`record_commands`" in text
    assert "生成 `docs/retros/wave-3-pda-runtime-evidence.json`" in text


def test_wave3_pda_readiness_commands_use_export_template_variables():
    """runbook 中可执行命令必须复用 from-env，避免现场照抄长参数失效。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "just wave-3-pda-service-precheck --from-env --json" in text
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in text
    assert "just wave-3-pda-field-precheck-summary --from-env\n" in text
    assert "just wave-3-pda-field-precheck-summary --from-env --json" in text
    assert "just wave-3-pda-field-owner-gap-actions\n" in text
    assert "just wave-3-pda-field-owner-gap-actions --json" in text
    assert "just wave-3-pda-field-handoff-bundle --json" in text
    assert "just wave-3-pda-field-handoff-bundle --from-env --json" in text
    assert "just wave-3-pda-runtime-readiness --from-env --json" in text
    assert "just wave-3-pda-runtime-evidence-record --from-env --check-only --json" in text
    assert "just wave-3-pda-runtime-evidence-record --from-env --json" in text
    assert '--environment "$WAVE_3_PDA_ENVIRONMENT"' not in text
    assert '--service-url "$WAVE_3_PDA_SERVICE_URL"' not in text
    assert '--pda-model "$WAVE_3_PDA_PDA_MODEL"' not in text
    assert '--scan-input-method "$WAVE_3_PDA_SCAN_INPUT_METHOD"' not in text
    assert '--pda-stack-candidate "$WAVE_3_PDA_STACK_CANDIDATE"' not in text
    assert "$WAVE_3_PDA_MODEL" not in text
    assert "--pda-model 'Honeywell EDA52'" not in text
    assert "--android-version 'Android 11'" not in text
    assert "--scan-input-method 'physical-scan-key-intent'" not in text
    assert "--pda-stack-candidate react-native" not in text


def test_wave3_pda_readiness_commands_use_count_variables():
    """runbook 的现场命令必须通过 from-env 读取 export-template 的计数变量。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    command_start = text.index("```bash\njust wave-3-pda-service-precheck --from-env --json")
    command_end = text.index("## 输出", command_start)
    command_text = text[command_start:command_end]

    for env_name in (
        "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED",
        "WAVE_3_PDA_M2_OPERATIONS_EXERCISED",
        "WAVE_3_PDA_M3_OPERATIONS_EXERCISED",
        "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED",
        "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED",
    ):
        assert env_name in text

    assert "--barcode-samples-scanned 50" not in command_text
    assert "--m2-operations-exercised 1" not in command_text
    assert "--m3-operations-exercised 1" not in command_text
    assert "--offline-replays-exercised 50" not in command_text
    assert "--idempotency-replays-exercised 50" not in command_text


def test_wave3_pda_readiness_field_table_covers_from_env_variables():
    """W6.D 现场字段表必须覆盖 --from-env 读取的全部变量。"""
    import check_wave3_pda_runtime_readiness as readiness

    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    table_start = text.index("## W6.D 现场采集字段表")
    table_end = text.index("现场准备顺序：", table_start)
    table_text = text[table_start:table_end]

    expected_env_fields = (
        set(readiness.ENV_STRING_FIELDS.values())
        | set(readiness.ENV_COUNT_FIELDS.values())
        | set(readiness.ENV_FLAG_FIELDS.values())
    )
    for env_name in sorted(expected_env_fields):
        assert env_name in table_text


def test_wave3_pda_readiness_documents_evidence_package_minimums():
    """W6.D runbook 必须说明 evidence 引用背后的最小材料内容。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## W6.D 证据包最小内容" in text
    assert "just wave-3-pda-evidence-package-template" in text
    assert "just wave-3-pda-evidence-package-template --json" in text
    assert "`sections`" in text
    assert "`mapping_variables`" in text
    assert "`WAVE_3_PDA_NATIVE_SHELL_REF`" in text
    assert "`WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF`" in text
    assert "`pda_stack_candidate=webview-capacitor`" in text
    assert "`owner_actions`" in text
    assert "JSON 中的 `owner_actions` 可直接转成现场派单" in text
    assert "`record_gate_after_owner_actions`" in text
    assert "采齐后必须运行的 record gate 命令顺序" in text
    assert "`blocked_flags_until_refs_present`" in text
    assert "`warnings`" in text
    assert "WAVE_3_PDA_*" in text
    assert "`--from-env`" in text
    assert "每个日志或文档引用至少记录" in text
    assert "环境（dev 或 staging）" in text
    assert "PDA 设备资产引用" in text
    assert "执行时间" in text
    assert "测试账号 / 租户" in text
    assert "场景名称" in text
    assert "关键业务 ID" in text
    assert "结果摘要" in text
    assert "保存 readiness `--json` 输出" in text
    assert "不能把 readiness 输出当作关闭 W6.D gate 的 evidence" in text
    assert "L7 执行记录" in text
    assert "不设本地性能阈值" in text
    assert "操作员现场走查清单" in text
    assert "设备握持和扫码键触达" in text
    assert "离线提示" in text
    assert "trace-code OpenAPI precheck" in text
    assert "追溯码 OpenAPI 预检附件" in text
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY" in text
    assert "不得写入证据包、截图或 evidence JSON" in text


def test_wave3_pda_readiness_documents_webview_native_refs_for_all_checks():
    """WebView/Capacitor 候选必须提醒 from-env 与 intake 都读取 native refs。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "如果 `WAVE_3_PDA_STACK_CANDIDATE=webview-capacitor`" in text
    assert "from-env 路径的 readiness、record check-only 和正式 record 都通过 `--from-env`" in text
    assert "`just wave-3-pda-intake-check --json` 和 `just wave-3-pda-intake-record --json`" in text
    assert "WAVE_3_PDA_NATIVE_SHELL_REF" in text
    assert "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF" in text
    assert "WAVE_3_PDA_NATIVE_ARGS" not in text
    assert '--native-shell-ref "$WAVE_3_PDA_NATIVE_SHELL_REF"' not in text
    assert '--native-scan-plugin-ref "$WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF"' not in text


def test_wave3_pda_readiness_documents_field_work_request_package():
    """W6.D runbook 必须提供可转交给现场负责人的资源申请包。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## W6.D 现场资源申请包" in text
    assert "just wave-3-pda-field-work-request" in text
    assert "just wave-3-pda-field-execution-summary --json" in text
    assert "just wave-3-pda-field-precheck-summary --from-env\n" in text
    assert "just wave-3-pda-field-precheck-summary --from-env --json" in text
    assert "just wave-3-pda-field-owner-gap-actions\n" in text
    assert "just wave-3-pda-field-owner-gap-actions --json" in text
    assert "just wave-3-pda-field-handoff-bundle --json" in text
    assert "可直接转给业务方、运维、设备方和测试负责人" in text
    assert "资源项" in text
    assert "负责人" in text
    assert "交付物" in text
    assert "验证命令 / 证据变量" in text
    assert "`--json` 输出保留英文资源字段" in text
    assert "中文资源项、负责人、交付物、验证变量" in text
    assert "`execution_order_zh`" in text
    assert "`troubleshooting`" in text
    assert "`next_commands`" in text
    assert "至少一台真 PDA" in text
    assert "`asset://.../pda/...`" in text
    assert "`WAVE_3_PDA_SERVICE_URL`" in text
    assert "追溯码 OpenAPI 合约" in text
    assert "`WAVE_3_PDA_TRACE_CODE_OPENAPI_URL`" in text
    assert "`WAVE_3_PDA_TRACE_CODE_API_KEY`" in text
    assert "just wave-3-pda-trace-code-openapi-precheck --from-env --json" in text
    assert "一次性汇总服务、追溯码和字段缺口" in text
    assert "50 个脱敏条码样本" in text
    assert "M2/M3 测试数据" in text
    assert "L7 执行人" in text
    assert "WebView/Capacitor Android native shell" in text
    assert "WebView/Capacitor native scan plugin" in text
    assert "`WAVE_3_PDA_NATIVE_SHELL_REF`" in text
    assert "`WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF`" in text
    assert "人工易用性走查人" in text
    assert "不能把这张表当作 evidence JSON" in text
    assert "快速排障顺序" in text
    assert "`--force`" in text
    assert "正常 closeout 不追加 `--force`" in text
    assert "保留原 evidence 引用" in text
    assert "`AUTH-001`" in text
    assert "scan-key / KeyEvent / Intent / DataWedge" in text
    assert "追溯码接口负责人 / 运维提供 `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL`" in text


def test_wave3_pda_readiness_documents_field_handoff_bundle():
    """W6.D runbook 必须说明现场交接总包的内容和只读边界。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "just wave-3-pda-field-handoff-bundle --json" in text
    assert "just wave-3-pda-field-handoff-bundle --from-env --json" in text
    assert "--field-handoff-output docs/retros/wave-3-pda-field-handoff-2026-06-14.json" in text
    assert "--field-handoff-force" in text
    assert "只读聚合 W6.D 现场交接总包" in text
    assert "`preaudit_kit`" in text
    assert "`materials_checklist`" in text
    assert "`field_work_request`" in text
    assert "`field_execution_summary`" in text
    assert "`field_owner_gap_actions`" in text
    assert "`evidence_package_template`" in text
    assert "`intake_template`" in text
    assert "默认不联网、不打印变量值" in text
    assert "真实 key 仍只从环境变量或 secret 管理系统读取，不进入输出" in text
    assert "JSON intake 模板统一交给现场系统" in text
    assert "JSON 输出中的 `intake_template` 可直接交给现场系统生成待填 intake 文件" in text
    assert "just wave-3-pda-intake-template --json --intake-template-output" in text
    assert "它仍然只是模板，不是 `docs/retros/wave-3-pda-runtime-evidence.json`" in text
    assert "`--field-handoff-output` 只写交接附件，仍不能当作 evidence JSON" in text
    assert "默认拒绝覆盖" in text


def test_wave3_pda_readiness_documents_preaudit_current_env_status():
    """W6.D runbook 必须说明 preaudit-kit 的当前环境变量状态输出。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## W6.D 预审包" in text
    assert "`current_env_status`" in text
    assert "`required_now_env_vars`" in text
    assert "`set_now_env_vars`" in text
    assert "`missing_now_env_vars`" in text
    assert "`missing_now_env_var_owners`" in text
    assert "不输出环境变量值" in text
    assert "`WAVE_3_PDA_ENVIRONMENT`" in text
    assert "`WAVE_3_PDA_SERVICE_URL`" in text
    assert "`WAVE_3_PDA_TRACE_CODE_OPENAPI_URL`" in text
    assert "`WAVE_3_PDA_TRACE_CODE_API_KEY`" in text
    assert "追溯码接口负责人 / 运维" in text


def test_wave3_pda_readiness_documents_precheck_attachment_reuse():
    """runbook 必须说明脱敏前置附件只能减少重复前置变量，不关闭 W6.D。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "--field-precheck-attachment" in text
    assert "docs/retros/wave-3-pda-field-precheck-2026-06-14.json" in text
    assert "service_precheck" in text
    assert "trace_code_openapi_precheck" in text
    assert "`AUTH-001`" in text
    assert "`X-API-Key`" in text
    assert "5 个 required GET/POST operations 全部存在" in text
    assert "不写 runtime evidence，不能关闭 W6.D gate" in text
    assert "field_execution_summary.false_truth_flag_env_vars_count=5" in text
    assert "satisfied_by_precheck_attachment_truth_flag_env_vars" in text
    assert "dev_or_staging_service_verified=true" in text
    assert "它仍不进入最终 evidence JSON" not in text


def test_wave3_pda_readiness_documents_l7_and_usability_templates():
    """W6.D runbook 必须提供可直接填写的 L7 与易用性走查模板。"""
    text = Path("docs/runbooks/wave-3-pda-readiness.md").read_text(
        encoding="utf-8",
    )

    assert "## W6.D L7 与易用性走查模板" in text
    assert "归档为 `WAVE_3_PDA_L7_RUN_REF`" in text
    assert "归档为 `WAVE_3_PDA_USABILITY_REVIEW_REF`" in text
    assert "不设本地性能阈值" in text
    assert "执行人" in text
    assert "设备型号" in text
    assert "网络条件" in text
    assert "条码样本批次" in text
    assert "M2 scan" in text
    assert "M3 scan" in text
    assert "offline replay" in text
    assert "Idempotency-Key replay" in text
    assert "`audit_event` resource ID" in text
    assert "设备握持" in text
    assert "扫码键触达" in text
    assert "扫码反馈" in text
    assert "离线提示" in text
    assert "错误提示" in text
    assert "恢复网络确认" in text
    assert "走查结论" in text
