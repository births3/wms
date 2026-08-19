"""Wave 6 pre-release report JSON 顶层合同治理测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import (
    assert_wave6_missing_evidence_files_are_consistent,
    assert_wave6_report_details_are_consistent,
    assert_wave6_report_json_groups_are_consistent,
    assert_wave6_report_tooling_metadata_is_consistent,
    patch_wave6_report_io,
    wave6_missing_file_validator,
)


def test_wave6_evidence_only_json_contract_marks_blocking_and_ignored_gaps(
    monkeypatch,
    capsys,
):
    """Wave 6 evidence-only JSON 必须明确区分真实证据缺口与暂忽略 retro。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave3_pda_runtime_evidence.py": (
                "docs/retros/wave-3-pda-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["mode"] == "evidence-only"
    assert payload["ok"] is False
    assert payload["evidence_gate_count"] == len(report.WAVE6_GATE_IDS)
    assert payload["evidence_gate_ids"] == report.WAVE6_GATE_IDS
    assert payload["evidence_gate_evidence_files"] == report.WAVE6_EVIDENCE_FILES
    assert_wave6_report_tooling_metadata_is_consistent(payload, report)
    assert [
        item["item_id"].split("-", 1)[0]
        for item in payload["evidence_gate_items"]
    ] == report.WAVE6_GATE_IDS
    assert [item["item_id"] for item in payload["non_evidence_items"]] == [
        "W6-startup",
        "W6-tooling",
        "W6-wave5-closeout",
        "W6-retro",
    ]
    assert payload["evidence_blocking_count"] == 1
    assert payload["evidence_ignored_count"] == 0
    assert payload["blocking_count"] == 1
    assert payload["ignored_count"] == 1
    assert [item["item_id"] for item in payload["evidence_blocking_gaps"]] == [
        "W6.D-wave3-pda-l7",
    ]
    assert payload["evidence_ignored_gaps"] == []
    assert payload["non_evidence_blocking_gaps"] == []
    assert [item["item_id"] for item in payload["non_evidence_ignored_gaps"]] == [
        "W6-retro",
    ]
    assert [item["item_id"] for item in payload["blocking_gaps"]] == [
        "W6.D-wave3-pda-l7",
    ]
    assert [item["item_id"] for item in payload["ignored_gaps"]] == ["W6-retro"]
    assert payload["missing_evidence_item_ids"] == ["W6.D-wave3-pda-l7"]
    assert payload["missing_evidence_files"] == [
        "docs/retros/wave-3-pda-runtime-evidence.json",
    ]
    assert payload["missing_evidence_details"] == [
        {
            "gate_id": "W6.D",
            "item_id": "W6.D-wave3-pda-l7",
            "evidence_file": "docs/retros/wave-3-pda-runtime-evidence.json",
            "status": report.MISSING_OR_NEEDS_EXTERNAL_STATE,
            "requirement": "Wave 3 真 PDA + L7 性能 / 易用性 runtime evidence",
            "gaps": ["missing file: docs/retros/wave-3-pda-runtime-evidence.json"],
            "external_prerequisites": [
                "真 PDA",
                "实体扫码键",
                "dev/staging M2/M3 API",
                "离线 replay 条件",
                "幂等 replay 条件",
                "L7 执行环境",
                "人工易用性走查人",
            ],
            "minimum_evidence_refs": [
                "PDA 资产引用",
                "扫码日志",
                "离线 replay 日志",
                "idempotency replay 日志",
                "audit_event 查询",
                "L7 执行记录",
                "走查记录",
            ],
            "readiness_commands": [
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
                "just wave-3-pda-runtime-readiness --from-env --json",
            ],
            "record_check_only_commands": [
                "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
                "just wave-3-pda-intake-check --json",
            ],
            "record_commands": [
                "just wave-3-pda-intake-record --json",
                "just wave-3-pda-runtime-evidence-record --from-env --json",
            ],
            "validate_commands": ["just wave-3-pda-runtime-evidence-validate"],
        }
    ]
    assert_wave6_report_json_groups_are_consistent(payload)
    assert_wave6_missing_evidence_files_are_consistent(payload)
    assert_wave6_report_details_are_consistent(payload)


def test_wave6_json_missing_evidence_details_marks_w1d_as_deployment_choice(
    monkeypatch,
    capsys,
):
    """W6.B JSON 明细必须给自动化消费者表达 rollback 部署路径二选一。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["missing_evidence_item_ids"] == [
        "W6.B-wave1-rollback-runtime",
    ]
    detail = payload["missing_evidence_details"][0]

    assert detail["deployment_choice_required"] is True
    assert detail["deployment_choice_label"] == (
        "W6.B rollback 按实际部署形态二选一：k8s 或 docker-compose"
    )
    assert detail["deployment_choice_options"] == ["k8s", "docker-compose"]
    assert detail["deployment_path_commands"] == [
        {
            "path": "k8s",
            "readiness_commands": [
                "just wave-1-runtime-prereq-rollback-k8s",
                "just wave-1-rollback-runtime-readiness-k8s",
            ],
            "record_commands": ["just wave-1-rollback-runtime-evidence-k8s"],
        },
        {
            "path": "docker-compose",
            "readiness_commands": [
                "just wave-1-runtime-prereq-rollback-compose",
                "just wave-1-rollback-runtime-readiness-compose",
            ],
            "record_commands": ["just wave-1-rollback-runtime-evidence-compose"],
        },
    ]


def test_wave6_json_w1d_deployment_paths_cover_all_readiness_and_record_commands(
    monkeypatch,
    capsys,
):
    """W6.B 路径分组不能静默丢失任何 readiness / record 命令。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    detail = payload["missing_evidence_details"][0]

    grouped_readiness = [
        command
        for group in detail["deployment_path_commands"]
        for command in group["readiness_commands"]
    ]
    grouped_record = [
        command
        for group in detail["deployment_path_commands"]
        for command in group["record_commands"]
    ]

    assert sorted(grouped_readiness) == sorted(detail["readiness_commands"])
    assert sorted(grouped_record) == sorted(detail["record_commands"])


