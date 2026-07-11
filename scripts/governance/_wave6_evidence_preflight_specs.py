"""Static specifications for Wave 6 evidence preflight."""
from dataclasses import dataclass, field
from pathlib import Path
import re

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

PREFLIGHT_DOC = "docs/runbooks/wave-6-evidence-preflight.md"
CLOSEOUT_DOC = "docs/runbooks/wave-6-closeout.md"
TODO_DOC = "TODO.md"
JUSTFILE = "justfile"
REQUIRED_BOUNDARY_TERMS = ("environment", "dev", "staging")
FORBIDDEN_BOUNDARY_TERMS = ("local", "prod", "production", "mock", "fake", "stub", "example")
TEMPLATE_PLACEHOLDER_TERMS = ("YYYY", "<...>", "TODO", "TBD", "待填", "待确认")
PLACEHOLDER_VALUE_TOKENS = ("yyyy", "todo", "tbd", "待填", "待确认")
REFERENCE_LINE_HINTS = (
    "_ref",
    "-ref",
    "ref\"",
    "ref'",
    "_url",
    "-url",
    "url\"",
    "url'",
    "_file",
    "-file",
    "_path",
    "-path",
    "_output",
    "-output",
    "s3://",
    "http://",
    "https://",
    "asset://",
    ".json",
    ".log",
    ".pdf",
    ".md",
)
HARDCODED_RECORD_REF_VALUE_HINTS = (
    "ci/",
    "docs/",
    "s3://",
    "asset://",
    "registry://",
    "grafana/",
    "gitlab/",
    "ticket://",
    "vault://",
    "postgres://",
    "http://",
    "https://",
    "/tmp/",
    "/srv/",
)
RECORD_REF_ARG_RE = re.compile(
    r"(?P<flag>--[a-z0-9-]*(?:-ref|-url|-file|-path|-output))\s+"
    r"(?:(?P<quote>['\"])(?P<quoted_value>[^'\"]+)(?P=quote)|(?P<bare_value>\S+))"
)
EXPORT_REF_RE = re.compile(
    r"\bexport\s+(?P<name>[A-Z0-9_]*(?:REF|URL|FILE|PATH|OUTPUT))\s*=\s*"
    r"(?:(?P<quote>['\"])(?P<quoted_value>[^'\"]+)(?P=quote)|(?P<bare_value>\S+))"
)
JUST_ENTRY_EXECUTION_FILE_OVERRIDES = {
    "wave-1-runtime-prereq-h2": "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "wave-1-h2-runtime-readiness": "scripts/governance/check_wave1_h2_runtime_readiness.py",
    "wave-1-h2-runtime-evidence": "scripts/governance/collect_wave1_h2_runtime_evidence.py",
    "wave-1-runtime-prereq-rollback-k8s": "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "wave-1-rollback-runtime-readiness-k8s": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-1-rollback-runtime-evidence-k8s": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-1-runtime-prereq-rollback-compose": "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "wave-1-rollback-runtime-readiness-compose": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-1-rollback-runtime-evidence-compose": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-2-runtime-evidence-validate": "scripts/governance/report_wave2_completion.py",
    "wave-2-runtime-evidence-readiness": "scripts/governance/collect_wave2_runtime_evidence.py",
    "wave-2-runtime-evidence-smoke": "scripts/governance/collect_wave2_runtime_evidence.py",
    "wave-2-runtime-evidence-collect": "scripts/governance/collect_wave2_runtime_evidence.py",
    "wave-3-pda-preaudit-kit": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-materials-checklist": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-work-request": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-execution-summary": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-precheck-summary": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-owner-gap-actions": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-handoff-bundle": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-service-precheck": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-trace-code-openapi-precheck": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-runtime-readiness": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-evidence-package-template": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-3-pda-intake-template": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-3-pda-intake-check": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-3-pda-intake-record": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-4-external-dependencies-readiness": "scripts/governance/check_wave4_external_dependencies_readiness.py",
    "wave-5-hardware-materials": "scripts/governance/record_wave5_hardware_evidence.py",
    "wave-5-hardware-readiness": "scripts/governance/record_wave5_hardware_evidence.py",
    "wave-5-tms-materials": "scripts/governance/record_wave5_tms_evidence.py",
    "wave-5-tms-readiness": "scripts/governance/record_wave5_tms_evidence.py",
    "wave-6-deploy-materials": "scripts/governance/report_wave6_deploy_materials.py",
    "wave-6-deploy-readiness": "scripts/governance/check_wave6_deploy_readiness.py",
}


