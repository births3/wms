"""Wave 3 PDA runtime evidence recorder 测试。"""
import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import valid_wave3_pda_evidence


def _valid_wave3_pda_env(candidate: str = "react-native") -> dict[str, str]:
    evidence = valid_wave3_pda_evidence(candidate)
    return {
        "WAVE_3_PDA_ENVIRONMENT": str(evidence["environment"]),
        "WAVE_3_PDA_PDA_MODEL": str(evidence["pda_model"]),
        "WAVE_3_PDA_ANDROID_VERSION": str(evidence["android_version"]),
        "WAVE_3_PDA_SCAN_INPUT_METHOD": str(evidence["scan_input_method"]),
        "WAVE_3_PDA_STACK_CANDIDATE": str(evidence["pda_stack_candidate"]),
        "WAVE_3_PDA_PDA_DEVICE_REF": str(evidence["pda_device_ref"]),
        "WAVE_3_PDA_SPIKE_RESULT_REF": str(evidence["spike005_result_ref"]),
        "WAVE_3_PDA_M2_SCAN_LOG_REF": str(evidence["m2_scan_log_ref"]),
        "WAVE_3_PDA_M3_SCAN_LOG_REF": str(evidence["m3_scan_log_ref"]),
        "WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF": str(evidence["offline_replay_log_ref"]),
        "WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF": str(
            evidence["idempotency_replay_log_ref"],
        ),
        "WAVE_3_PDA_AUDIT_EVENT_QUERY_REF": str(evidence["audit_event_query_ref"]),
        "WAVE_3_PDA_L7_RUN_REF": str(evidence["l7_run_ref"]),
        "WAVE_3_PDA_USABILITY_REVIEW_REF": str(evidence["usability_review_ref"]),
        "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED": str(evidence["barcode_samples_scanned"]),
        "WAVE_3_PDA_M2_OPERATIONS_EXERCISED": str(evidence["m2_operations_exercised"]),
        "WAVE_3_PDA_M3_OPERATIONS_EXERCISED": str(evidence["m3_operations_exercised"]),
        "WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED": str(
            evidence["offline_replays_exercised"],
        ),
        "WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED": str(
            evidence["idempotency_replays_exercised"],
        ),
        "WAVE_3_PDA_REAL_PDA_USED": "true",
        "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED": "true",
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED": "true",
        "WAVE_3_PDA_AUDIT_EVENT_VERIFIED": "true",
        "WAVE_3_PDA_L7_REVIEW_COMPLETED": "true",
        "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED": "true",
        "WAVE_3_PDA_NATIVE_SHELL_REF": str(evidence.get("native_shell_ref", "")),
        "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF": str(
            evidence.get("native_scan_plugin_ref", ""),
        ),
    }


