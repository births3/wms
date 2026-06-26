"""Wave 5 runtime evidence validator 测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave_runtime_evidence_test_helpers import (
    valid_wave5_hardware_evidence,
    valid_wave5_tms_evidence,
    write_evidence,
)


def wave5_hardware_cli_args(
    evidence: dict[str, object],
    *,
    output: Path | None = None,
) -> list[str]:
    """Build recorder CLI args from the shared valid Wave 5 hardware fixture."""
    args = []
    if output is not None:
        args.extend(["--output", str(output)])
    args.extend([
        "--environment",
        str(evidence["environment"]),
        "--station-code",
        str(evidence["station_code"]),
        "--scale-device-ref",
        str(evidence["scale_device_ref"]),
        "--bluetooth-printer-ref",
        str(evidence["bluetooth_printer_ref"]),
        "--waybill-printer-ref",
        str(evidence["waybill_printer_ref"]),
        "--calibration-record-ref",
        str(evidence["calibration_record_ref"]),
        "--scale-reading-log-ref",
        str(evidence["scale_reading_log_ref"]),
        "--bluetooth-print-log-ref",
        str(evidence["bluetooth_print_log_ref"]),
        "--waybill-print-log-ref",
        str(evidence["waybill_print_log_ref"]),
        "--audit-event-query-ref",
        str(evidence["audit_event_query_ref"]),
        "--scale-readings-recorded",
        str(evidence["scale_readings_recorded"]),
        "--bluetooth-labels-printed",
        str(evidence["bluetooth_labels_printed"]),
        "--waybills-printed",
        str(evidence["waybills_printed"]),
    ])
    if evidence["hardware_connected"]:
        args.append("--hardware-connected")
    if evidence["print_artifacts_reviewed"]:
        args.append("--print-artifacts-reviewed")
    if evidence["audit_event_verified"]:
        args.append("--audit-event-verified")
    return args


def wave5_tms_cli_args(
    evidence: dict[str, object],
    *,
    output: Path | None = None,
) -> list[str]:
    """Build recorder CLI args from the shared valid Wave 5 TMS fixture."""
    args = []
    if output is not None:
        args.extend(["--output", str(output)])
    args.extend([
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
    ])
    if evidence["retry_succeeded"]:
        args.append("--retry-succeeded")
    if evidence["audit_event_verified"]:
        args.append("--audit-event-verified")
    return args


def test_wave5_hardware_materials_and_readiness_entries_are_read_only():
    """W6.F materials/readiness 入口必须只校验证据输入，不能写 hardware evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")

    assert check.just_recipe_commands(
        just_text,
        "wave-5-hardware-materials",
    ) == [[
        "python3",
        "scripts/governance/record_wave5_hardware_evidence.py",
        "--check-only",
        "{{args}}",
    ]]
    assert check.just_recipe_commands(
        just_text,
        "wave-5-hardware-readiness",
    ) == [[
        "python3",
        "scripts/governance/record_wave5_hardware_evidence.py",
        "--check-only",
        "{{args}}",
    ]]


def test_wave5_tms_materials_and_readiness_entries_are_read_only():
    """W6.G materials/readiness 入口必须只校验证据输入，不能写 TMS evidence。"""
    import check_wave6_evidence_preflight as check

    just_text = Path("justfile").read_text(encoding="utf-8")
    expected = [
        "python3",
        "scripts/governance/record_wave5_tms_evidence.py",
        "--check-only",
        "{{args}}",
    ]

    assert check.just_recipe_commands(
        just_text,
        "wave-5-tms-materials",
    ) == [expected]
    assert check.just_recipe_commands(
        just_text,
        "wave-5-tms-readiness",
    ) == [expected]


