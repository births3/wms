"""Wave 3 PDA runtime evidence validator 拒绝路径与帮助文案测试。"""
import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import valid_wave3_pda_evidence


def _write_evidence(path: Path, evidence: dict[str, object]) -> None:
    path.write_text(json.dumps(evidence), encoding="utf-8")


def test_validate_wave3_pda_runtime_evidence_rejects_insufficient_barcode_samples(tmp_path):
    """PDA runtime 证据必须满足 SPIKE-005 / 005B 的 50 个条码样本口径。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["barcode_samples_scanned"] = 49
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    _write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "barcode_samples_scanned" in message


def test_validate_wave3_pda_runtime_evidence_rejects_insufficient_replay_counts(tmp_path):
    """PDA runtime 证据必须证明 50 个离线任务 replay 与幂等 replay 都被覆盖。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence("webview-capacitor")
    evidence["offline_replays_exercised"] = 49
    evidence["idempotency_replays_exercised"] = 49
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    _write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "offline_replays_exercised" in message
    assert "idempotency_replays_exercised" in message


def test_validate_wave3_pda_runtime_evidence_rejects_phone_camera_scan_method(tmp_path):
    """PDA runtime 证据不能用手机摄像头扫码替代实体扫码键。"""
    import validate_wave3_pda_runtime_evidence as validator

    for scan_input_method in ("phone-camera", "camera-intent"):
        evidence = valid_wave3_pda_evidence()
        evidence["scan_input_method"] = scan_input_method
        path = tmp_path / f"wave-3-pda-runtime-evidence-{scan_input_method}.json"
        _write_evidence(path, evidence)

        ok, message = validator.validate_one(path, allow_example_refs=False)

        assert ok is False
        assert "scan_input_method" in message


def test_validate_wave3_pda_runtime_evidence_rejects_non_pda_device_identity(tmp_path):
    """PDA runtime 证据不能把模拟器、浏览器或手机伪装成真 PDA。"""
    import validate_wave3_pda_runtime_evidence as validator

    cases = [
        ("pda_model", "Android Emulator"),
        ("pda_model", "Pixel phone"),
        ("android_version", "Android 14 simulator image"),
    ]
    for key, value in cases:
        case = valid_wave3_pda_evidence()
        case[key] = value
        path = tmp_path / f"wave-3-pda-runtime-evidence-{key}.json"
        _write_evidence(path, case)

        ok, message = validator.validate_one(path, allow_example_refs=False)

        assert ok is False
        assert key in message


def test_validate_wave3_pda_runtime_evidence_rejects_unknown_fields(tmp_path):
    """PDA runtime 证据 JSON 不能混入未定义字段绕过证据契约。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["operator_photo_ref"] = "s3://wms-staging-evidence/wave3/pda/operator-photo.jpg"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    _write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "未知字段" in message
    assert "operator_photo_ref" in message


def test_validate_wave3_pda_runtime_evidence_rejects_react_native_with_webview_native_refs(
    tmp_path,
):
    """React Native 候选证据不能夹带 WebView/Capacitor native shell/plugin 引用。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence("react-native")
    evidence["native_shell_ref"] = "ci/staging/wave3-pda-native-shell-webview-capacitor/123"
    evidence["native_scan_plugin_ref"] = "ci/staging/wave3-pda-native-scan-plugin/123"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    _write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "react-native" in message
    assert "native_shell_ref" in message
    assert "native_scan_plugin_ref" in message


@pytest.mark.parametrize(
    ("key", "bad_value"),
    [
        ("barcode_samples_scanned", True),
        ("barcode_samples_scanned", 50.0),
        ("barcode_samples_scanned", "50"),
        ("barcode_samples_scanned", None),
        ("m2_operations_exercised", True),
        ("m2_operations_exercised", 1.0),
        ("m2_operations_exercised", "1"),
        ("m2_operations_exercised", None),
        ("m3_operations_exercised", True),
        ("m3_operations_exercised", 1.0),
        ("m3_operations_exercised", "1"),
        ("m3_operations_exercised", None),
        ("offline_replays_exercised", True),
        ("offline_replays_exercised", 50.0),
        ("offline_replays_exercised", "50"),
        ("offline_replays_exercised", None),
        ("idempotency_replays_exercised", True),
        ("idempotency_replays_exercised", 50.0),
        ("idempotency_replays_exercised", "50"),
        ("idempotency_replays_exercised", None),
    ],
)
def test_validate_wave3_pda_runtime_evidence_rejects_non_integer_counts(
    tmp_path,
    key,
    bad_value,
):
    """PDA runtime 计数字段必须是真 JSON integer，不能用 bool/float/string/null。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence[key] = bad_value
    path = tmp_path / f"wave-3-pda-runtime-evidence-{key}.json"
    _write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert key in message


@pytest.mark.parametrize(
    ("key", "bad_value"),
    [
        ("pda_model", ["Honeywell EDA52"]),
        ("android_version", {"value": "Android 11"}),
        ("scan_input_method", ["physical-scan-key-intent"]),
        ("spike005_result_ref", ["s3://wms-staging-evidence/wave3/pda/spike-005-runtime-20260604.md"]),
        ("m2_scan_log_ref", ["ci/staging/wave3-pda-m2-scan/123"]),
        ("audit_event_query_ref", ["ci/staging/wave3-pda-audit-event/123"]),
    ],
)
def test_validate_wave3_pda_runtime_evidence_rejects_non_string_fields(
    tmp_path,
    key,
    bad_value,
):
    """PDA runtime 字符串字段必须是真 JSON string，不能用 array/object 规约绕过。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence[key] = bad_value
    path = tmp_path / f"wave-3-pda-runtime-evidence-{key}.json"
    _write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert key in message
    assert "字符串字段" in message
