"""Wave 3 PDA runtime evidence validator 引用类型拒绝路径测试。"""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import (
    valid_wave3_pda_evidence,
    write_evidence,
)


def test_validate_wave3_pda_runtime_evidence_rejects_non_pda_device_refs(tmp_path):
    """PDA runtime 证据引用不能指向模拟器、手机或摄像头替代证据。"""
    import validate_wave3_pda_runtime_evidence as validator

    cases = [
        ("pda_device_ref", "asset://wms-staging/pda/android-emulator-01"),
        ("pda_device_ref", "asset://wms-staging/pda/pixel-phone-camera-01"),
        ("m2_scan_log_ref", "ci/staging/wave3-pda-phone-camera-scan/123"),
        ("m2_scan_log_ref", "ci/local/wave3-pda-m2-scan/123"),
    ]
    for key, value in cases:
        evidence = valid_wave3_pda_evidence()
        evidence[key] = value
        path = tmp_path / f"wave-3-pda-runtime-evidence-{key}.json"
        write_evidence(path, evidence)

        ok, message = validator.validate_one(path, allow_example_refs=False)

        assert ok is False
        assert "emulator/phone/camera" in message or "local/prod/production" in message


def test_validate_wave3_pda_runtime_evidence_requires_pda_asset_ref(tmp_path):
    """PDA runtime 证据必须把设备记录为 PDA 资产引用，而不是普通日志路径。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["pda_device_ref"] = "ci/staging/wave3-pda-device/honeywell-eda52-01"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "pda_device_ref" in message
    assert "asset://" in message


def test_validate_wave3_pda_runtime_evidence_requires_audit_event_ref(tmp_path):
    """PDA runtime 证据必须引用 H2 audit_event 查询链路。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["audit_event_query_ref"] = "ci/staging/wave3-pda-operator-log/123"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "audit_event_query_ref" in message
    assert "audit_event" in message


def test_validate_wave3_pda_runtime_evidence_requires_idempotency_replay_ref(tmp_path):
    """PDA runtime 证据必须引用 Idempotency-Key replay 链路。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["idempotency_replay_log_ref"] = "ci/staging/wave3-pda-replay/123"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "idempotency_replay_log_ref" in message
    assert "Idempotency-Key" in message


@pytest.mark.parametrize(
    ("key", "value", "expected"),
    [
        ("m2_scan_log_ref", "ci/staging/wave3-pda-m2-replay/123", "M2 scan"),
        ("m3_scan_log_ref", "ci/staging/wave3-pda-inventory-scan/123", "M3 scan"),
        ("offline_replay_log_ref", "ci/staging/wave3-pda-queue-drain/123", "offline replay"),
        ("l7_run_ref", "ci/staging/wave3-pda-performance/123", "L7"),
        ("usability_review_ref", "s3://wms-staging-evidence/wave3/pda/operator-review.md", "usability review"),
    ],
)
def test_validate_wave3_pda_runtime_evidence_requires_typed_operation_refs(
    tmp_path,
    key,
    value,
    expected,
):
    """PDA runtime 证据引用必须指向对应 M2/M3 扫码与离线 replay 链路。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence[key] = value
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert key in message
    assert expected in message
