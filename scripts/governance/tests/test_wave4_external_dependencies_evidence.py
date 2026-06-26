"""Wave 4 external dependency evidence validator 与 recorder 测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave_runtime_evidence_test_helpers import (
    valid_wave4_external_evidence,
    write_evidence,
)


def test_validate_wave4_external_dependencies_accepts_real_staging_evidence(tmp_path):
    """Wave 4 外部依赖证据必须来自真实 dev/staging 边界。"""
    import validate_wave4_external_dependencies as validator

    evidence = valid_wave4_external_evidence()
    path = tmp_path / "wave-4-external-dependencies.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_validate_wave4_external_dependencies_rejects_fake_or_secret_refs(tmp_path):
    """Wave 4 外部依赖证据不能用禁用边界引用或明文凭证替代。"""
    import validate_wave4_external_dependencies as validator

    evidence = valid_wave4_external_evidence()
    evidence["api_doc_ref"] = "s3://wms-prod-evidence/wave4/traceability/api-doc.pdf"
    evidence["credential_ref"] = "secret-inline-token"
    path = tmp_path / "wave-4-external-dependencies.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "prod/production/mock/fake/stub/example" in message

    evidence["api_doc_ref"] = "s3://wms-local-evidence/wave4/traceability/api-doc.pdf"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "local/prod/production/mock/fake/stub/example" in message

    evidence["api_doc_ref"] = "s3://wms-production-evidence/wave4/traceability/api-doc.pdf"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "prod/production/mock/fake/stub/example" in message

    evidence["api_doc_ref"] = "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "vault://" in message

    evidence["credential_ref"] = "vault://wms/staging/traceability/masxf"
    evidence["success_report_log_ref"] = "ci/wave4-traceability-success/123"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "environment 标记 staging" in message


def test_record_wave4_external_dependencies_writes_valid_evidence(tmp_path):
    """记录脚本生成的 evidence 必须能被同一 validator 接受。"""
    import record_wave4_external_dependencies as recorder
    import validate_wave4_external_dependencies as validator

    output = tmp_path / "wave-4-external-dependencies.json"

    assert recorder.main([
        "--output", str(output),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
    ]) == 0

    ok, message = validator.validate_one(output, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_record_wave4_external_dependencies_rejects_invalid_refs_before_write(tmp_path):
    """记录脚本不能把禁用边界证据写入仓库。"""
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"

    assert recorder.main([
        "--output", str(output),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-prod-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
    ]) == 1

    assert not output.exists()


def test_record_wave4_external_dependencies_check_only_validates_without_writing(
    tmp_path,
):
    """W6.E check-only 只校验外部依赖材料，不生成正式 evidence。"""
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"

    assert recorder.main([
        "--check-only",
        "--output", str(output),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
    ]) == 0

    assert not output.exists()


def test_record_wave4_external_dependencies_check_only_json_reports_no_writes(
    tmp_path,
    capsys,
):
    """W6.E check-only JSON 必须明确不写 runtime evidence、不关闭 gate。"""
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"

    result = recorder.main([
        "--check-only",
        "--json",
        "--output", str(output),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "W6.E gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave4_external_dependencies_check_only_json_reports_validation_failure(
    tmp_path,
    capsys,
):
    """W6.E check-only JSON 失败路径也必须输出 JSON 且不写 evidence。"""
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"

    result = recorder.main([
        "--check-only",
        "--json",
        "--output", str(output),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-prod-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
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
    assert "prod" in payload["message"]
    assert captured.err == ""
    assert not output.exists()


def test_record_wave4_external_dependencies_check_only_rejects_invalid_refs_without_writing(
    tmp_path,
):
    """W6.E check-only 失败时也不能留下正式外部依赖 evidence。"""
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"

    assert recorder.main([
        "--check-only",
        "--output", str(output),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-prod-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
    ]) == 1

    assert not output.exists()


def test_record_wave4_external_dependencies_requires_force_to_overwrite(tmp_path):
    """已存在 evidence 时必须显式 --force 才能覆盖。"""
    import record_wave4_external_dependencies as recorder

    output = tmp_path / "wave-4-external-dependencies.json"
    output.write_text("{}", encoding="utf-8")

    args = [
        "--output", str(output),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
    ]

    assert recorder.main(args) == 1
    assert output.read_text(encoding="utf-8") == "{}"

    assert recorder.main([*args, "--force"]) == 0
    assert json.loads(output.read_text(encoding="utf-8"))["platform"] == "码上放心"


def test_wave4_external_dependencies_readiness_accepts_valid_staging_refs(capsys):
    """readiness 只校验真实 dev/staging 证据材料，不写 evidence。"""
    import check_wave4_external_dependencies_readiness as readiness

    assert readiness.main([
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
        "--json",
    ]) == 0

    payload = json.loads(capsys.readouterr().out)
    assert payload["ok"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-4-external-dependencies.json"
    assert (
        "just wave-4-external-dependencies-record --from-env --check-only --json"
        in payload["next_commands"]
    )
    assert (
        "just wave-4-external-dependencies-record --from-env --json"
        in payload["next_commands"]
    )
    assert "just wave-4-external-dependencies-validate" in payload["next_commands"]
    assert payload["evidence_scope"] == {
        "platform_source": "码上放心",
        "environment": "staging",
        "scope_verified": True,
        "internal_api_rejected": True,
    }
    assert payload["evidence_items"]["api_doc"]["env_var"] == (
        "WAVE_4_EXTERNAL_API_DOC_REF"
    )
    assert payload["evidence_items"]["success_log"]["owner"] == (
        "测试执行人 / 后端负责人"
    )
    assert payload["proof"]["success_case_count"] == 1
    assert payload["proof"]["failure_case_count"] == 1
    assert payload["proof"]["pending_replay_queue_verified"] is True


def test_wave4_external_dependencies_readiness_from_env_accepts_valid_refs(
    capsys,
    monkeypatch,
):
    """readiness 可从 WAVE_4_EXTERNAL_* 读取现场材料，且保持只读。"""
    import check_wave4_external_dependencies_readiness as readiness

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

    assert readiness.main(["--from-env", "--json"]) == 0

    payload = json.loads(capsys.readouterr().out)
    assert payload["ok"] is True
    assert payload["mode"] == "readiness"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False


def test_wave4_external_dependencies_readiness_from_env_reports_missing_vars(
    capsys,
    monkeypatch,
):
    """readiness from-env 缺材料时返回缺失变量和负责人。"""
    import check_wave4_external_dependencies_readiness as readiness

    monkeypatch.setenv("WAVE_4_EXTERNAL_ENVIRONMENT", "staging")

    assert readiness.main(["--from-env", "--json"]) == 1

    payload = json.loads(capsys.readouterr().out)
    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert "WAVE_4_EXTERNAL_API_DOC_REF" in payload["missing_env_vars"]
    assert {
        "env_var": "WAVE_4_EXTERNAL_CREDENTIAL_REF",
        "source_owner": "运维 / 安全负责人",
        "evidence_requirement": "Vault 凭证引用",
    } in payload["missing_env_var_owners"]


def test_wave4_external_dependencies_readiness_rejects_invalid_refs_before_write(tmp_path):
    """readiness 不能把 prod/local/stub 等引用误判为可关闭 gate。"""
    import check_wave4_external_dependencies_readiness as readiness

    evidence_file = tmp_path / "wave-4-external-dependencies.json"

    assert readiness.main([
        "--evidence-file", str(evidence_file),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-prod-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
        "--pending-replay-queue-verified",
    ]) == 1

    assert not evidence_file.exists()


def test_wave4_external_dependencies_rejects_wms_internal_trace_code_refs():
    """W6.E 不能把 WMS 自己的追溯码查询接口当成码上放心 evidence。"""
    import validate_wave4_external_dependencies as validator

    payload = {
        "environment": "staging",
        "platform": "码上放心",
        "api_doc_ref": "internal://wms-staging/wms-api/openapi/wms-openapi.yaml",
        "auth_doc_ref": "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "error_code_doc_ref": "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "rate_limit_doc_ref": "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "credential_ref": "vault://wms/staging/traceability/masxf",
        "success_report_log_ref": "https://wms-staging.internal/api/codes/87004720000000005994",
        "failure_retry_log_ref": "ci/staging/wave4-traceability-retry/123",
        "audit_event_query_ref": "ci/staging/wave4-traceability-audit/123",
        "reported_events": 1,
        "failed_events_exercised": 1,
        "pending_replay_queue_verified": True,
    }

    ok, message = validator.validate_wave4_external_dependency_payload(payload)

    assert ok is False
    assert "WMS 内部追溯码接口" in message
    assert "api_doc_ref" in message
    assert "success_report_log_ref" in message


def test_wave4_external_dependencies_readiness_rejects_missing_inputs_without_write(
    tmp_path,
):
    """缺少必需材料时 readiness 失败且不创建 evidence JSON。"""
    import check_wave4_external_dependencies_readiness as readiness

    evidence_file = tmp_path / "wave-4-external-dependencies.json"

    assert readiness.main([
        "--evidence-file", str(evidence_file),
        "--environment", "staging",
        "--api-doc-ref", "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "--auth-doc-ref", "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "--credential-ref", "vault://wms/staging/traceability/masxf",
        "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
        "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
        "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
        "--reported-events", "1",
        "--failed-events-exercised", "1",
    ]) == 1

    assert not evidence_file.exists()
