"""Wave 4/5/6 runtime evidence validator 模板占位拒绝测试。"""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import valid_wave3_pda_evidence
from wave_runtime_evidence_test_helpers import (
    valid_wave4_external_evidence,
    valid_wave5_hardware_evidence,
    valid_wave5_tms_evidence,
    valid_wave6_deploy_evidence,
    write_evidence,
)


@pytest.mark.parametrize(
    ("module_name", "filename", "payload_factory", "placeholder_field"),
    [
        (
            "validate_wave3_pda_runtime_evidence",
            "wave-3-pda-runtime-evidence.json",
            valid_wave3_pda_evidence,
            "l7_run_ref",
        ),
        (
            "validate_wave4_external_dependencies",
            "wave-4-external-dependencies.json",
            valid_wave4_external_evidence,
            "api_doc_ref",
        ),
        (
            "validate_wave5_hardware_evidence",
            "wave-5-hardware-evidence.json",
            valid_wave5_hardware_evidence,
            "calibration_record_ref",
        ),
        (
            "validate_wave5_tms_evidence",
            "wave-5-tms-evidence.json",
            valid_wave5_tms_evidence,
            "dispatch_push_log_ref",
        ),
        (
            "validate_wave6_deploy_evidence",
            "wave-6-deploy-evidence.json",
            valid_wave6_deploy_evidence,
            "release_plan_ref",
        ),
    ],
)
def test_runtime_evidence_validators_reject_placeholder_values(
    tmp_path,
    module_name,
    filename,
    payload_factory,
    placeholder_field,
):
    """Wave 4/5/6 evidence validator 不能让模板占位关闭真实 evidence gate。"""
    validator = __import__(module_name)
    evidence = payload_factory()
    evidence[placeholder_field] = f"{evidence[placeholder_field]}-YYYYMMDD"
    path = tmp_path / filename
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "占位" in message
    assert placeholder_field in message

    ok_with_example_refs, message_with_example_refs = validator.validate_one(
        path,
        allow_example_refs=True,
    )

    assert ok_with_example_refs is False
    assert "占位" in message_with_example_refs
    assert placeholder_field in message_with_example_refs
