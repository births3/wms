"""W6.G TMS evidence materials export template and check-only guard tests."""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


def test_record_wave5_tms_export_template_lists_materials_without_writing(tmp_path, capsys):
    """W6.G 模板模式必须只输出变量清单与 check-only 命令，不写 evidence。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave5_tms_evidence as recorder

    output = tmp_path / "wave-5-tms-evidence.json"
    result = recorder.main(["--export-template", "--output", str(output)])
    template = capsys.readouterr().out

    assert result == 0
    assert "WAVE_5_TMS_ENVIRONMENT=" in template
    assert "WAVE_5_TMS_SYSTEM_REF=" in template
    assert "WAVE_5_TMS_DISPATCH_PUSH_LOG_REF=" in template
    assert "WAVE_5_TMS_CALLBACK_LOG_REF=" in template
    assert "WAVE_5_TMS_FAILURE_RETRY_LOG_REF=" in template
    assert "WAVE_5_TMS_AUDIT_EVENT_QUERY_REF=" in template
    assert "WAVE_5_TMS_CREDENTIAL_REF=" in template
    assert "WAVE_5_TMS_DISPATCHES_RECEIVED=" in template
    assert "WAVE_5_TMS_CALLBACKS_RECEIVED=" in template
    assert "WAVE_5_TMS_FAILED_CALLBACKS_EXERCISED=" in template
    assert "WAVE_5_TMS_RETRY_SUCCEEDED=true" in template
    assert "WAVE_5_TMS_AUDIT_EVENT_VERIFIED=true" in template
    assert "just wave-5-tms-materials --from-env --json" in template
    assert "just wave-5-tms-evidence-record --from-env --check-only --json" in template
    assert "no evidence JSON written" not in template
    assert not output.exists()


def test_record_wave5_tms_export_template_can_be_called_from_materials_entry(
    tmp_path,
    capsys,
):
    """just wave-5-tms-materials --export-template 会叠加 --check-only，仍必须只输出模板。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave5_tms_evidence as recorder

    output = tmp_path / "wave-5-tms-evidence.json"
    result = recorder.main(["--check-only", "--export-template", "--output", str(output)])
    template = capsys.readouterr().out

    assert result == 0
    assert "WAVE_5_TMS_SYSTEM_REF=" in template
    assert "just wave-5-tms-evidence-record --from-env --check-only --json" in template
    assert "the following arguments are required" not in template
    assert not output.exists()


def test_record_wave5_tms_check_only_without_required_args_still_fails(capsys):
    """W6.G check-only 缺真实材料参数时不能静默通过。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave5_tms_evidence as recorder

    with pytest.raises(SystemExit) as excinfo:
        recorder.main(["--check-only"])
    captured = capsys.readouterr()

    assert excinfo.value.code == 2
    assert "the following arguments are required" in captured.err


def test_record_wave5_tms_check_only_json_failure_reports_no_writes(tmp_path, capsys):
    """W6.G check-only JSON 失败时仍需报告不写证据、不关 gate。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave5_tms_evidence as recorder

    evidence = {
        "environment": "staging",
        "tms_system_ref": "partner://wms-staging/tms/vendor-a",
        "dispatch_push_log_ref": "ci/staging/wave5-tms-fake-dispatch/123",
        "callback_log_ref": "ci/staging/wave5-tms-callback/123",
        "failure_retry_log_ref": "ci/staging/wave5-tms-failure-retry/123",
        "audit_event_query_ref": "ci/staging/wave5-tms-audit/123",
        "credential_ref": "vault://wms/staging/tms/vendor-a",
        "dispatches_received": 1,
        "callbacks_received": 1,
        "failed_callbacks_exercised": 1,
        "retry_succeeded": True,
        "audit_event_verified": True,
    }
    output = tmp_path / "wave-5-tms-evidence.json"
    args = [
        "--output",
        str(output),
        "--environment",
        str(evidence["environment"]),
        "--tms-system-ref",
        str(evidence["tms_system_ref"]),
        "--dispatch-push-log-ref",
        str(evidence["dispatch_push_log_ref"]),
        "--callback-log-ref",
        str(evidence["callback_log_ref"]),
        "--failure-retry-log-ref",
        str(evidence["failure_retry_log_ref"]),
        "--audit-event-query-ref",
        str(evidence["audit_event_query_ref"]),
        "--credential-ref",
        str(evidence["credential_ref"]),
        "--dispatches-received",
        str(evidence["dispatches_received"]),
        "--callbacks-received",
        str(evidence["callbacks_received"]),
        "--failed-callbacks-exercised",
        str(evidence["failed_callbacks_exercised"]),
    ]
    if evidence["retry_succeeded"]:
        args.append("--retry-succeeded")
    if evidence["audit_event_verified"]:
        args.append("--audit-event-verified")

    result = recorder.main(["--check-only", "--json", *args])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "prod/production/mock/fake/stub/example" in payload["message"]
    assert not output.exists()
