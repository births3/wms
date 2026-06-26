"""Wave 3 PDA runtime evidence validator 成功路径与候选边界测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import (
    valid_wave3_pda_evidence,
    write_evidence,
)


def test_validate_wave3_pda_runtime_evidence_accepts_real_staging_payload(tmp_path):
    """Wave 3 PDA runtime 证据必须接受真 PDA + staging 日志引用。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_validate_wave3_pda_runtime_evidence_accepts_spike005b_result_ref(tmp_path):
    """Wave 3 PDA runtime 证据允许 SPIKE-005B WebView/Capacitor 实测引用。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence("webview-capacitor")
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_validate_wave3_pda_runtime_evidence_requires_webview_native_refs(tmp_path):
    """WebView/Capacitor 证据必须证明 Android native shell 与 native scan plugin。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["pda_stack_candidate"] = "webview-capacitor"
    evidence["spike005_result_ref"] = "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "native_shell_ref" in message
    assert "native_scan_plugin_ref" in message

    evidence["native_shell_ref"] = "ci/staging/wave3-pda-native-shell-webview-capacitor/123"
    evidence["native_scan_plugin_ref"] = "ci/staging/wave3-pda-native-scan-plugin/123"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_validate_wave3_pda_runtime_evidence_cli_json_success_contract(
    tmp_path,
    capsys,
):
    """validator CLI JSON 成功路径必须可被自动收口链稳定消费。"""
    import validate_wave3_pda_runtime_evidence as validator

    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, valid_wave3_pda_evidence())

    result = validator.main([
        "--evidence-file",
        str(path),
        "--json",
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["path"] == str(path)
    assert payload["evidence_file"] == str(path)
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "内容有效" in payload["message"]


def test_validate_wave3_pda_runtime_evidence_cli_json_failure_contract(
    tmp_path,
    capsys,
):
    """validator CLI JSON 失败路径也必须明确只读且不关闭 W6.D。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["pda_device_ref"] = "asset://wms-staging/pda/fake-device"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    result = validator.main([
        "--evidence-file",
        str(path),
        "--json",
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["path"] == str(path)
    assert payload["evidence_file"] == str(path)
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "fake" in payload["message"]


def test_validate_wave3_pda_runtime_evidence_cli_json_allows_example_refs_for_template(
    tmp_path,
    capsys,
):
    """--allow-example-refs 只放开 example 引用 token，用于模板验证。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["pda_device_ref"] = "asset://wms-staging/pda/example-eda52-01"
    path = tmp_path / "wave-3-pda-runtime-evidence.example.json"
    write_evidence(path, evidence)

    blocked_result = validator.main([
        "--evidence-file",
        str(path),
        "--json",
    ])
    blocked_payload = json.loads(capsys.readouterr().out)

    allowed_result = validator.main([
        "--evidence-file",
        str(path),
        "--allow-example-refs",
        "--json",
    ])
    allowed_payload = json.loads(capsys.readouterr().out)

    assert blocked_result == 1
    assert "example" in blocked_payload["message"]
    assert allowed_result == 0
    assert allowed_payload["ok"] is True
    assert allowed_payload["evidence_file"] == str(path)
    assert allowed_payload["writes_runtime_evidence"] is False
    assert allowed_payload["closes_gate"] is False


def test_validate_wave3_pda_runtime_evidence_rejects_candidate_ref_mismatch(tmp_path):
    """PDA 技术栈候选必须与 SPIKE-005 / 005B 实测引用一致。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["pda_stack_candidate"] = "webview-capacitor"
    evidence["native_shell_ref"] = "ci/staging/wave3-pda-native-shell-webview-capacitor/123"
    evidence["native_scan_plugin_ref"] = "ci/staging/wave3-pda-native-scan-plugin/123"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "pda_stack_candidate" in message


def test_validate_wave3_pda_runtime_evidence_rejects_non_spike_result_ref(tmp_path):
    """Wave 3 PDA runtime 证据的 spike 结果引用不能指向任意运行记录。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["spike005_result_ref"] = "s3://wms-staging-evidence/wave3/pda/pda-runtime-20260606.md"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "SPIKE-005" in message
