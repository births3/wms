"""Wave 3 PDA runtime evidence recorder 测试。"""
import json
import sys
from pathlib import Path


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


def test_record_wave3_pda_runtime_evidence_check_only_json_reports_validation_failure(
    tmp_path,
    capsys,
):
    """W6.D check-only JSON 失败路径也必须输出 JSON 且不写 evidence。"""
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
        "--pda-device-ref", "asset://wms-staging/pda/fake-device",
        "--spike-result-ref", "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md",
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
    captured = capsys.readouterr()
    payload = json.loads(captured.out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "证据引用不能指向" in payload["message"]
    assert "fake" in payload["message"]
    assert captured.err == ""
    assert not output.exists()

def test_wave3_pda_validator_reports_blocked_ref_field_names(tmp_path):
    """W6.D blocked ref 错误应指出具体字段。"""
    import validate_wave3_pda_runtime_evidence as validator

    output = tmp_path / "wave-3-pda-runtime-evidence.json"
    output.write_text(
        json.dumps(
            {
                "environment": "staging",
                "pda_model": "Honeywell EDA52",
                "android_version": "Android 11",
                "scan_input_method": "physical-scan-key-intent",
                "pda_stack_candidate": "webview-capacitor",
                "pda_device_ref": "asset://wms-staging/pda/fake-device",
                "spike005_result_ref": "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md",
                "m2_scan_log_ref": "ci/staging/wave3-pda-m2-scan/123",
                "m3_scan_log_ref": "ci/local/wave3-pda-m3-scan/123",
                "offline_replay_log_ref": "ci/staging/wave3-pda-offline-replay/123",
                "idempotency_replay_log_ref": "ci/staging/wave3-pda-idempotency-replay/123",
                "audit_event_query_ref": "ci/staging/wave3-pda-audit-event/123",
                "l7_run_ref": "ci/staging/wave3-pda-l7/123",
                "usability_review_ref": "s3://wms-staging-evidence/wave3/pda/usability-review.md",
                "native_shell_ref": "ci/staging/wave3-pda-native-shell-webview-capacitor/123",
                "native_scan_plugin_ref": "ci/staging/wave3-pda-native-scan-plugin/123",
                "barcode_samples_scanned": 50,
                "m2_operations_exercised": 1,
                "m3_operations_exercised": 1,
                "offline_replays_exercised": 50,
                "idempotency_replays_exercised": 50,
                "real_pda_used": True,
                "physical_scan_key_verified": True,
                "dev_or_staging_service_verified": True,
                "audit_event_verified": True,
                "l7_review_completed": True,
                "usability_review_completed": True,
            },
        ),
        encoding="utf-8",
    )

    ok, message = validator.validate_one(output, allow_example_refs=False)

    assert ok is False
    assert "local/prod/production/mock/fake/stub/example/browser/simulator/emulator/phone/camera" in message
    assert "pda_device_ref" in message
    assert "m3_scan_log_ref" in message

def test_record_wave3_pda_runtime_evidence_check_only_rejects_bad_refs_without_writing(
    tmp_path,
):
    """W6.D check-only 失败时也不能留下正式 PDA evidence。"""
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
        "--pda-device-ref", "asset://wms-staging/pda/fake-device",
        "--spike-result-ref", "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md",
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
    ]) == 1

    assert not output.exists()
