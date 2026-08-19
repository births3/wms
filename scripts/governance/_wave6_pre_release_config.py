"""Configuration and derived constants for the Wave 6 pre-release report."""
from pathlib import Path

from check_wave6_evidence_preflight import GATES as PREFLIGHT_GATES
from check_wave6_evidence_preflight import REQUIRED_EXECUTION_FILES
from check_wave6_evidence_preflight import WAVE6_CLOSEOUT_JUST_ENTRIES
from check_wave6_evidence_preflight import gate_commands_by_phase as gate_commands_by_phase
from check_wave6_evidence_preflight import gate_execution_file_map as gate_execution_file_map
from check_wave6_evidence_preflight import gate_just_entries as gate_just_entries
from check_wave6_evidence_preflight import validation_commands as preflight_validation_commands

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

PROVED_BY_STATIC_FILES = "PROVED_BY_STATIC_FILES"
PROVED_BY_RUNTIME_EVIDENCE = "PROVED_BY_RUNTIME_EVIDENCE"
MISSING_OR_NEEDS_EXTERNAL_STATE = "MISSING_OR_NEEDS_EXTERNAL_STATE"
NEEDS_VALIDATOR = "NEEDS_VALIDATOR"

WAVE6_TOOLING_DOCS = [
    "docs/runbooks/wave-1-runtime-evidence.md",
    "docs/runbooks/wave-2-runtime-evidence.md",
    "docs/runbooks/wave-3-pda-readiness.md",
    "docs/runbooks/wave-4-external-dependencies.md",
    "docs/runbooks/wave-5-hardware-evidence.md",
    "docs/runbooks/wave-5-tms-evidence.md",
    "docs/runbooks/wave-6-deploy-evidence.md",
    "docs/runbooks/wave-6-evidence-preflight.md",
    "docs/runbooks/wave-6-closeout.md",
]

WAVE6_TOOLING_FILES = [
    *WAVE6_TOOLING_DOCS,
    "scripts/governance/check_wave6_evidence_preflight.py",
    *REQUIRED_EXECUTION_FILES,
]

def wave6_just_entries() -> list[str]:
    return list(dict.fromkeys(
        [
            *(entry for gate in PREFLIGHT_GATES for entry in gate.just_entries),
            "wave-6-evidence-preflight",
            *WAVE6_CLOSEOUT_JUST_ENTRIES,
        ],
    ))


WAVE6_JUST_ENTRIES = wave6_just_entries()

def derive_wave6_gate_ids(gates=PREFLIGHT_GATES) -> list[str]:
    return [gate.gate_id for gate in gates]


def derive_wave6_evidence_files(gates=PREFLIGHT_GATES) -> list[str]:
    return [gate.evidence_file for gate in gates]


WAVE6_GATE_IDS = derive_wave6_gate_ids()
WAVE6_EVIDENCE_FILES = derive_wave6_evidence_files()
WAVE6_STARTUP_DOC_REQUIREMENTS = {
    "TODO.md": ("当前 Wave：Wave 6",),
    "ROADMAP.md": ("Wave 6：预发布证据与外部依赖收口",),
    "docs/architecture-dependencies.md": ("Wave 6：预发布证据与外部依赖收口",),
    "docs/adr/0035-wave-6-pre-release-evidence-closeout.md": ("ADR-0035",),
}

def derive_wave6_validation_commands() -> list[str]:
    return list(dict.fromkeys([
        *preflight_validation_commands(),
        "just wave-6-evidence-preflight",
        "just wave-6-evidence-check",
        "just wave-6-status",
        "just gov-t1",
        "just task-check",
        "git diff --check",
    ]))


WAVE6_VALIDATION_COMMANDS = derive_wave6_validation_commands()
WAVE6_PREFLIGHT_COMMAND = ("python3", "scripts/governance/check_wave6_evidence_preflight.py")

