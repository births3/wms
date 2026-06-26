"""Wave 6 deploy runtime evidence validator tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave_runtime_evidence_test_helpers import (
    valid_wave6_deploy_evidence,
    write_evidence,
)


def wave6_deploy_cli_args(
    evidence: dict[str, object],
    *,
    output: Path | None = None,
) -> list[str]:
    """Build recorder CLI args from the shared valid Wave 6 deploy fixture."""
    args = []
    if output is not None:
        args.extend(["--output", str(output)])
    args.extend([
        "--environment",
        str(evidence["environment"]),
        "--deployment-mode",
        str(evidence["deployment_mode"]),
        "--release-version",
        str(evidence["release_version"]),
        "--release-plan-ref",
        str(evidence["release_plan_ref"]),
        "--artifact-ref",
        str(evidence["artifact_ref"]),
        "--canary-config-ref",
        str(evidence["canary_config_ref"]),
        "--smoke-gate-ref",
        str(evidence["smoke_gate_ref"]),
        "--observability-dashboard-ref",
        str(evidence["observability_dashboard_ref"]),
        "--rollback-drill-log-ref",
        str(evidence["rollback_drill_log_ref"]),
        "--approval-record-ref",
        str(evidence["approval_record_ref"]),
        "--audit-event-query-ref",
        str(evidence["audit_event_query_ref"]),
        "--canary-stages-exercised",
        str(evidence["canary_stages_exercised"]),
        "--smoke-checks-passed",
        str(evidence["smoke_checks_passed"]),
        "--rollback-drills-exercised",
        str(evidence["rollback_drills_exercised"]),
    ])
    if evidence["canary_used"]:
        args.append("--canary-used")
    if evidence["full_release_blocked"]:
        args.append("--full-release-blocked")
    if evidence["rollback_verified"]:
        args.append("--rollback-verified")
    if evidence["audit_event_verified"]:
        args.append("--audit-event-verified")
    if evidence["dual_approval_recorded"]:
        args.append("--dual-approval-recorded")
    return args


def wave6_deploy_env(evidence: dict[str, object]) -> dict[str, str]:
    """Build WAVE_6_* env vars from the shared valid Wave 6 deploy fixture."""
    return {
        "WAVE_6_ENVIRONMENT": str(evidence["environment"]),
        "WAVE_6_DEPLOYMENT_MODE": str(evidence["deployment_mode"]),
        "WAVE_6_RELEASE_VERSION": str(evidence["release_version"]),
        "WAVE_6_RELEASE_PLAN_REF": str(evidence["release_plan_ref"]),
        "WAVE_6_ARTIFACT_REF": str(evidence["artifact_ref"]),
        "WAVE_6_CANARY_CONFIG_REF": str(evidence["canary_config_ref"]),
        "WAVE_6_SMOKE_GATE_REF": str(evidence["smoke_gate_ref"]),
        "WAVE_6_OBSERVABILITY_DASHBOARD_REF": str(
            evidence["observability_dashboard_ref"],
        ),
        "WAVE_6_ROLLBACK_DRILL_LOG_REF": str(evidence["rollback_drill_log_ref"]),
        "WAVE_6_APPROVAL_RECORD_REF": str(evidence["approval_record_ref"]),
        "WAVE_6_AUDIT_EVENT_QUERY_REF": str(evidence["audit_event_query_ref"]),
        "WAVE_6_CANARY_STAGES_EXERCISED": str(evidence["canary_stages_exercised"]),
        "WAVE_6_SMOKE_CHECKS_PASSED": str(evidence["smoke_checks_passed"]),
        "WAVE_6_ROLLBACK_DRILLS_EXERCISED": str(
            evidence["rollback_drills_exercised"],
        ),
        "WAVE_6_CANARY_USED": str(evidence["canary_used"]).lower(),
        "WAVE_6_FULL_RELEASE_BLOCKED": str(evidence["full_release_blocked"]).lower(),
        "WAVE_6_ROLLBACK_VERIFIED": str(evidence["rollback_verified"]).lower(),
        "WAVE_6_AUDIT_EVENT_VERIFIED": str(evidence["audit_event_verified"]).lower(),
        "WAVE_6_DUAL_APPROVAL_RECORDED": str(
            evidence["dual_approval_recorded"],
        ).lower(),
    }


def test_validate_wave6_deploy_evidence_accepts_real_staging_payload(tmp_path):
    """Wave 6 灰度发布证据必须接受真实 staging 灰度、回滚和审批引用。"""
    import validate_wave6_deploy_evidence as validator

    evidence = valid_wave6_deploy_evidence()
    path = tmp_path / "wave-6-deploy-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_validate_wave6_deploy_evidence_rejects_blocked_refs_or_full_release(tmp_path):
    """Wave 6 灰度发布证据必须拒绝禁用边界引用和全量直发。"""
    import validate_wave6_deploy_evidence as validator

    evidence = valid_wave6_deploy_evidence()
    path = tmp_path / "wave-6-deploy-evidence.json"

    for token in ("local", "dev", "prod", "production", "mock", "fake", "stub", "example"):
        evidence["release_plan_ref"] = (
            f"s3://wms-{token}-evidence/wave6/deploy/release-plan.md"
        )
        write_evidence(path, evidence)

        ok, message = validator.validate_one(path, allow_example_refs=False)

        assert ok is False
        assert "local/dev/prod/production/mock/fake/stub/example" in message

    evidence["release_plan_ref"] = "s3://wms-staging-evidence/wave6/deploy/release-plan.md"
    evidence["full_release_blocked"] = False
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "full_release_blocked" in message

    evidence["full_release_blocked"] = True
    evidence["approval_record_ref"] = "ticket://release-approval/WMS-20260604"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "environment 标记 staging" in message


def test_validate_wave6_deploy_evidence_reports_blocked_ref_field_name(tmp_path):
    """W6.H blocked ref 错误应指出具体字段，方便现场补材料。"""
    import validate_wave6_deploy_evidence as validator

    evidence = valid_wave6_deploy_evidence()
    evidence["artifact_ref"] = "docker/staging/wms-api-staging/latest-local-image"
    evidence["approval_record_ref"] = "ticket://wms-local-staging/release-approval/WMS-20260608"
    path = tmp_path / "wave-6-deploy-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "local/dev/prod/production/mock/fake/stub/example" in message
    assert "artifact_ref" in message
    assert "approval_record_ref" in message


def test_validate_wave6_deploy_evidence_rejects_dev_environment(tmp_path):
    """W6.H 是首次试运行 staging 灰度发布 gate，不能用 dev evidence 关闭。"""
    import validate_wave6_deploy_evidence as validator

    evidence = valid_wave6_deploy_evidence()
    evidence["environment"] = "dev"
    for key in (
        "release_plan_ref",
        "artifact_ref",
        "canary_config_ref",
        "smoke_gate_ref",
        "observability_dashboard_ref",
        "rollback_drill_log_ref",
        "approval_record_ref",
        "audit_event_query_ref",
    ):
        evidence[key] = str(evidence[key]).replace("staging", "dev")
    path = tmp_path / "wave-6-deploy-evidence.json"
    write_evidence(path, evidence)

    ok, message = validator.validate_one(path, allow_example_refs=False)

    assert ok is False
    assert "environment 必须是 staging" in message


def test_record_wave6_deploy_evidence_check_only_validates_without_writing(
    tmp_path,
):
    """W6.H check-only 只校验灰度发布 evidence 字段，不生成正式 JSON。"""
    import record_wave6_deploy_evidence as recorder

    evidence = valid_wave6_deploy_evidence()
    output = tmp_path / "wave-6-deploy-evidence.json"

    assert recorder.main([
        "--check-only",
        *wave6_deploy_cli_args(evidence, output=output),
    ]) == 0

    assert not output.exists()


def test_record_wave6_deploy_evidence_check_only_json_reports_no_writes(
    tmp_path,
    capsys,
):
    """W6.H check-only JSON 必须明确不写 runtime evidence、不关闭 gate。"""
    import json
    import record_wave6_deploy_evidence as recorder

    evidence = valid_wave6_deploy_evidence()
    output = tmp_path / "wave-6-deploy-evidence.json"

    result = recorder.main([
        "--check-only",
        "--json",
        *wave6_deploy_cli_args(evidence, output=output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "W6.H gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave6_deploy_evidence_check_only_json_uses_relative_default_path(
    capsys,
):
    """W6.H 默认 evidence 目标在 JSON 中使用仓库相对路径，便于交接。"""
    import json
    import record_wave6_deploy_evidence as recorder

    evidence = valid_wave6_deploy_evidence()

    result = recorder.main([
        "--check-only",
        "--json",
        *wave6_deploy_cli_args(evidence),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 0
    assert payload["ok"] is True
    assert payload["evidence_file"] == "docs/retros/wave-6-deploy-evidence.json"
    assert "W6.H gate remains open" in payload["message"]


def test_record_wave6_deploy_evidence_from_env_check_only_json_no_writes(
    tmp_path,
    capsys,
    monkeypatch,
):
    """W6.H evidence record 应能从 WAVE_6_* 读取材料并只读预检。"""
    import json
    import record_wave6_deploy_evidence as recorder

    evidence = valid_wave6_deploy_evidence()
    output = tmp_path / "wave-6-deploy-evidence.json"
    for key, value in wave6_deploy_env(evidence).items():
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
    assert "W6.H gate remains open" in payload["message"]
    assert not output.exists()


def test_record_wave6_deploy_evidence_from_env_reports_missing_vars(
    capsys,
    monkeypatch,
):
    """W6.H evidence record from-env 缺材料时输出缺失变量，不写 evidence。"""
    import json
    import record_wave6_deploy_evidence as recorder

    monkeypatch.setenv("WAVE_6_ENVIRONMENT", "staging")

    result = recorder.main(["--from-env", "--check-only", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-6-deploy-evidence.json"
    assert "WAVE_6_RELEASE_PLAN_REF" in payload["missing_env_vars"]
    assert "WAVE_6_AUDIT_EVENT_QUERY_REF" in payload["missing_env_vars"]


def test_record_wave6_deploy_evidence_check_only_json_failure_reports_no_writes(
    tmp_path,
    capsys,
):
    """W6.H check-only JSON 失败时也必须明确只读且不关闭 gate。"""
    import json
    import record_wave6_deploy_evidence as recorder

    evidence = valid_wave6_deploy_evidence()
    evidence["release_plan_ref"] = "s3://wms-prod-evidence/wave6/deploy/release-plan.md"
    output = tmp_path / "wave-6-deploy-evidence.json"

    result = recorder.main([
        "--check-only",
        "--json",
        *wave6_deploy_cli_args(evidence, output=output),
    ])
    payload = json.loads(capsys.readouterr().out)

    assert result == 1
    assert payload["ok"] is False
    assert payload["check_only"] is True
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == str(output)
    assert "local/dev/prod/production/mock/fake/stub/example" in payload["message"]
    assert not output.exists()


def test_record_wave6_deploy_evidence_check_only_rejects_bad_refs_without_writing(
    tmp_path,
):
    """W6.H check-only 失败时也不能留下正式灰度发布 evidence。"""
    import record_wave6_deploy_evidence as recorder

    evidence = valid_wave6_deploy_evidence()
    evidence["release_plan_ref"] = "s3://wms-prod-evidence/wave6/deploy/release-plan.md"
    output = tmp_path / "wave-6-deploy-evidence.json"

    assert recorder.main([
        "--check-only",
        *wave6_deploy_cli_args(evidence, output=output),
    ]) == 1

    assert not output.exists()


def test_wave6_closeout_lists_deploy_audit_check_only_before_write():
    """W6.H closeout 必须先跑 deploy audit check-only，再正式写 audit_event。"""
    closeout = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")

    check_only_index = closeout.index("just wave-6-deploy-audit --from-env --check-only")
    write_index = closeout.index("just wave-6-deploy-audit --from-env\n")

    assert check_only_index < write_index
