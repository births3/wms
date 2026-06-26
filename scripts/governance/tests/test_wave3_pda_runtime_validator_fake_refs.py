"""Wave 3 PDA runtime evidence validator 占位与伪造证据拒绝测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import (
    valid_wave3_pda_evidence,
    write_evidence,
)


def test_validate_wave3_pda_runtime_evidence_rejects_placeholder_values(tmp_path):
    """Wave 3 PDA runtime 证据不能保留 YYYY / <...> / 待填等模板占位。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["pda_model"] = "<真实 PDA 型号>"
    evidence["spike005_result_ref"] = (
        "s3://wms-staging-evidence/wave3/pda/spike-005-runtime-YYYYMMDD.md"
    )
    evidence["usability_review_ref"] = (
        "s3://wms-staging-evidence/wave3/pda/usability-review-TODO.md"
    )
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "占位" in message
    assert "pda_model" in message
    assert "spike005_result_ref" in message
    assert "usability_review_ref" in message


def test_validate_wave3_pda_runtime_evidence_rejects_simulator_or_blocked_refs(tmp_path):
    """Wave 3 PDA runtime 证据不能用浏览器、模拟器或禁用边界引用替代。"""
    import validate_wave3_pda_runtime_evidence as validator

    evidence = valid_wave3_pda_evidence()
    evidence["scan_input_method"] = "browser-camera"
    path = tmp_path / "wave-3-pda-runtime-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "browser" in message

    evidence["scan_input_method"] = "physical-scan-key-intent"
    for token in ("prod", "production", "mock", "fake", "stub", "example"):
        evidence["pda_device_ref"] = f"asset://wms-staging/pda/{token}-device"
        write_evidence(path, evidence)

        ok, message = validator.validate_one(path, allow_example_refs=False)

        assert ok is False
        assert "prod/production/mock/fake/stub/example" in message

    evidence["pda_device_ref"] = "asset://wms-staging/pda/honeywell-eda52-01"
    evidence["m2_scan_log_ref"] = "ci/wave3-pda-m2-scan/123"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "environment 标记 staging" in message