def test_wave5_hardware_readiness_chain_is_documented_before_record():
    """W6.F closeout/runbook 必须先列只读预检，再列 record/validate。"""
    closeout_text = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")
    preflight_text = Path("docs/runbooks/wave-6-evidence-preflight.md").read_text(
        encoding="utf-8",
    )
    runbook_text = Path("docs/runbooks/wave-5-hardware-evidence.md").read_text(
        encoding="utf-8",
    )

    for text in (closeout_text, preflight_text, runbook_text):
        assert "just wave-5-hardware-materials" in text
        assert "just wave-5-hardware-readiness" in text
        assert "just wave-5-hardware-evidence-record" in text

    closeout_order = [
        closeout_text.index("just wave-5-hardware-materials"),
        closeout_text.index("just wave-5-hardware-readiness"),
        closeout_text.index("just wave-5-hardware-evidence-record"),
        closeout_text.index("just wave-5-hardware-evidence-validate"),
    ]
    runbook_order = [
        runbook_text.index("just wave-5-hardware-materials"),
        runbook_text.index("just wave-5-hardware-readiness"),
        runbook_text.index("just wave-5-hardware-evidence-record"),
        runbook_text.index("just wave-5-hardware-evidence-validate"),
    ]

    assert closeout_order == sorted(closeout_order)
    assert runbook_order == sorted(runbook_order)
    assert "不连接真实硬件" in runbook_text
    assert "不写 `docs/retros/wave-5-hardware-evidence.json`" in runbook_text
    assert "不能关闭 W6.F gate" in runbook_text


def test_wave5_tms_readiness_chain_is_documented_before_record():
    """W6.G closeout/runbook 必须先列只读预检，再列 record/validate。"""
    closeout_text = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")
    preflight_text = Path("docs/runbooks/wave-6-evidence-preflight.md").read_text(
        encoding="utf-8",
    )
    runbook_text = Path("docs/runbooks/wave-5-tms-evidence.md").read_text(
        encoding="utf-8",
    )

    for text in (closeout_text, preflight_text, runbook_text):
        assert "just wave-5-tms-materials" in text
        assert "just wave-5-tms-readiness" in text
        assert "just wave-5-tms-evidence-record" in text

    closeout_order = [
        closeout_text.index("just wave-5-tms-materials"),
        closeout_text.index("just wave-5-tms-readiness"),
        closeout_text.index("just wave-5-tms-evidence-record"),
        closeout_text.index("just wave-5-tms-evidence-validate"),
    ]
    runbook_order = [
        runbook_text.index("just wave-5-tms-materials"),
        runbook_text.index("just wave-5-tms-readiness"),
        runbook_text.index("just wave-5-tms-evidence-record"),
        runbook_text.index("just wave-5-tms-evidence-validate"),
    ]

    assert closeout_order == sorted(closeout_order)
    assert runbook_order == sorted(runbook_order)
    assert "不调用 TMS" in runbook_text
    assert "不写 `docs/retros/wave-5-tms-evidence.json`" in runbook_text
    assert "不能关闭 W6.G gate" in runbook_text


def test_record_wave5_hardware_check_only_validates_without_writing(tmp_path):
    """W6.F check-only 只校验字段、引用和边界，不生成 evidence JSON。"""
    import record_wave5_hardware_evidence as recorder

    evidence = valid_wave5_hardware_evidence()
    output = tmp_path / "wave-5-hardware-evidence.json"

    result = recorder.main([
        "--check-only",
        *wave5_hardware_cli_args(evidence, output=output),
    ])

    assert result == 0
    assert not output.exists()