WAVE6_RETRO_FILE = "docs/retros/wave-6-retro.md"
WAVE6_RETRO_ITEM_ID = "W6-retro"
WAVE6_FORBIDDEN_EVIDENCE_BOUNDARY_STATEMENT = (
    "没有使用 local/mock/fake/stub/example/prod/production"
)
SCHEMA_VERSION = 1
REPORT_MODES = ("evidence-only", "complete")
REPORT_COMMAND = "python3 scripts/governance/report_wave6_pre_release.py --strict --json"
COMMANDS_ONLY_COMMAND = "just wave-6-missing-evidence-commands"
COMMANDS_ONLY_BOUNDARY_LINE = (
    "Wave 6 missing evidence commands: 只读命令清单；"
    "不会写入 runtime evidence，不能关闭 evidence gate"
)
COMMANDS_ONLY_STRICT_EXIT_LINE = (
    "Wave 6 missing evidence commands: 缺失 evidence 时 --strict 返回非零；"
    "这是阻塞信号，不代表命令写入或关闭 gate"
)
COMMANDS_ONLY_NONE_LINE = (
    "Wave 6 missing evidence commands: none；只读命令清单；"
    "不会写入 runtime evidence，不能关闭 evidence gate"
)
COMMANDS_ONLY_NONE_COMPLETE_MODE_LINE = (
    "Wave 6 missing evidence commands: none 只表示没有缺失 evidence gate 的采集命令；"
    "complete-check 仍可能因非 evidence blocker 返回非零"
)
W6B_ROLLBACK_DEPLOYMENT_CHOICE_LINE = (
    "choice: W6.B rollback 按实际部署形态二选一：k8s 或 docker-compose"
)
W6B_ROLLBACK_DEPLOYMENT_CHOICE_LABEL = (
    "W6.B rollback 按实际部署形态二选一：k8s 或 docker-compose"
)
W6B_ROLLBACK_DEPLOYMENT_OPTIONS = ("k8s", "docker-compose")
WAVE6_EXTERNAL_PREREQUISITES = {
    "W6.A": (
        "dev PostgreSQL",
        "最新 migration",
        "60M audit_event 基线",
        "wrk",
        "7 天 seal cron 0 失败",
    ),
    "W6.B": (
        "dev/staging k8s 或 docker-compose",
        "真实 smoke gate 或 Prometheus rollback 信号",
        "上一稳定版本",
    ),
    "W6.C": (
        "dev/staging wms-api",
        "H1 鉴权",
        "配置中心",
        "W1 文件版 flag 快照",
        "审计链路",
    ),
    "W6.D": (
        "真 PDA",
        "实体扫码键",
        "dev/staging M2/M3 API",
        "离线 replay 条件",
        "幂等 replay 条件",
        "L7 执行环境",
        "人工易用性走查人",
    ),
    "W6.E": (
        "码上放心账号 / 租户",
        "正式接口文档",
        "鉴权方式",
        "错误码",
        "频率限制",
        "真实测试环境",
    ),
    "W6.F": (
        "电子秤",
        "蓝牙打印机",
        "面单打印机",
        "校准记录",
        "dev/staging 包装站工位",
    ),
    "W6.G": (
        "TMS dev/staging endpoint",
        "回调鉴权",
        "调度结果格式",
        "失败重试条件",
        "Vault 凭证引用",
    ),
    "W6.H": (
        "staging 发布环境",
        "release plan",
        "构建产物",
        "灰度配置",
        "smoke gate",
        "dashboard",
        "回滚链路",
        "双人审批",
    ),
}
WAVE6_MINIMUM_EVIDENCE_REFS = {
    "W6.A": ("wrk 原始日志", "seal cron 日志", "DB readiness 输出"),
    "W6.B": ("rollback 日志", "smoke 或 Prometheus 触发日志"),
    "W6.C": ("smoke 日志", "reconcile 日志", "旧文件归档引用"),
    "W6.D": (
        "PDA 资产引用",
        "扫码日志",
        "离线 replay 日志",
        "idempotency replay 日志",
        "audit_event 查询",
        "L7 执行记录",
        "走查记录",
    ),
    "W6.E": ("文档归档", "Vault 凭证引用", "成功回执", "失败重试日志", "audit_event 查询"),
    "W6.F": ("设备资产引用", "校准记录", "称重日志", "打印产物", "audit_event 查询"),
    "W6.G": ("推送日志", "回调日志", "失败重试日志", "audit_event 查询"),
    "W6.H": (
        "发布计划",
        "artifact",
        "灰度配置",
        "smoke",
        "dashboard",
        "rollback",
        "审批",
        "audit_event 查询",
    ),
}