def test_wave6_json_w6h_records_deploy_audit_before_evidence_record(
    monkeypatch,
    capsys,
):
    """W6.H JSON 明细必须把 check-only 和 deploy audit 列在 evidence record 之前。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave6_deploy_evidence.py": (
                "docs/retros/wave-6-deploy-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    detail = payload["missing_evidence_details"][0]

    assert detail["item_id"] == "W6.H-gray-release"
    assert detail["readiness_commands"] == [
        "just wave-6-deploy-materials --export-template",
        "just wave-6-deploy-materials --from-env --json",
        "just wave-6-deploy-readiness --from-env --json",
    ]
    assert detail["record_check_only_commands"] == [
        "just wave-6-deploy-audit --from-env --check-only",
        "just wave-6-deploy-evidence-record --from-env --check-only --json",
    ]
    assert detail["record_commands"] == [
        "just wave-6-deploy-audit --from-env",
        "just wave-6-deploy-evidence-record --from-env --json",
    ]
    assert detail["validate_commands"] == ["just wave-6-deploy-evidence-validate"]


def test_wave6_json_lists_record_check_only_for_recorder_backed_gates(
    monkeypatch,
    capsys,
):
    """W6.D/E/F/G/H JSON 明细必须列出正式 record 前的 check-only 预检。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave3_pda_runtime_evidence.py": (
                "docs/retros/wave-3-pda-runtime-evidence.json"
            ),
            "validate_wave4_external_dependencies.py": (
                "docs/retros/wave-4-external-dependencies.json"
            ),
            "validate_wave5_hardware_evidence.py": (
                "docs/retros/wave-5-hardware-evidence.json"
            ),
            "validate_wave5_tms_evidence.py": (
                "docs/retros/wave-5-tms-evidence.json"
            ),
            "validate_wave6_deploy_evidence.py": (
                "docs/retros/wave-6-deploy-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    details = {
        detail["gate_id"]: detail
        for detail in payload["missing_evidence_details"]
    }

    assert details["W6.D"]["record_check_only_commands"] == [
        "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
        "just wave-3-pda-intake-check --json",
    ]
    assert details["W6.E"]["record_check_only_commands"] == [
        "just wave-4-external-dependencies-record --from-env --check-only --json",
    ]
    assert details["W6.F"]["record_check_only_commands"] == [
        "just wave-5-hardware-evidence-record --from-env --check-only --json",
    ]
    assert details["W6.G"]["record_check_only_commands"] == [
        "just wave-5-tms-evidence-record --from-env --check-only --json",
    ]
    assert details["W6.H"]["record_check_only_commands"] == [
        "just wave-6-deploy-audit --from-env --check-only",
        "just wave-6-deploy-evidence-record --from-env --check-only --json",
    ]


def test_wave6_json_lists_export_template_before_readiness_for_material_gates(
    monkeypatch,
    capsys,
):
    """W6.D/E/F/G JSON 明细必须先给现场材料模板入口。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave3_pda_runtime_evidence.py": (
                "docs/retros/wave-3-pda-runtime-evidence.json"
            ),
            "validate_wave4_external_dependencies.py": (
                "docs/retros/wave-4-external-dependencies.json"
            ),
            "validate_wave5_hardware_evidence.py": (
                "docs/retros/wave-5-hardware-evidence.json"
            ),
            "validate_wave5_tms_evidence.py": (
                "docs/retros/wave-5-tms-evidence.json"
            ),
        }),
    )

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    details = {
        detail["gate_id"]: detail
        for detail in payload["missing_evidence_details"]
    }

    assert details["W6.D"]["readiness_commands"][:11] == [
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
    ]
    assert details["W6.D"]["readiness_commands"][11:13] == [
        "just wave-3-pda-intake-template --json",
        "just wave-3-pda-intake-check --json",
    ]
    assert details["W6.E"]["readiness_commands"][0] == (
        "just wave-4-external-dependencies-record --export-template"
    )
    assert details["W6.F"]["readiness_commands"][0] == (
        "just wave-5-hardware-materials --export-template"
    )
    assert details["W6.F"]["readiness_commands"][1:3] == [
        "just wave-5-hardware-materials --from-env --json",
        "just wave-5-hardware-readiness --from-env --json",
    ]
    assert details["W6.G"]["readiness_commands"][0] == (
        "just wave-5-tms-materials --export-template"
    )
    assert details["W6.G"]["readiness_commands"][1:3] == [
        "just wave-5-tms-materials --from-env --json",
        "just wave-5-tms-readiness --from-env --json",
    ]