def test_record_wave5_hardware_check_only_json_reports_no_writes(tmp_path, capsys):
    """W6.F check-only JSON 必须明确不写 runtime evidence、不关闭 gate。"""
    import record_wave5_hardware_evidence as recorder

    evidence = valid_wave5_hardware_evidence()
    output = tmp_path / "wave-5-hardware-evidence.json"

    result = recorder.main([
        "--check-only",
        "--json",
        *wave5_hardware_cli_args(evidence, output=output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "W6.F gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave5_hardware_check_only_json_uses_relative_default_path(capsys):
    """W6.F 默认 evidence 目标在 JSON 中使用仓库相对路径，便于交接。"""
    import record_wave5_hardware_evidence as recorder

    evidence = valid_wave5_hardware_evidence()

    result = recorder.main([
        "--check-only",
        "--json",
        *wave5_hardware_cli_args(evidence),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["evidence_file"] == "docs/retros/wave-5-hardware-evidence.json"
    assert "W6.F gate remains open" in payload["message"]


def test_record_wave5_hardware_check_only_json_failure_reports_no_writes(
    tmp_path,
    capsys,
):
    """W6.F check-only JSON 失败时也必须明确只读且不关闭 gate。"""
    import record_wave5_hardware_evidence as recorder

    evidence = valid_wave5_hardware_evidence()
    evidence["scale_device_ref"] = "asset://wms-staging/hardware/fake-scale-01"
    output = tmp_path / "wave-5-hardware-evidence.json"

    result = recorder.main([
        "--check-only",
        "--json",
        *wave5_hardware_cli_args(evidence, output=output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "prod/production/mock/fake/stub/example" in payload["message"]
    assert not output.exists()


def test_record_wave5_hardware_export_template_lists_materials_without_writing(
    tmp_path,
    capsys,
):
    """W6.F materials 模板只输出采集变量和 check-only 命令，不生成 evidence。"""
    import record_wave5_hardware_evidence as recorder

    output = tmp_path / "wave-5-hardware-evidence.json"

    result = recorder.main(["--export-template", "--output", str(output)])
    template = capsys.readouterr().out

    assert result == 0
    assert "WAVE_5_SCALE_DEVICE_REF=" in template
    assert "WAVE_5_BLUETOOTH_PRINTER_REF=" in template
    assert "WAVE_5_WAYBILL_PRINTER_REF=" in template
    assert "WAVE_5_AUDIT_EVENT_QUERY_REF=" in template
    assert "just wave-5-hardware-materials --from-env --json" in template
    assert "just wave-5-hardware-evidence-record --from-env --check-only --json" in template
    assert "no evidence JSON written" not in template
    assert not output.exists()


def test_record_wave5_hardware_export_template_can_be_called_from_materials_entry(
    tmp_path,
    capsys,
):
    """just wave-5-hardware-materials --export-template 会叠加 --check-only，仍必须只输出模板。"""
    import record_wave5_hardware_evidence as recorder

    output = tmp_path / "wave-5-hardware-evidence.json"

    result = recorder.main([
        "--check-only",
        "--export-template",
        "--output",
        str(output),
    ])
    template = capsys.readouterr().out

    assert result == 0
    assert "WAVE_5_STATION_CODE=" in template
    assert "WAVE_5_SCALE_READINGS_RECORDED=1" in template
    assert "W6.F gate remains open" not in template
    assert not output.exists()


def test_record_wave5_tms_check_only_validates_without_writing(tmp_path):
    """W6.G check-only 只校验字段、引用和边界，不生成 evidence JSON。"""
    import record_wave5_tms_evidence as recorder

    evidence = valid_wave5_tms_evidence()
    output = tmp_path / "wave-5-tms-evidence.json"

    result = recorder.main([
        "--check-only",
        *wave5_tms_cli_args(evidence, output=output),
    ])

    assert result == 0
    assert not output.exists()


def test_record_wave5_hardware_from_env_check_only_json_no_writes(
    tmp_path,
    capsys,
    monkeypatch,
):
    """W6.F 现场材料可从 WAVE_5_* 读取并只读预检。"""
    import record_wave5_hardware_evidence as recorder

    output = tmp_path / "wave-5-hardware-evidence.json"
    env_values = {
        "WAVE_5_ENVIRONMENT": "staging",
        "WAVE_5_STATION_CODE": "PK-STAGING-01",
        "WAVE_5_SCALE_DEVICE_REF": "asset://wms-staging/hardware/scale-01",
        "WAVE_5_BLUETOOTH_PRINTER_REF": "asset://wms-staging/hardware/bt-printer-01",
        "WAVE_5_WAYBILL_PRINTER_REF": "asset://wms-staging/hardware/waybill-printer-01",
        "WAVE_5_CALIBRATION_RECORD_REF": "s3://wms-staging-evidence/wave5/hardware/calibration.pdf",
        "WAVE_5_SCALE_READING_LOG_REF": "ci/staging/wave5-hardware-scale/123",
        "WAVE_5_BLUETOOTH_PRINT_LOG_REF": "ci/staging/wave5-hardware-bt-print/123",
        "WAVE_5_WAYBILL_PRINT_LOG_REF": "ci/staging/wave5-hardware-waybill/123",
        "WAVE_5_AUDIT_EVENT_QUERY_REF": "ci/staging/wave5-hardware-audit/123",
        "WAVE_5_SCALE_READINGS_RECORDED": "1",
        "WAVE_5_BLUETOOTH_LABELS_PRINTED": "1",
        "WAVE_5_WAYBILLS_PRINTED": "1",
        "WAVE_5_HARDWARE_CONNECTED": "true",
        "WAVE_5_PRINT_ARTIFACTS_REVIEWED": "true",
        "WAVE_5_AUDIT_EVENT_VERIFIED": "true",
    }
    for key, value in env_values.items():
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
    assert "W6.F gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave5_hardware_from_env_reports_missing_vars(capsys, monkeypatch):
    """W6.F from-env 缺材料时输出缺失变量和负责人，不写 evidence。"""
    import record_wave5_hardware_evidence as recorder

    monkeypatch.setenv("WAVE_5_ENVIRONMENT", "staging")

    result = recorder.main(["--from-env", "--check-only", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["evidence_file"] == "docs/retros/wave-5-hardware-evidence.json"
    assert "WAVE_5_SCALE_DEVICE_REF" in payload["missing_env_vars"]
    assert {
        "env_var": "WAVE_5_AUDIT_EVENT_QUERY_REF",
        "source_owner": "后端 / 数据库操作人",
        "evidence_requirement": "audit_event 查询",
    } in payload["missing_env_var_owners"]


def test_record_wave5_tms_check_only_json_reports_no_writes(tmp_path, capsys):
    """W6.G check-only JSON 必须明确不写 runtime evidence、不关闭 gate。"""
    import record_wave5_tms_evidence as recorder

    evidence = valid_wave5_tms_evidence()
    output = tmp_path / "wave-5-tms-evidence.json"

    result = recorder.main([
        "--check-only",
        "--json",
        *wave5_tms_cli_args(evidence, output=output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "W6.G gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave5_tms_from_env_check_only_json_no_writes(
    tmp_path,
    capsys,
    monkeypatch,
):
    """W6.G 现场材料可从 WAVE_5_TMS_* 读取并只读预检。"""
    import record_wave5_tms_evidence as recorder

    output = tmp_path / "wave-5-tms-evidence.json"
    env_values = {
        "WAVE_5_TMS_ENVIRONMENT": "staging",
        "WAVE_5_TMS_SYSTEM_REF": "partner://wms-staging/tms/vendor-a",
        "WAVE_5_TMS_DISPATCH_PUSH_LOG_REF": "ci/staging/wave5-tms-dispatch/123",
        "WAVE_5_TMS_CALLBACK_LOG_REF": "ci/staging/wave5-tms-callback/123",
        "WAVE_5_TMS_FAILURE_RETRY_LOG_REF": "ci/staging/wave5-tms-failure-retry/123",
        "WAVE_5_TMS_AUDIT_EVENT_QUERY_REF": "ci/staging/wave5-tms-audit/123",
        "WAVE_5_TMS_CREDENTIAL_REF": "vault://wms/staging/tms/vendor-a",
        "WAVE_5_TMS_DISPATCHES_RECEIVED": "1",
        "WAVE_5_TMS_CALLBACKS_RECEIVED": "1",
        "WAVE_5_TMS_FAILED_CALLBACKS_EXERCISED": "1",
        "WAVE_5_TMS_RETRY_SUCCEEDED": "true",
        "WAVE_5_TMS_AUDIT_EVENT_VERIFIED": "true",
    }
    for key, value in env_values.items():
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
    assert "W6.G gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave5_tms_from_env_reports_missing_vars(capsys, monkeypatch):
    """W6.G from-env 缺材料时输出缺失变量和负责人，不写 evidence。"""
    import record_wave5_tms_evidence as recorder

    monkeypatch.setenv("WAVE_5_TMS_ENVIRONMENT", "staging")

    result = recorder.main(["--from-env", "--check-only", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["evidence_file"] == "docs/retros/wave-5-tms-evidence.json"
    assert "WAVE_5_TMS_CREDENTIAL_REF" in payload["missing_env_vars"]
    assert {
        "env_var": "WAVE_5_TMS_CREDENTIAL_REF",
        "source_owner": "运维 / 安全负责人",
        "evidence_requirement": "Vault 凭证引用",
    } in payload["missing_env_var_owners"]


def test_record_wave5_tms_check_only_json_uses_relative_default_path(capsys):
    """W6.G 默认 evidence 目标在 JSON 中使用仓库相对路径，便于交接。"""
    import record_wave5_tms_evidence as recorder

    evidence = valid_wave5_tms_evidence()

    result = recorder.main([
        "--check-only",
        "--json",
        *wave5_tms_cli_args(evidence),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["evidence_file"] == "docs/retros/wave-5-tms-evidence.json"
    assert "W6.G gate remains open" in payload["message"]


def test_record_wave5_tms_check_only_json_failure_reports_no_writes(
    tmp_path,
    capsys,
):
    """W6.G check-only JSON 失败时也必须明确只读且不关闭 gate。"""
    import record_wave5_tms_evidence as recorder

    evidence = valid_wave5_tms_evidence()
    evidence["dispatch_push_log_ref"] = "ci/staging/wave5-tms-fake-dispatch/123"
    output = tmp_path / "wave-5-tms-evidence.json"

    result = recorder.main([
        "--check-only",
        "--json",
        *wave5_tms_cli_args(evidence, output=output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "prod/production/mock/fake/stub/example" in payload["message"]
    assert not output.exists()


def test_record_wave5_hardware_check_only_rejects_bad_refs_without_writing(tmp_path):
    """W6.F check-only 失败时也不能留下 evidence JSON。"""
    import record_wave5_hardware_evidence as recorder

    evidence = valid_wave5_hardware_evidence()
    evidence["scale_device_ref"] = "asset://wms-staging/hardware/fake-scale-01"
    output = tmp_path / "wave-5-hardware-evidence.json"

    result = recorder.main([
        "--check-only",
        *wave5_hardware_cli_args(evidence, output=output),
    ])

    assert result == 1
    assert not output.exists()


def test_record_wave5_tms_check_only_rejects_bad_refs_without_writing(tmp_path):
    """W6.G check-only 失败时也不能留下 evidence JSON。"""
    import record_wave5_tms_evidence as recorder

    evidence = valid_wave5_tms_evidence()
    evidence["dispatch_push_log_ref"] = "ci/staging/wave5-tms-fake-dispatch/123"
    output = tmp_path / "wave-5-tms-evidence.json"

    result = recorder.main([
        "--check-only",
        *wave5_tms_cli_args(evidence, output=output),
    ])

    assert result == 1
    assert not output.exists()


def test_validate_wave5_hardware_evidence_accepts_real_staging_payload(tmp_path):
    """Wave 5 硬件证据必须能接受真实 staging 设备和日志引用。"""
    import validate_wave5_hardware_evidence as validator

    evidence = valid_wave5_hardware_evidence()
    path = tmp_path / "wave-5-hardware-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_validate_wave5_hardware_evidence_rejects_fake_or_prod_refs(tmp_path):
    """Wave 5 硬件证据不能用 prod/production/mock/fake/stub/example 引用替代。"""
    import validate_wave5_hardware_evidence as validator

    evidence = valid_wave5_hardware_evidence()
    evidence["scale_device_ref"] = "asset://wms-prod/hardware/scale-01"
    path = tmp_path / "wave-5-hardware-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "prod/production/mock/fake/stub/example" in message

    evidence["scale_device_ref"] = "asset://wms-local/hardware/scale-01"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "local/prod/production/mock/fake/stub/example" in message

    evidence["scale_device_ref"] = "asset://wms-production/hardware/scale-01"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "prod/production/mock/fake/stub/example" in message

    evidence["scale_device_ref"] = "asset://wms-staging/hardware/fake-scale-01"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "prod/production/mock/fake/stub/example" in message

    evidence["scale_device_ref"] = "asset://wms-staging/hardware/scale-01"
    evidence["audit_event_query_ref"] = "ci/wave5-hardware-audit/123"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "environment 标记 staging" in message


def test_validate_wave5_tms_evidence_accepts_real_staging_payload(tmp_path):
    """Wave 5 TMS 证据必须能接受真实 staging 推送、回调和重试引用。"""
    import validate_wave5_tms_evidence as validator

    evidence = valid_wave5_tms_evidence()
    path = tmp_path / "wave-5-tms-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_validate_wave5_tms_evidence_rejects_fake_refs_or_plain_credentials(tmp_path):
    """Wave 5 TMS 证据必须拒绝 fake 引用和非 vault 凭证。"""
    import validate_wave5_tms_evidence as validator

    evidence = valid_wave5_tms_evidence()
    evidence["tms_system_ref"] = "partner://wms-staging/tms/fake-vendor"
    path = tmp_path / "wave-5-tms-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "prod/production/mock/fake/stub/example" in message

    evidence["tms_system_ref"] = "partner://wms-local/tms/vendor-a"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "local/prod/production/mock/fake/stub/example" in message

    evidence["tms_system_ref"] = "partner://wms-production/tms/vendor-a"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "prod/production/mock/fake/stub/example" in message

    evidence["tms_system_ref"] = "partner://wms-staging/tms/vendor-a"
    evidence["credential_ref"] = "inline-secret-token"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "vault://" in message

    evidence["credential_ref"] = "vault://wms/staging/tms/vendor-a"
    evidence["callback_log_ref"] = "ci/wave5-tms-callback/123"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "environment 标记 staging" in message
