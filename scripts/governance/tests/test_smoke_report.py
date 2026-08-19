"""报告型治理脚本 smoke 测试。"""
import pytest

from test_smoke import _json_payload_or_fail, _run_script

REPORT_SCRIPTS = [
    "report_wave6_pre_release.py",
]


@pytest.mark.parametrize("script_name", REPORT_SCRIPTS)
def test_report_script_strict_json_contract(script_name):
    """报告型脚本必须提供可消费 JSON；strict 模式下退出码与 ok 一致。"""
    result = _run_script(
        script_name,
        "--strict",
        "--evidence-only",
        "--json",
        timeout=10,
    )
    assert result.returncode in (0, 1, 2), \
        f"invalid exit code {result.returncode}; stderr: {result.stderr}"

    payload = _json_payload_or_fail(result)

    script_stem = script_name.removesuffix(".py")
    assert payload["script"] == script_stem
    assert payload["report"] == script_stem.removeprefix("report_")
    assert payload["category"] == "流程治理"
    assert payload["mode"] == "evidence-only"
    assert isinstance(payload["ok"], bool)
    assert isinstance(payload["items"], list)
    assert isinstance(payload["evidence_gate_count"], int)
    assert payload["evidence_gate_count"] == len(payload["evidence_gate_items"])
    assert isinstance(payload["evidence_gate_ids"], list)
    assert isinstance(payload["evidence_gate_evidence_files"], list)
    assert len(payload["evidence_gate_ids"]) == payload["evidence_gate_count"]
    assert len(payload["evidence_gate_evidence_files"]) == payload["evidence_gate_count"]
    assert len(payload["items"]) == (
        len(payload["evidence_gate_items"]) + len(payload["non_evidence_items"])
    )
    item_ids = [item["item_id"] for item in payload["items"]]
    evidence_gate_item_ids = [item["item_id"] for item in payload["evidence_gate_items"]]
    non_evidence_item_ids = [item["item_id"] for item in payload["non_evidence_items"]]
    blocking_item_ids = [item["item_id"] for item in payload["blocking_gaps"]]
    ignored_item_ids = [item["item_id"] for item in payload["ignored_gaps"]]
    evidence_blocking_item_ids = [
        item["item_id"] for item in payload["evidence_blocking_gaps"]
    ]
    evidence_ignored_item_ids = [
        item["item_id"] for item in payload["evidence_ignored_gaps"]
    ]
    non_evidence_blocking_item_ids = [
        item["item_id"] for item in payload["non_evidence_blocking_gaps"]
    ]
    non_evidence_ignored_item_ids = [
        item["item_id"] for item in payload["non_evidence_ignored_gaps"]
    ]
    assert payload["evidence_gate_item_ids"] == evidence_gate_item_ids
    assert payload["non_evidence_item_ids"] == non_evidence_item_ids
    assert payload["blocking_item_ids"] == blocking_item_ids
    assert payload["ignored_item_ids"] == ignored_item_ids
    assert payload["evidence_blocking_item_ids"] == evidence_blocking_item_ids
    assert payload["evidence_ignored_item_ids"] == evidence_ignored_item_ids
    assert payload["non_evidence_blocking_item_ids"] == non_evidence_blocking_item_ids
    assert payload["non_evidence_ignored_item_ids"] == non_evidence_ignored_item_ids
    assert set(evidence_gate_item_ids).isdisjoint(non_evidence_item_ids)
    assert sorted(evidence_gate_item_ids + non_evidence_item_ids) == sorted(item_ids)
    assert set(blocking_item_ids).isdisjoint(ignored_item_ids)
    assert set(evidence_blocking_item_ids).isdisjoint(non_evidence_blocking_item_ids)
    assert sorted(evidence_blocking_item_ids + non_evidence_blocking_item_ids) == sorted(
        blocking_item_ids
    )
    assert set(evidence_ignored_item_ids).isdisjoint(non_evidence_ignored_item_ids)
    assert sorted(evidence_ignored_item_ids + non_evidence_ignored_item_ids) == sorted(
        ignored_item_ids
    )
    assert isinstance(payload["evidence_blocking_count"], int)
    assert isinstance(payload["evidence_ignored_count"], int)
    assert payload["evidence_gate_count"] >= payload["evidence_blocking_count"]
    assert payload["evidence_gate_count"] >= payload["evidence_ignored_count"]
    assert payload["evidence_blocking_count"] == len(payload["evidence_blocking_gaps"])
    assert payload["evidence_ignored_count"] == len(payload["evidence_ignored_gaps"])
    assert isinstance(payload["non_evidence_blocking_gaps"], list)
    assert isinstance(payload["non_evidence_ignored_gaps"], list)
    assert payload["blocking_count"] == len(payload["blocking_gaps"])
    assert payload["ignored_count"] == len(payload["ignored_gaps"])
    if result.returncode != 2:
        assert (result.returncode == 0) is payload["ok"]
