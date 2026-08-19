"""Wave 6 pre-release report complete JSON 合同治理测试。"""
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
    wave6_all_missing_evidence_validator,
)


def test_wave6_complete_json_contract_separates_retro_from_evidence_gates(
    monkeypatch,
    capsys,
):
    """Wave 6 complete JSON 必须区分最终 retro 缺口和 8 个真实 evidence gate。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(monkeypatch, report)

    assert report.main(["--strict", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["mode"] == "complete"
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
    assert payload["evidence_blocking_count"] == 0
    assert payload["evidence_ignored_count"] == 0
    assert payload["blocking_count"] == 1
    assert payload["ignored_count"] == 0
    assert payload["evidence_blocking_gaps"] == []
    assert payload["evidence_ignored_gaps"] == []
    assert [item["item_id"] for item in payload["non_evidence_blocking_gaps"]] == [
        "W6-retro",
    ]
    assert payload["non_evidence_ignored_gaps"] == []
    assert [item["item_id"] for item in payload["blocking_gaps"]] == ["W6-retro"]
    assert payload["missing_evidence_count"] == 0
    assert payload["missing_evidence_item_ids"] == []
    assert payload["missing_evidence_files"] == []
    assert payload["missing_evidence_details"] == []
    assert_wave6_report_json_groups_are_consistent(payload)
    assert_wave6_missing_evidence_files_are_consistent(payload)
    assert_wave6_report_details_are_consistent(payload)


def test_wave6_complete_json_contract_reports_evidence_and_retro_blockers(
    monkeypatch,
    capsys,
):
    """Wave 6 complete JSON 必须同时保留真实 evidence 缺口和 retro 缺口。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_all_missing_evidence_validator(),
    )

    assert report.main(["--strict", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["mode"] == "complete"
    assert payload["ok"] is False
    assert_wave6_report_tooling_metadata_is_consistent(payload, report)
    assert payload["evidence_blocking_count"] == len(report.WAVE6_GATE_IDS)
    assert payload["missing_evidence_count"] == len(report.WAVE6_EVIDENCE_FILES)
    assert payload["missing_evidence_files"] == report.WAVE6_EVIDENCE_FILES
    assert payload["missing_evidence_item_ids"] == payload["evidence_gate_item_ids"]
    assert [
        detail["evidence_file"]
        for detail in payload["missing_evidence_details"]
    ] == report.WAVE6_EVIDENCE_FILES
    w6b_detail = payload["missing_evidence_details"][1]
    assert w6b_detail["gate_id"] == "W6.B"
    assert w6b_detail["record_commands"] == [
        "just wave-1-rollback-runtime-evidence-k8s",
        "just wave-1-rollback-runtime-evidence-compose",
    ]
    assert w6b_detail["validate_commands"] == [
        "just wave-1-runtime-evidence-validate",
    ]
    assert payload["non_evidence_blocking_item_ids"] == ["W6-retro"]
    assert payload["ignored_count"] == 0
    assert payload["evidence_ignored_count"] == 0
    assert payload["blocking_count"] == len(report.WAVE6_GATE_IDS) + 1
    assert payload["blocking_item_ids"] == [
        *payload["evidence_gate_item_ids"],
        "W6-retro",
    ]
    assert_wave6_report_json_groups_are_consistent(payload)
    assert_wave6_missing_evidence_files_are_consistent(payload)
    assert_wave6_report_details_are_consistent(payload)
