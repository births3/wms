"""Wave runtime evidence validator error message tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave_runtime_evidence_test_helpers import (
    valid_wave4_external_evidence,
    valid_wave5_hardware_evidence,
    valid_wave5_tms_evidence,
    write_evidence,
)


def test_wave4_validator_reports_blocked_ref_field_names(tmp_path):
    """W6.E blocked ref 错误应指出具体字段。"""
    import validate_wave4_external_dependencies as validator

    evidence = valid_wave4_external_evidence()
    evidence["api_doc_ref"] = "s3://wms-prod-evidence/wave4/traceability/api-doc.pdf"
    evidence["failure_retry_log_ref"] = "ci/staging/wave4-traceability-fake-retry/123"
    path = tmp_path / "wave-4-external-dependencies.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "local/prod/production/mock/fake/stub/example" in message
    assert "api_doc_ref" in message
    assert "failure_retry_log_ref" in message


def test_wave5_hardware_validator_reports_blocked_ref_field_names(tmp_path):
    """W6.F blocked ref 错误应指出具体字段。"""
    import validate_wave5_hardware_evidence as validator

    evidence = valid_wave5_hardware_evidence()
    evidence["scale_device_ref"] = "asset://wms-staging/hardware/fake-scale-01"
    evidence["waybill_print_log_ref"] = "ci/local/wave5-hardware-waybill-print/123"
    path = tmp_path / "wave-5-hardware-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "local/prod/production/mock/fake/stub/example" in message
    assert "scale_device_ref" in message
    assert "waybill_print_log_ref" in message


def test_wave5_tms_validator_reports_blocked_ref_field_names(tmp_path):
    """W6.G blocked ref 错误应指出具体字段。"""
    import validate_wave5_tms_evidence as validator

    evidence = valid_wave5_tms_evidence()
    evidence["dispatch_push_log_ref"] = "ci/staging/wave5-tms-fake-dispatch/123"
    evidence["callback_log_ref"] = "ci/local/wave5-tms-callback/123"
    path = tmp_path / "wave-5-tms-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "local/prod/production/mock/fake/stub/example" in message
    assert "dispatch_push_log_ref" in message
    assert "callback_log_ref" in message
