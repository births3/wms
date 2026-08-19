"""Wave 4 外部依赖 evidence 模板与 check-only 缺参校验。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


def test_record_wave4_external_dependencies_export_template_lists_materials_without_writing(tmp_path, capsys):
    """显式 --export-template 仅输出变量清单与 check-only 命令。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"
    result = recorder.main(["--export-template", "--output", str(output)])
    template = capsys.readouterr().out

    assert result == 0
    assert "WAVE_4_EXTERNAL_ENVIRONMENT=" in template
    assert "WAVE_4_EXTERNAL_API_DOC_REF=" in template
    assert "WAVE_4_EXTERNAL_AUTH_DOC_REF=" in template
    assert "WAVE_4_EXTERNAL_ERROR_CODE_DOC_REF=" in template
    assert "WAVE_4_EXTERNAL_RATE_LIMIT_DOC_REF=" in template
    assert "WAVE_4_EXTERNAL_CREDENTIAL_REF=" in template
    assert "WAVE_4_EXTERNAL_SUCCESS_REPORT_LOG_REF=" in template
    assert "WAVE_4_EXTERNAL_FAILURE_RETRY_LOG_REF=" in template
    assert "WAVE_4_EXTERNAL_AUDIT_EVENT_QUERY_REF=" in template
    assert "WAVE_4_EXTERNAL_REPORTED_EVENTS=" in template
    assert "WAVE_4_EXTERNAL_FAILED_EVENTS_EXERCISED=" in template
    assert "WAVE_4_EXTERNAL_PENDING_REPLAY_QUEUE_VERIFIED=" in template
    assert (
        "just wave-4-external-dependencies-record --from-env --check-only --json"
        in template
    )
    assert "--environment \"$WAVE_4_EXTERNAL_ENVIRONMENT\"" not in template
    assert "W6.E gate remains open" not in template
    assert not output.exists()


def test_record_wave4_external_dependencies_export_template_can_be_called_with_check_only(
    tmp_path,
    capsys,
):
    """"--export-template 与 --check-only 同时出现时仍返回模板，不做参数校验。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"
    result = recorder.main(["--check-only", "--export-template", "--output", str(output)])
    template = capsys.readouterr().out

    assert result == 0
    assert (
        "just wave-4-external-dependencies-record --from-env --check-only --json"
        in template
    )
    assert "the following arguments are required" not in template
    assert not output.exists()


def test_record_wave4_external_dependencies_check_only_without_required_args_fails(capsys):
    """"--check-only" 缺参数不能静默通过（也不能输出模板）。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave4_external_dependencies as recorder

    with pytest.raises(SystemExit) as excinfo:
        recorder.main(["--check-only"])
    captured = capsys.readouterr()

    assert excinfo.value.code == 2
    assert "the following arguments are required" in captured.err


def test_record_wave4_external_dependencies_check_only_json_no_writes_when_valid(tmp_path, capsys):
    """check-only --json 成功时也要报告不写 runtime evidence 与 gate 未关闭。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"
    result = recorder.main(
        [
            "--check-only",
            "--json",
            "--output",
            str(output),
            "--environment",
            "staging",
            "--api-doc-ref",
            "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
            "--auth-doc-ref",
            "s3://wms-staging-evidence/wave4/traceability/auth.md",
            "--error-code-doc-ref",
            "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
            "--rate-limit-doc-ref",
            "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
            "--credential-ref",
            "vault://wms/staging/traceability/masxf",
            "--success-report-log-ref",
            "ci/staging/wave4-traceability-success/123",
            "--failure-retry-log-ref",
            "ci/staging/wave4-traceability-retry/123",
            "--audit-event-query-ref",
            "ci/staging/wave4-traceability-audit/123",
            "--reported-events",
            "1",
            "--failed-events-exercised",
            "1",
            "--pending-replay-queue-verified",
        ]
    )
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "W6.E gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave4_external_dependencies_from_env_check_only_json_no_writes(
    tmp_path,
    capsys,
    monkeypatch,
):
    """W6.E 现场采集链可从 WAVE_4_EXTERNAL_* 读取材料并只读预检。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"
    env_values = {
        "WAVE_4_EXTERNAL_ENVIRONMENT": "staging",
        "WAVE_4_EXTERNAL_API_DOC_REF": "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "WAVE_4_EXTERNAL_AUTH_DOC_REF": "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "WAVE_4_EXTERNAL_ERROR_CODE_DOC_REF": "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "WAVE_4_EXTERNAL_RATE_LIMIT_DOC_REF": "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "WAVE_4_EXTERNAL_CREDENTIAL_REF": "vault://wms/staging/traceability/masxf",
        "WAVE_4_EXTERNAL_SUCCESS_REPORT_LOG_REF": "ci/staging/wave4-traceability-success/123",
        "WAVE_4_EXTERNAL_FAILURE_RETRY_LOG_REF": "ci/staging/wave4-traceability-retry/123",
        "WAVE_4_EXTERNAL_AUDIT_EVENT_QUERY_REF": "ci/staging/wave4-traceability-audit/123",
        "WAVE_4_EXTERNAL_REPORTED_EVENTS": "1",
        "WAVE_4_EXTERNAL_FAILED_EVENTS_EXERCISED": "1",
        "WAVE_4_EXTERNAL_PENDING_REPLAY_QUEUE_VERIFIED": "true",
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
    assert "W6.E gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave4_external_dependencies_from_env_reports_missing_vars(
    tmp_path,
    capsys,
    monkeypatch,
):
    """W6.E from-env 缺材料时输出缺失变量和负责人，不写 evidence。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"
    monkeypatch.setenv("WAVE_4_EXTERNAL_ENVIRONMENT", "staging")

    result = recorder.main([
        "--from-env",
        "--check-only",
        "--json",
        "--output",
        str(output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "WAVE_4_EXTERNAL_API_DOC_REF" in payload["missing_env_vars"]
    assert "WAVE_4_EXTERNAL_SUCCESS_REPORT_LOG_REF" in payload["missing_env_vars"]
    assert {
        "env_var": "WAVE_4_EXTERNAL_API_DOC_REF",
        "source_owner": "业务方 / 平台对接负责人",
        "evidence_requirement": "正式接口文档归档",
    } in payload["missing_env_var_owners"]
    assert not output.exists()


def test_record_wave4_external_dependencies_from_env_missing_vars_uses_relative_default_path(
    capsys,
    monkeypatch,
):
    """W6.E 默认 evidence 目标在 JSON 中使用仓库相对路径，便于交接。"""
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import record_wave4_external_dependencies as recorder

    monkeypatch.setenv("WAVE_4_EXTERNAL_ENVIRONMENT", "staging")

    result = recorder.main(["--from-env", "--check-only", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["evidence_file"] == "docs/retros/wave-4-external-dependencies.json"
    assert "WAVE_4_EXTERNAL_API_DOC_REF" in payload["missing_env_vars"]


def test_wave4_external_runbook_uses_export_template_variables():
    """runbook 示例必须和导出模板的变量前缀一致，避免现场照抄传空。"""
    text = Path("docs/runbooks/wave-4-external-dependencies.md").read_text(
        encoding="utf-8",
    )

    assert "just wave-4-external-dependencies-record --export-template" in text
    assert "source \"$WAVE4_EXTERNAL_TEMPLATE\"" not in text
    assert "$WAVE_4_API_DOC_REF" not in text
    assert "$WAVE_4_EXTERNAL_API_DOC_REF" in text
    assert "$WAVE_4_EXTERNAL_AUDIT_EVENT_QUERY_REF" in text