@pytest.mark.parametrize(
    ("guard_field", "message"),
    [
        (
            "writes_runtime_evidence",
            "intake writes_runtime_evidence is required and must be false",
        ),
        ("closes_gate", "intake closes_gate is required and must be false"),
    ],
)
def test_record_wave3_pda_runtime_evidence_from_intake_file_rejects_write_claims(
    tmp_path,
    capsys,
    guard_field,
    message,
):
    """intake wrapper 不能声称自己写 evidence 或关闭 W6.D。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(
        json.dumps({
            "schema_version": 1,
            "kind": "wave3-pda-runtime-evidence-intake",
            "writes_runtime_evidence": False,
            "closes_gate": False,
            guard_field: True,
            "evidence": valid_wave3_pda_evidence(),
        }, ensure_ascii=False),
        encoding="utf-8",
    )

    result = recorder.main([
        "--from-intake-file",
        str(intake),
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert message in payload["message"]
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_intake_file_rejects_schema_version_drift(
    tmp_path,
    capsys,
):
    """from-intake-file 必须拒绝不支持的 schema_version。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(
        json.dumps({
            "schema_version": 2,
            "kind": "wave3-pda-runtime-evidence-intake",
            "writes_runtime_evidence": False,
            "closes_gate": False,
            "evidence": valid_wave3_pda_evidence(),
        }, ensure_ascii=False),
        encoding="utf-8",
    )

    result = recorder.main([
        "--from-intake-file",
        str(intake),
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "intake schema_version is required and must be 1" in payload["message"]
    assert not output.exists()

@pytest.mark.parametrize(
    ("field", "bad_value", "message"),
    [
        ("scan_input_method", ["physical-scan-key-intent"], "scan_input_method must be a JSON string"),
        ("barcode_samples_scanned", "50", "barcode_samples_scanned must be a JSON integer"),
        ("offline_replays_exercised", 50.0, "offline_replays_exercised must be a JSON integer"),
        ("real_pda_used", "true", "real_pda_used must be a JSON boolean"),
        ("unexpected_extra_ref", "ci/staging/extra/123", "intake evidence contains unknown fields: unexpected_extra_ref"),
    ],
)
def test_record_wave3_pda_runtime_evidence_from_intake_file_rejects_raw_type_drift(
    tmp_path,
    capsys,
    field,
    bad_value,
    message,
):
    """intake check-only 不得把错误 JSON 类型静默强转成正式 evidence 类型。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    evidence = valid_wave3_pda_evidence()
    evidence[field] = bad_value
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(
        json.dumps({
            "schema_version": 1,
            "kind": "wave3-pda-runtime-evidence-intake",
            "writes_runtime_evidence": False,
            "closes_gate": False,
            "evidence": evidence,
        }, ensure_ascii=False),
        encoding="utf-8",
    )

    result = recorder.main([
        "--from-intake-file",
        str(intake),
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert message in payload["message"]
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_intake_file_reports_missing_ref_owner(
    tmp_path,
    capsys,
):
    """intake 文件缺关键证据字段时，应输出缺失变量和负责人。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    evidence = valid_wave3_pda_evidence()
    evidence.pop("pda_device_ref")
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(
        json.dumps({
            "schema_version": 1,
            "kind": "wave3-pda-runtime-evidence-intake",
            "writes_runtime_evidence": False,
            "closes_gate": False,
            "evidence": evidence,
        }, ensure_ascii=False),
        encoding="utf-8",
    )

    result = recorder.main([
        "--from-intake-file",
        str(intake),
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "--pda-device-ref" in payload["message"]
    assert payload["missing_args"] == ["--pda-device-ref"]
    assert payload["missing_env_vars"] == ["WAVE_3_PDA_PDA_DEVICE_REF"]
    assert payload["missing_env_var_owners"][0]["source_owner"] == "资产负责人"
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_intake_file_requires_webview_native_refs(
    tmp_path,
    capsys,
):
    """WebView/Capacitor intake 缺 native refs 时必须输出缺失变量和负责人。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    evidence = valid_wave3_pda_evidence("webview-capacitor")
    evidence.pop("native_shell_ref")
    evidence.pop("native_scan_plugin_ref")
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(
        json.dumps({
            "schema_version": 1,
            "kind": "wave3-pda-runtime-evidence-intake",
            "writes_runtime_evidence": False,
            "closes_gate": False,
            "evidence": evidence,
        }, ensure_ascii=False),
        encoding="utf-8",
    )

    result = recorder.main([
        "--from-intake-file",
        str(intake),
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["missing_args"] == [
        "--native-shell-ref",
        "--native-scan-plugin-ref",
    ]
    assert payload["missing_env_vars"] == [
        "WAVE_3_PDA_NATIVE_SHELL_REF",
        "WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF",
    ]
    assert {
        owner["source_owner"]
        for owner in payload["missing_env_var_owners"]
    } == {"PDA 技术验证负责人"}
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_rejects_missing_required_ref(
    tmp_path,
    monkeypatch,
    capsys,
):
    """from-env 缺关键证据变量时必须失败，避免现场误以为空值可通过。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env()
    env.pop("WAVE_3_PDA_PDA_DEVICE_REF")
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    with pytest.raises(SystemExit) as error:
        recorder.main([
            "--from-env",
            "--check-only",
            "--output",
            str(output),
        ])

    captured = capsys.readouterr()
    assert error.value.code == 2
    assert "--pda-device-ref" in captured.err
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_json_reports_missing_required_ref(
    tmp_path,
    monkeypatch,
    capsys,
):
    """record --json 缺现场变量时应输出结构化错误，便于自动采集链消费。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env()
    env.pop("WAVE_3_PDA_PDA_DEVICE_REF")
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    result = recorder.main([
        "--from-env",
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    captured = capsys.readouterr()
    payload = json.loads(captured.out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "--pda-device-ref" in payload["message"]
    assert payload["missing_args"] == ["--pda-device-ref"]
    assert payload["missing_env_vars"] == ["WAVE_3_PDA_PDA_DEVICE_REF"]
    assert payload["missing_env_var_owners"] == [
        {
            "env_var": "WAVE_3_PDA_PDA_DEVICE_REF",
            "source_owner": "资产负责人",
            "no_pda_stage": "blocked_until_device",
            "requires_real_pda": True,
            "evidence_requirement": "PDA 资产引用",
        },
    ]
    assert captured.err == ""
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_json_formal_record_error_not_check_only(
    tmp_path,
    monkeypatch,
    capsys,
):
    """正式 record 的结构化输入错误不能误标为 check-only。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env()
    env.pop("WAVE_3_PDA_PDA_DEVICE_REF")
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    result = recorder.main([
        "--from-env",
        "--json",
        "--output",
        str(output),
    ])
    captured = capsys.readouterr()
    payload = json.loads(captured.out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "--pda-device-ref" in payload["message"]
    assert payload["missing_args"] == ["--pda-device-ref"]
    assert payload["missing_env_vars"] == ["WAVE_3_PDA_PDA_DEVICE_REF"]
    assert captured.err == ""
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_json_formal_record_reports_write(
    tmp_path,
    monkeypatch,
    capsys,
):
    """正式 record --json 成功时必须结构化报告写入结果。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    for key, value in _valid_wave3_pda_env().items():
        monkeypatch.setenv(key, value)

    result = recorder.main([
        "--from-env",
        "--json",
        "--output",
        str(output),
    ])
    captured = capsys.readouterr()
    payload = json.loads(captured.out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is False
    assert payload["writes_runtime_evidence"] is True
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "wrote" in payload["message"]
    assert captured.err == ""
    assert output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_json_formal_validation_failure(
    tmp_path,
    monkeypatch,
    capsys,
):
    """正式 record --json 校验失败时也必须输出 JSON 且不写 evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_PDA_DEVICE_REF"] = "asset://wms-staging/pda/fake-device"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    result = recorder.main([
        "--from-env",
        "--json",
        "--output",
        str(output),
    ])
    captured = capsys.readouterr()
    payload = json.loads(captured.out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "fake" in payload["message"]
    assert captured.err == ""
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_strips_boolean_whitespace(
    tmp_path,
    monkeypatch,
    capsys,
):
    """record from-env 布尔变量允许 shell 拷贝时的首尾空白。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_REAL_PDA_USED"] = " true "
    env["WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED"] = "\ttrue\n"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    result = recorder.main([
        "--from-env",
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["writes_runtime_evidence"] is False
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_strips_string_and_integer_whitespace(
    tmp_path,
    monkeypatch,
):
    """record from-env 允许现场复制变量值时带首尾空白。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_ENVIRONMENT"] = " staging "
    env["WAVE_3_PDA_PDA_MODEL"] = "\tHoneywell EDA52\n"
    env["WAVE_3_PDA_BARCODE_SAMPLES_SCANNED"] = " 50 "
    env["WAVE_3_PDA_M2_OPERATIONS_EXERCISED"] = "\t1\n"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    assert recorder.main([
        "--from-env",
        "--output",
        str(output),
    ]) == 0

    payload = json.loads(output.read_text(encoding="utf-8"))
    assert payload["environment"] == "staging"
    assert payload["pda_model"] == "Honeywell EDA52"
    assert payload["barcode_samples_scanned"] == 50
    assert payload["m2_operations_exercised"] == 1

def test_record_wave3_pda_runtime_evidence_from_env_rejects_non_positive_counts_json(
    tmp_path,
    monkeypatch,
    capsys,
):
    """record from-env 计数字段必须为正整数，不能等 validator 后置兜底。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env()
    env["WAVE_3_PDA_BARCODE_SAMPLES_SCANNED"] = "0"
    env["WAVE_3_PDA_M2_OPERATIONS_EXERCISED"] = "-1"
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    result = recorder.main([
        "--from-env",
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 2
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "WAVE_3_PDA_BARCODE_SAMPLES_SCANNED must be > 0" in payload["message"]
    assert "WAVE_3_PDA_M2_OPERATIONS_EXERCISED must be > 0" in payload["message"]
    assert not output.exists()

def test_record_wave3_pda_runtime_evidence_from_env_rejects_blank_webview_native_refs(
    tmp_path,
    monkeypatch,
    capsys,
):
    """WebView/Capacitor 候选下 native refs 为空时 check-only 必须失败。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    env = _valid_wave3_pda_env("webview-capacitor")
    env["WAVE_3_PDA_NATIVE_SHELL_REF"] = ""
    env["WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF"] = ""
    for key, value in env.items():
        monkeypatch.setenv(key, value)

    with pytest.raises(SystemExit) as error:
        recorder.main([
            "--from-env",
            "--check-only",
            "--output",
            str(output),
        ])

    captured = capsys.readouterr()
    assert error.value.code == 2
    assert "--native-shell-ref" in captured.err
    assert "--native-scan-plugin-ref" in captured.err
    assert not output.exists()
