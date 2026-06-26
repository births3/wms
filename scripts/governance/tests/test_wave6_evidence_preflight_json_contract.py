"""Wave 6 evidence preflight JSON 合同测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave6_evidence_preflight_json_declares_process_governance_category(capsys):
    """Wave 6 preflight JSON 必须声明流程治理分类，便于统一报告消费。"""
    import check_wave6_evidence_preflight as check

    assert check.main(["--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["category"] == "流程治理"


def test_wave6_evidence_preflight_json_contract_aligns_with_report_metadata(capsys):
    """Wave 6 preflight JSON 必须暴露和 report 对齐的 gate 元数据。"""
    import check_wave6_evidence_preflight as check
    import report_wave6_pre_release as report

    assert check.main(["--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    gate_specs = payload["gate_specs"]
    assert payload["script"] == "check_wave6_evidence_preflight"
    assert payload["schema_version"] == 1
    assert payload["mode"] == "static-preflight"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "commands_only_command" not in payload
    assert payload["preflight_command"] == check.PREFLIGHT_COMMAND
    assert payload["gate_count"] == len(check.GATES)
    assert payload["evidence_gate_ids"] == report.WAVE6_GATE_IDS
    assert payload["evidence_gate_evidence_files"] == report.WAVE6_EVIDENCE_FILES
    assert payload["evidence_gate_runbooks"] == [
        gate.runbook for gate in check.GATES
    ]
    assert payload["evidence_gate_just_entries"] == {
        gate.gate_id: list(gate.just_entries) for gate in check.GATES
    }
    assert payload["evidence_gate_just_entries"]["W6.D"] == [
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
    ]
    assert payload["evidence_gate_execution_files"] == {
        gate.gate_id: check.gate_execution_files(gate) for gate in check.GATES
    }
    assert payload["required_execution_files"] == list(check.REQUIRED_EXECUTION_FILES)
    assert payload["required_top_level_files"] == list(check.REQUIRED_TOP_LEVEL_FILES)
    assert payload["required_runbooks"] == list(dict.fromkeys(gate.runbook for gate in check.GATES))
    assert payload["overwrite_guard_execution_files"] == [
        path for path in check.REQUIRED_EXECUTION_FILES
        if check.is_evidence_writer_execution_file(path)
    ]
    assert payload["overwrite_guard_required_markers"] == list(
        check.OVERWRITE_GUARD_REQUIRED_MARKERS
    )
    assert payload["closeout_just_entries"] == list(check.WAVE6_CLOSEOUT_JUST_ENTRIES)
    assert payload["validation_commands"] == list(dict.fromkeys([
        f"just {entry}"
        for gate in check.GATES
        for entry in gate.just_entries
        if entry.endswith("-validate")
    ]))
    assert payload["gate_commands_by_phase"] == {
        gate_id: {
            "readiness": phases["readiness"],
            "record_check_only": phases["record_check_only"],
            "record": phases["record"],
            "validate": phases["validate"],
        }
        for gate_id, phases in check.gate_commands_by_phase().items()
    }
    assert payload["gate_commands_by_phase"]["W6.D"] == {
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
        "validate": ["just wave-3-pda-runtime-evidence-validate"],
    }
    assert payload["gate_commands_by_phase"]["W6.H"] == {
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
        "validate": ["just wave-6-deploy-evidence-validate"],
    }
    assert [gate["gate_id"] for gate in gate_specs] == report.WAVE6_GATE_IDS
    assert [gate["evidence_file"] for gate in gate_specs] == report.WAVE6_EVIDENCE_FILES
    assert [gate["execution_files"] for gate in gate_specs] == [
        check.gate_execution_files(gate) for gate in check.GATES
    ]
    assert [gate["gate_id"] for gate in payload["gates"]] == payload["evidence_gate_ids"]
    assert str(check.REPO_ROOT) not in json.dumps(payload, ensure_ascii=False)