WAVE6_EXPORT_TEMPLATE_COMMANDS = {
    "W6.D": (
        "just wave-3-pda-preaudit-kit --json",
        "just wave-3-pda-materials-checklist --json",
        "just wave-3-pda-field-work-request",
        "just wave-3-pda-field-execution-summary --json",
        "just wave-3-pda-field-precheck-summary --from-env --json",
        "just wave-3-pda-field-owner-gap-actions --json",
        "just wave-3-pda-field-handoff-bundle --json",
        "just wave-3-pda-evidence-package-template",
        "just wave-3-pda-service-precheck --from-env --json",
        "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
        "just wave-3-pda-runtime-evidence-record --export-template",
        "just wave-3-pda-intake-template --json",
        "just wave-3-pda-intake-check --json",
    ),
    "W6.E": ("just wave-4-external-dependencies-record --export-template",),
    "W6.F": ("just wave-5-hardware-materials --export-template",),
    "W6.G": ("just wave-5-tms-materials --export-template",),
}

WAVE6_RECORD_CHECK_ONLY_COMMANDS = {
    "W6.D": (
        "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
        "just wave-3-pda-intake-check --json",
    ),
    "W6.E": ("just wave-4-external-dependencies-record --from-env --check-only --json",),
    "W6.F": ("just wave-5-hardware-evidence-record --from-env --check-only --json",),
    "W6.G": ("just wave-5-tms-evidence-record --from-env --check-only --json",),
    "W6.H": (
        "just wave-6-deploy-audit --from-env --check-only",
        "just wave-6-deploy-evidence-record --from-env --check-only --json",
    ),
}
WAVE6_PHASE_COMMAND_REPLACEMENTS = {
    "W6.D": {
        "readiness": {
            "just wave-3-pda-runtime-readiness": (
                "just wave-3-pda-runtime-readiness --from-env --json"
            ),
        },
        "record": {
            "just wave-3-pda-runtime-evidence-record": (
                "just wave-3-pda-runtime-evidence-record --from-env --json"
            ),
            "just wave-3-pda-intake-record": "just wave-3-pda-intake-record --json",
        },
    },
    "W6.E": {
        "readiness": {
            "just wave-4-external-dependencies-readiness": (
                "just wave-4-external-dependencies-readiness --from-env --json"
            ),
        },
        "record": {
            "just wave-4-external-dependencies-record": (
                "just wave-4-external-dependencies-record --from-env --json"
            ),
        },
    },
    "W6.F": {
        "readiness": {
            "just wave-5-hardware-materials": (
                "just wave-5-hardware-materials --from-env --json"
            ),
            "just wave-5-hardware-readiness": (
                "just wave-5-hardware-readiness --from-env --json"
            ),
        },
        "record": {
            "just wave-5-hardware-evidence-record": (
                "just wave-5-hardware-evidence-record --from-env --json"
            ),
        },
    },
    "W6.G": {
        "readiness": {
            "just wave-5-tms-materials": (
                "just wave-5-tms-materials --from-env --json"
            ),
            "just wave-5-tms-readiness": (
                "just wave-5-tms-readiness --from-env --json"
            ),
        },
        "record": {
            "just wave-5-tms-evidence-record": (
                "just wave-5-tms-evidence-record --from-env --json"
            ),
        },
    },
    "W6.H": {
        "readiness": {
            "just wave-6-deploy-materials --json": (
                "just wave-6-deploy-materials --from-env --json"
            ),
            "just wave-6-deploy-readiness": (
                "just wave-6-deploy-readiness --from-env --json"
            ),
        },
        "record": {
            "just wave-6-deploy-audit": "just wave-6-deploy-audit --from-env",
            "just wave-6-deploy-evidence-record": (
                "just wave-6-deploy-evidence-record --from-env --json"
            ),
        },
    },
}