@dataclass(frozen=True)
class GateSpec:
    gate_id: str
    title: str
    runbook: str
    evidence_file: str
    just_entries: tuple[str, ...]
    required_terms: tuple[str, ...] = REQUIRED_BOUNDARY_TERMS
    forbidden_boundary_terms: tuple[str, ...] = FORBIDDEN_BOUNDARY_TERMS
    template_placeholder_terms: tuple[str, ...] = TEMPLATE_PLACEHOLDER_TERMS


@dataclass
class GateResult:
    gate_id: str
    title: str
    ok: bool
    errors: list[str] = field(default_factory=list)


GATES: tuple[GateSpec, ...] = (
    GateSpec(
        "W6.A",
        "Wave 1 H2 runtime evidence",
        "docs/runbooks/wave-1-runtime-evidence.md",
        "docs/retros/wave-1-h2-runtime-evidence.json",
        (
            "wave-1-runtime-prereq-h2",
            "wave-1-h2-runtime-readiness",
            "wave-1-h2-runtime-evidence",
            "wave-1-runtime-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.B",
        "Wave 1 W1.D rollback evidence",
        "docs/runbooks/wave-1-runtime-evidence.md",
        "docs/retros/wave-1-runtime-evidence.json",
        (
            "wave-1-runtime-prereq-rollback-k8s",
            "wave-1-rollback-runtime-readiness-k8s",
            "wave-1-rollback-runtime-evidence-k8s",
            "wave-1-runtime-prereq-rollback-compose",
            "wave-1-rollback-runtime-readiness-compose",
            "wave-1-rollback-runtime-evidence-compose",
            "wave-1-runtime-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.C",
        "Wave 2 config-center Feature Flag evidence",
        "docs/runbooks/wave-2-runtime-evidence.md",
        "docs/retros/wave-2-runtime-evidence.json",
        (
            "wave-2-runtime-evidence-readiness",
            "wave-2-runtime-evidence-smoke",
            "wave-2-runtime-evidence-record",
            "wave-2-runtime-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.D",
        "Wave 3 real PDA / L7 evidence",
        "docs/runbooks/wave-3-pda-readiness.md",
        "docs/retros/wave-3-pda-runtime-evidence.json",
        (
            "wave-3-pda-preaudit-kit",
            "wave-3-pda-materials-checklist",
            "wave-3-pda-field-work-request",
            "wave-3-pda-field-execution-summary",
            "wave-3-pda-field-precheck-summary",
            "wave-3-pda-field-owner-gap-actions",
            "wave-3-pda-field-handoff-bundle",
            "wave-3-pda-evidence-package-template",
            "wave-3-pda-intake-template",
            "wave-3-pda-intake-check",
            "wave-3-pda-intake-record",
            "wave-3-pda-service-precheck",
            "wave-3-pda-trace-code-openapi-precheck",
            "wave-3-pda-runtime-readiness",
            "wave-3-pda-runtime-evidence-record",
            "wave-3-pda-runtime-evidence-validate",
        ),
        (
            *REQUIRED_BOUNDARY_TERMS,
            "真 PDA",
            "实体扫码键",
            "蓝牙打印",
            "Wave 5",
            "docs/retros/wave-5-hardware-evidence.json",
        ),
    ),
    GateSpec(
        "W6.E",
        "Wave 4 M-TC external evidence",
        "docs/runbooks/wave-4-external-dependencies.md",
        "docs/retros/wave-4-external-dependencies.json",
        (
            "wave-4-external-dependencies-readiness",
            "wave-4-external-dependencies-record",
            "wave-4-external-dependencies-validate",
        ),
    ),
    GateSpec(
        "W6.F",
        "Wave 5 M-PK hardware evidence",
        "docs/runbooks/wave-5-hardware-evidence.md",
        "docs/retros/wave-5-hardware-evidence.json",
        (
            "wave-5-hardware-materials",
            "wave-5-hardware-readiness",
            "wave-5-hardware-evidence-record",
            "wave-5-hardware-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.G",
        "Wave 5 M10 TMS+ evidence",
        "docs/runbooks/wave-5-tms-evidence.md",
        "docs/retros/wave-5-tms-evidence.json",
        (
            "wave-5-tms-materials",
            "wave-5-tms-readiness",
            "wave-5-tms-evidence-record",
            "wave-5-tms-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.H",
        "Wave 6 gray release evidence",
        "docs/runbooks/wave-6-deploy-evidence.md",
        "docs/retros/wave-6-deploy-evidence.json",
        (
            "wave-6-deploy-materials",
            "wave-6-deploy-readiness",
            "wave-6-deploy-audit",
            "wave-6-deploy-evidence-record",
            "wave-6-deploy-evidence-validate",
        ),
    ),
)

REQUIRED_TOP_LEVEL_FILES = (
    PREFLIGHT_DOC,
    CLOSEOUT_DOC,
    TODO_DOC,
    JUSTFILE,
    "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
)
WAVE6_EVIDENCE_BOUNDARY_DOCS = (
    TODO_DOC,
    "ROADMAP.md",
    "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
    "docs/retros/wave-1-retro.md",
)
WAVE6_SCOPE_GATE_DOCS = (
    TODO_DOC,
    "ROADMAP.md",
    "docs/architecture-dependencies.md",
    "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
)
WAVE6_CLOSEOUT_JUST_ENTRIES = (
    "wave-6-status",
    "wave-6-evidence-check",
    "wave-6-missing-evidence-commands",
    "wave-6-complete-check",
)
WAVE6_GATE_COMMAND_OVERRIDES = {
    "W6.D": {
        "readiness": [
            "just wave-3-pda-preaudit-kit --json",
            "just wave-3-pda-materials-checklist --json",
            "just wave-3-pda-field-work-request",
            "just wave-3-pda-field-execution-summary --json",
            "just wave-3-pda-field-precheck-summary --from-env --json",
            "just wave-3-pda-field-owner-gap-actions --json",
            "just wave-3-pda-field-handoff-bundle --json",
            "just wave-3-pda-evidence-package-template",
            "just wave-3-pda-intake-template --json",
            "just wave-3-pda-intake-check --json",
            "just wave-3-pda-service-precheck --from-env --json",
            "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
            "just wave-3-pda-runtime-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
            "just wave-3-pda-intake-check --json",
        ],
        "record": [
            "just wave-3-pda-intake-record --json",
            "just wave-3-pda-runtime-evidence-record",
        ],
    },
    "W6.E": {
        "readiness": [
            "just wave-4-external-dependencies-record --export-template",
            "just wave-4-external-dependencies-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-4-external-dependencies-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-4-external-dependencies-record --from-env --json",
        ],
    },
    "W6.F": {
        "readiness": [
            "just wave-5-hardware-materials --export-template",
            "just wave-5-hardware-materials --from-env --json",
            "just wave-5-hardware-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-5-hardware-evidence-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-5-hardware-evidence-record --from-env --json",
        ],
    },
    "W6.G": {
        "readiness": [
            "just wave-5-tms-materials --export-template",
            "just wave-5-tms-materials --from-env --json",
            "just wave-5-tms-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-5-tms-evidence-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-5-tms-evidence-record --from-env --json",
        ],
    },
    "W6.H": {
        "readiness": [
            "just wave-6-deploy-materials --export-template",
            "just wave-6-deploy-materials --from-env --json",
            "just wave-6-deploy-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-6-deploy-audit --from-env --check-only",
            "just wave-6-deploy-evidence-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-6-deploy-audit --from-env",
            "just wave-6-deploy-evidence-record --from-env --json",
        ],
    },
}
WAVE6_H_CLOSEOUT_COMMAND_ORDER = (
    "just wave-6-deploy-materials --export-template",
    "just wave-6-deploy-materials --from-env --json",
    "just wave-6-deploy-audit --from-env --check-only",
    "just wave-6-deploy-audit --from-env",
    "just wave-6-deploy-readiness --from-env --json",
    "just wave-6-deploy-evidence-record --from-env --check-only --json",
    "just wave-6-deploy-evidence-record --from-env --json",
    "just wave-6-deploy-evidence-validate",
)
WAVE6_CLOSEOUT_JUST_ENTRY_COMMANDS = {
    "wave-6-status": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
    ),
    "wave-6-evidence-check": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
        "--strict",
        "--evidence-only",
    ),
    "wave-6-missing-evidence-commands": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
        "--commands-only",
        "--strict",
        "--evidence-only",
    ),
    "wave-6-complete-check": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
        "--strict",
    ),
}
WAVE6_CLOSEOUT_REPORT_JSON_FIELDS = (
    "schema_version",
    "available_modes",
    "writes_runtime_evidence",
    "closes_gate",
    "report_command",
    "evidence_only_command",
    "commands_only_command",
    "evidence_gate_ids",
    "evidence_gate_evidence_files",
    "evidence_gate_just_entries",
    "evidence_gate_execution_files",
    "required_top_level_files",
    "required_runbooks",
    "required_execution_files",
    "validation_commands",
    "closeout_just_entries",
    "evidence_gate_item_ids",
    "non_evidence_item_ids",
    "blocking_count",
    "ignored_count",
    "evidence_blocking_count",
    "evidence_ignored_count",
    "evidence_blocking_item_ids",
    "non_evidence_blocking_item_ids",
    "evidence_ignored_item_ids",
    "non_evidence_ignored_item_ids",
    "blocking_details",
    "ignored_details",
    "missing_evidence_count",
    "missing_evidence_item_ids",
    "missing_evidence_files",
    "missing_evidence_details",
    "readiness_commands",
    "record_check_only_commands",
    "record_commands",
    "validate_commands",
    "deployment_choice_required",
    "deployment_choice_label",
    "deployment_choice_options",
    "deployment_path_commands",
)
WAVE6_CLOSEOUT_REPORT_JSON_SECTION_MARKER = (
    "report_wave6_pre_release.py --strict --evidence-only --json"
)

REQUIRED_EXECUTION_FILES = (
    "deploy/scripts/wave1_auto_rollback_probe.sh",
    "deploy/scripts/wave1_rollback.sh",
    "scripts/governance/report_wave6_pre_release.py",
    "scripts/governance/validate_wave1_runtime_evidence.py",
    "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "scripts/governance/check_wave1_h2_runtime_readiness.py",
    "scripts/governance/collect_wave1_h2_runtime_evidence.py",
    "scripts/governance/report_wave2_completion.py",
    "scripts/governance/collect_wave2_runtime_evidence.py",
    "scripts/governance/record_wave2_runtime_evidence.py",
    "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "scripts/governance/validate_wave3_pda_runtime_evidence.py",
    "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "scripts/governance/check_wave4_external_dependencies_readiness.py",
    "scripts/governance/validate_wave4_external_dependencies.py",
    "scripts/governance/record_wave4_external_dependencies.py",
    "scripts/governance/validate_wave5_hardware_evidence.py",
    "scripts/governance/record_wave5_hardware_evidence.py",
    "scripts/governance/validate_wave5_tms_evidence.py",
    "scripts/governance/record_wave5_tms_evidence.py",
    "scripts/governance/report_wave6_deploy_materials.py",
    "scripts/governance/check_wave6_deploy_readiness.py",
    "scripts/governance/validate_wave6_deploy_evidence.py",
    "scripts/governance/record_wave6_deploy_evidence.py",
)

OVERWRITE_GUARD_REQUIRED_MARKERS = (
    "--force",
    "already exists",
    "pass --force to overwrite",
)
SCHEMA_VERSION = 1
PREFLIGHT_MODE = "static-preflight"
PREFLIGHT_COMMAND = "python3 scripts/governance/check_wave6_evidence_preflight.py --json"
