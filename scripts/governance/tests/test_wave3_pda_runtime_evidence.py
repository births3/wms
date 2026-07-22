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


def test_record_wave3_pda_runtime_evidence_writes_valid_evidence(tmp_path):
    """Wave 3 PDA 记录脚本生成的 evidence 必须能被 validator 接受。"""
    import record_wave3_pda_runtime_evidence as recorder
    import validate_wave3_pda_runtime_evidence as validator

    output = tmp_path / "wave-3-pda-runtime-evidence.json"

    assert recorder.main([
        "--output", str(output),
        "--environment", "staging",
        "--pda-model", "Honeywell EDA52",
        "--android-version", "Android 11",
        "--scan-input-method", "physical-scan-key-intent",
        "--pda-stack-candidate", "webview-capacitor",
        "--pda-device-ref", "asset://wms-staging/pda/honeywell-eda52-01",
        "--spike005-result-ref", "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md",
        "--m2-scan-log-ref", "ci/staging/wave3-pda-m2-scan/123",
        "--m3-scan-log-ref", "ci/staging/wave3-pda-m3-scan/123",
        "--offline-replay-log-ref", "ci/staging/wave3-pda-offline-replay/123",
        "--idempotency-replay-log-ref", "ci/staging/wave3-pda-idempotency-replay/123",
        "--audit-event-query-ref", "ci/staging/wave3-pda-audit-event/123",
        "--l7-run-ref", "ci/staging/wave3-pda-l7/123",
        "--usability-review-ref", "s3://wms-staging-evidence/wave3/pda/usability-review.md",
        "--native-shell-ref", "ci/staging/wave3-pda-native-shell-webview-capacitor/123",
        "--native-scan-plugin-ref", "ci/staging/wave3-pda-native-scan-plugin/123",
        "--barcode-samples-scanned", "50",
        "--m2-operations-exercised", "1",
        "--m3-operations-exercised", "1",
        "--offline-replays-exercised", "50",
        "--idempotency-replays-exercised", "50",
        "--real-pda-used",
        "--physical-scan-key-verified",
        "--dev-or-staging-service-verified",
        "--audit-event-verified",
        "--l7-review-completed",
        "--usability-review-completed",
    ]) == 0

    ok, message = validator.validate_one(output, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_record_wave3_pda_runtime_evidence_check_only_validates_without_writing(
    tmp_path,
):
    """W6.D check-only 只校验证据字段和边界，不生成正式 PDA evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"

    assert recorder.main([
        "--check-only",
        "--output", str(output),
        "--environment", "staging",
        "--pda-model", "Honeywell EDA52",
        "--android-version", "Android 11",
        "--scan-input-method", "physical-scan-key-intent",
        "--pda-stack-candidate", "webview-capacitor",
        "--pda-device-ref", "asset://wms-staging/pda/honeywell-eda52-01",
        "--spike005-result-ref", "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md",
        "--m2-scan-log-ref", "ci/staging/wave3-pda-m2-scan/123",
        "--m3-scan-log-ref", "ci/staging/wave3-pda-m3-scan/123",
        "--offline-replay-log-ref", "ci/staging/wave3-pda-offline-replay/123",
        "--idempotency-replay-log-ref", "ci/staging/wave3-pda-idempotency-replay/123",
        "--audit-event-query-ref", "ci/staging/wave3-pda-audit-event/123",
        "--l7-run-ref", "ci/staging/wave3-pda-l7/123",
        "--usability-review-ref", "s3://wms-staging-evidence/wave3/pda/usability-review.md",
        "--native-shell-ref", "ci/staging/wave3-pda-native-shell-webview-capacitor/123",
        "--native-scan-plugin-ref", "ci/staging/wave3-pda-native-scan-plugin/123",
        "--barcode-samples-scanned", "50",
        "--m2-operations-exercised", "1",
        "--m3-operations-exercised", "1",
        "--offline-replays-exercised", "50",
        "--idempotency-replays-exercised", "50",
        "--real-pda-used",
        "--physical-scan-key-verified",
        "--dev-or-staging-service-verified",
        "--audit-event-verified",
        "--l7-review-completed",
        "--usability-review-completed",
    ]) == 0

    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_check_only_json_reports_no_writes(
    tmp_path,
    capsys,
):
    """W6.D check-only JSON 必须明确不写 runtime evidence、不关闭 gate。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"

    result = recorder.main([
        "--check-only",
        "--json",
        "--output", str(output),
        "--environment", "staging",
        "--pda-model", "Honeywell EDA52",
        "--android-version", "Android 11",
        "--scan-input-method", "physical-scan-key-intent",
        "--pda-stack-candidate", "webview-capacitor",
        "--pda-device-ref", "asset://wms-staging/pda/honeywell-eda52-01",
        "--spike005-result-ref", "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md",
        "--m2-scan-log-ref", "ci/staging/wave3-pda-m2-scan/123",
        "--m3-scan-log-ref", "ci/staging/wave3-pda-m3-scan/123",
        "--offline-replay-log-ref", "ci/staging/wave3-pda-offline-replay/123",
        "--idempotency-replay-log-ref", "ci/staging/wave3-pda-idempotency-replay/123",
        "--audit-event-query-ref", "ci/staging/wave3-pda-audit-event/123",
        "--l7-run-ref", "ci/staging/wave3-pda-l7/123",
        "--usability-review-ref", "s3://wms-staging-evidence/wave3/pda/usability-review.md",
        "--native-shell-ref", "ci/staging/wave3-pda-native-shell-webview-capacitor/123",
        "--native-scan-plugin-ref", "ci/staging/wave3-pda-native-scan-plugin/123",
        "--barcode-samples-scanned", "50",
        "--m2-operations-exercised", "1",
        "--m3-operations-exercised", "1",
        "--offline-replays-exercised", "50",
        "--idempotency-replays-exercised", "50",
        "--real-pda-used",
        "--physical-scan-key-verified",
        "--dev-or-staging-service-verified",
        "--audit-event-verified",
        "--l7-review-completed",
        "--usability-review-completed",
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "no PDA runtime evidence JSON written" in payload["message"]
    assert "W6.D gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_from_env_check_only_json_reports_no_writes(
    tmp_path,
    monkeypatch,
    capsys,
):
    """from-env check-only 预检现场变量，不写正式 PDA runtime evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    for key, value in _valid_wave3_pda_env().items():
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
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "no PDA runtime evidence JSON written" in payload["message"]
    assert "W6.D gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_export_intake_template_json_no_write(
    tmp_path,
    capsys,
):
    """intake 模板用于现场填写 JSON，不写正式 PDA runtime evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    result = recorder.main([
        "--export-intake-template",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["mode"] == "wave3-pda-runtime-evidence-intake-template"
    assert payload["kind"] == "wave3-pda-runtime-evidence-intake"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert payload["evidence"]["environment"] == "staging"
    assert payload["evidence"]["pda_stack_candidate"] == "react-native"
    assert payload["evidence"]["real_pda_used"] is False
    assert payload["evidence"]["barcode_samples_scanned"] == 50
    assert (
        "Empty string values and false truth flags mean field evidence is still "
        "missing and must be filled by the assigned owner."
    ) in payload["instructions"]
    assert "pda_device_ref" in payload["required_evidence_fields"]
    assert "native_shell_ref" in payload["webview_capacitor_evidence_fields"]
    assert payload["record_gate_after_intake"] == [
        "just wave-3-pda-intake-check --json",
        "just wave-3-pda-intake-record --json",
        "just wave-3-pda-runtime-evidence-validate",
    ]
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY" not in json.dumps(payload)
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_export_intake_template_default_path_is_portable(
    capsys,
):
    """默认 intake 模板不能把当前机器绝对路径写入现场交接材料。"""
    import record_wave3_pda_runtime_evidence as recorder

    result = recorder.main([
        "--export-intake-template",
        "--json",
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["evidence_file"] == "docs/retros/wave-3-pda-runtime-evidence.json"
    assert "/home/" not in payload["evidence_file"]
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False


def test_record_wave3_pda_runtime_evidence_export_intake_template_can_write_file(
    tmp_path,
    capsys,
):
    """intake 模板可安全落盘供现场填写，但不能写正式 PDA evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake_template = tmp_path / "field" / "wave-3-pda-intake.staging.json"

    result = recorder.main([
        "--export-intake-template",
        "--json",
        "--output",
        str(output),
        "--intake-template-output",
        str(intake_template),
    ])
    stdout_payload = json.loads(capsys.readouterr().out)
    file_payload = json.loads(intake_template.read_text(encoding="utf-8"))

    assert result == 0
    assert stdout_payload["ok"] is True
    assert stdout_payload["writes_runtime_evidence"] is False
    assert stdout_payload["closes_gate"] is False
    assert stdout_payload["writes_intake_template"] is True
    assert stdout_payload["intake_template_output"] == str(intake_template)
    assert "wrote" in stdout_payload["message"]
    assert file_payload["mode"] == "wave3-pda-runtime-evidence-intake-template"
    assert file_payload["kind"] == "wave3-pda-runtime-evidence-intake"
    assert file_payload["writes_runtime_evidence"] is False
    assert file_payload["closes_gate"] is False
    assert file_payload["writes_intake_template"] is True
    assert "WAVE_3_PDA_TRACE_CODE_API_KEY" not in json.dumps(file_payload)
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_from_intake_template_reports_false_flag_owners(
    tmp_path,
    capsys,
):
    """直接校验未填写 intake 模板时，应同时指出未确认 truth flags 的负责人。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake_template = tmp_path / "field" / "wave-3-pda-intake.staging.json"

    assert recorder.main([
        "--export-intake-template",
        "--json",
        "--output",
        str(output),
        "--intake-template-output",
        str(intake_template),
    ]) == 0
    capsys.readouterr()

    result = recorder.main([
        "--from-intake-file",
        str(intake_template),
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
    assert payload["false_flag_env_vars"] == [
        "WAVE_3_PDA_REAL_PDA_USED",
        "WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED",
        "WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED",
        "WAVE_3_PDA_AUDIT_EVENT_VERIFIED",
        "WAVE_3_PDA_L7_REVIEW_COMPLETED",
        "WAVE_3_PDA_USABILITY_REVIEW_COMPLETED",
    ]
    assert {
        owner["env_var"]
        for owner in payload["false_flag_env_var_owners"]
    } == set(payload["false_flag_env_vars"])
    for owner in payload["false_flag_env_var_owners"]:
        assert owner["source_owner"]
        assert owner["evidence_requirement"]
        assert owner["requires_real_pda"] in (True, False)
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_export_intake_template_rejects_overwrite(
    tmp_path,
    capsys,
):
    """intake 模板文件已存在时默认拒绝覆盖，避免覆盖现场已填材料。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake_template = tmp_path / "field" / "wave-3-pda-intake.staging.json"
    intake_template.parent.mkdir(parents=True)
    intake_template.write_text('{"existing": true}\n', encoding="utf-8")

    result = recorder.main([
        "--export-intake-template",
        "--json",
        "--output",
        str(output),
        "--intake-template-output",
        str(intake_template),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["writes_intake_template"] is False
    assert "already exists; pass --intake-template-force to overwrite" in payload[
        "message"
    ]
    assert json.loads(intake_template.read_text(encoding="utf-8")) == {
        "existing": True,
    }
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_export_intake_template_force_overwrites(
    tmp_path,
    capsys,
):
    """明确 force 后才可重生成 intake 模板文件。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake_template = tmp_path / "field" / "wave-3-pda-intake.staging.json"
    intake_template.parent.mkdir(parents=True)
    intake_template.write_text('{"existing": true}\n', encoding="utf-8")

    result = recorder.main([
        "--export-intake-template",
        "--json",
        "--output",
        str(output),
        "--intake-template-output",
        str(intake_template),
        "--intake-template-force",
    ])
    stdout_payload = json.loads(capsys.readouterr().out)
    file_payload = json.loads(intake_template.read_text(encoding="utf-8"))

    assert result == 0
    assert stdout_payload["ok"] is True
    assert stdout_payload["writes_intake_template"] is True
    assert file_payload["mode"] == "wave3-pda-runtime-evidence-intake-template"
    assert "existing" not in file_payload
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_from_intake_file_check_only_json_no_write(
    tmp_path,
    capsys,
):
    """from-intake-file check-only 校验现场 JSON，不写正式 PDA evidence。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(
        json.dumps({
            "schema_version": 1,
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

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "check-only passed" in payload["message"]
    assert not output.exists()


def test_record_wave3_pda_runtime_evidence_from_intake_file_rejects_wrong_kind(
    tmp_path,
    capsys,
):
    """from-intake-file 必须拒绝错误 wrapper，避免现场交错附件。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(
        json.dumps({
            "schema_version": 1,
            "kind": "wave3-pda-field-precheck-attachment",
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
    assert (
        "intake kind is required and must be wave3-pda-runtime-evidence-intake"
        in payload["message"]
    )
    assert not output.exists()


@pytest.mark.parametrize(
    ("missing_field", "message"),
    [
        ("schema_version", "intake schema_version is required and must be 1"),
        ("kind", "intake kind is required and must be wave3-pda-runtime-evidence-intake"),
        (
            "writes_runtime_evidence",
            "intake writes_runtime_evidence is required and must be false",
        ),
        ("closes_gate", "intake closes_gate is required and must be false"),
    ],
)
def test_record_wave3_pda_runtime_evidence_from_intake_file_requires_wrapper_contract(
    tmp_path,
    capsys,
    missing_field,
    message,
):
    """from-intake-file 必须强制 wrapper 合同，避免裸 evidence 或混用附件。"""
    import record_wave3_pda_runtime_evidence as recorder

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    intake_payload = {
        "schema_version": 1,
        "kind": "wave3-pda-runtime-evidence-intake",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence": valid_wave3_pda_evidence(),
    }
    intake_payload.pop(missing_field)
    intake = tmp_path / "wave3-pda-intake.json"
    intake.write_text(json.dumps(intake_payload, ensure_ascii=False), encoding="utf-8")

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



































_MOVED_TESTS = {
    'test_record_wave3_pda_runtime_evidence_from_intake_file_rejects_write_claims': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_intake_file_rejects_schema_version_drift': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_intake_file_rejects_raw_type_drift': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_intake_file_reports_missing_ref_owner': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_intake_file_requires_webview_native_refs': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_rejects_missing_required_ref': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_json_reports_missing_required_ref': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_json_formal_record_error_not_check_only': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_json_formal_record_reports_write': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_json_formal_validation_failure': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_strips_boolean_whitespace': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_strips_string_and_integer_whitespace': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_rejects_non_positive_counts_json': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_from_env_rejects_blank_webview_native_refs': 'test_wave3_pda_runtime_evidence_part_1',
    'test_record_wave3_pda_runtime_evidence_check_only_json_reports_validation_failure': 'test_wave3_pda_runtime_evidence_part_2',
    'test_wave3_pda_validator_reports_blocked_ref_field_names': 'test_wave3_pda_runtime_evidence_part_2',
    'test_record_wave3_pda_runtime_evidence_check_only_rejects_bad_refs_without_writing': 'test_wave3_pda_runtime_evidence_part_2',
}

def __getattr__(name: str):
    module = _MOVED_TESTS.get(name)
    if module is None:
        raise AttributeError(name)
    return getattr(__import__(module), name)
