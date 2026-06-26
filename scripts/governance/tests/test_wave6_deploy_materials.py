"""Wave 6 gray release materials worksheet tests."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _valid_env() -> dict[str, str]:
    return {
        "WAVE_6_ENVIRONMENT": "staging",
        "WAVE_6_DEPLOYMENT_MODE": "docker-compose",
        "WAVE_6_RELEASE_VERSION": "wms-api-20260607.1",
        "WAVE_6_SERVICE_URL": "http://wms-staging.internal",
        "WAVE_6_RELEASE_PLAN_REF": "ticket://wms-staging-release-plan/W6H-20260607",
        "WAVE_6_ARTIFACT_REF": "registry://wms-staging/wms-api@sha256:abcdef1234567890",
        "WAVE_6_CANARY_CONFIG_REF": "git://wms-staging/deploy/canary/W6H-20260607",
        "WAVE_6_SMOKE_GATE_REF": "ci/staging/wave6-deploy-smoke/123",
        "WAVE_6_OBSERVABILITY_DASHBOARD_REF": "grafana/staging/wms/wave6-deploy/123",
        "WAVE_6_ROLLBACK_DRILL_LOG_REF": "ci/staging/wave6-deploy-rollback/123",
        "WAVE_6_APPROVAL_RECORD_REF": "ticket://wms-staging-release-approval/W6H-20260607",
        "WAVE_6_AUDIT_EVENT_QUERY_REF": "postgres://wms-staging/audit_event/deploy/W6H-20260607",
        "WAVE_6_DEPLOY_MODULE": "W6.H",
        "WAVE_6_DEPLOY_ACTION": "deploy.gray_release.recorded",
        "WAVE_6_DEPLOY_RESOURCE_TYPE": "deployment_release",
        "WAVE_6_DEPLOY_RESOURCE_ID": "W6H:staging:wms-api-20260607.1",
        "WAVE_6_DEPLOY_ACTOR_ID": "11111111-1111-4111-8111-111111111111",
        "WAVE_6_DEPLOY_ACTOR_NAME": "release-operator",
        "WAVE_6_DEPLOY_OWNER_ID": "22222222-2222-4222-8222-222222222222",
        "WAVE_6_DEPLOY_JTI": "deploy-staging-W6H-20260607-1",
        "WAVE_6_CANARY_STAGES_EXERCISED": "1",
        "WAVE_6_SMOKE_CHECKS_PASSED": "3",
        "WAVE_6_ROLLBACK_DRILLS_EXERCISED": "1",
        "WAVE_6_CANARY_USED": "true",
        "WAVE_6_FULL_RELEASE_BLOCKED": "true",
        "WAVE_6_ROLLBACK_VERIFIED": "true",
        "WAVE_6_AUDIT_EVENT_VERIFIED": "true",
        "WAVE_6_DUAL_APPROVAL_RECORDED": "true",
    }


def test_wave6_deploy_materials_reports_missing_env_without_writing_evidence(capsys):
    """materials worksheet 缺变量时必须只报告缺口，不写 runtime evidence。"""
    import report_wave6_deploy_materials as materials

    assert materials.main(["--json"], env={}) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-6-deploy-evidence.json"
    assert "WAVE_6_RELEASE_PLAN_REF" in payload["missing_env_vars"]
    assert "WAVE_6_ARTIFACT_REF" in payload["missing_env_vars"]
    assert "WAVE_6_DUAL_APPROVAL_RECORDED" in payload["missing_env_vars"]
    assert "WAVE_6_DEPLOY_ACTOR_ID" in payload["missing_env_vars"]
    assert "WAVE_6_DEPLOY_JTI" in payload["missing_env_vars"]


def test_wave6_deploy_materials_accepts_complete_staging_refs(capsys):
    """外部材料齐备时，worksheet 应输出 readiness / record / validate 命令模板。"""
    import report_wave6_deploy_materials as materials

    assert materials.main(["--json"], env=_valid_env()) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["missing_env_vars"] == []
    assert payload["invalid_env_vars"] == []
    assert payload["validator_message"] == "Wave 6 deploy evidence 内容有效"
    assert payload["readiness_command"] == "just wave-6-deploy-readiness --from-env --json"
    assert payload["record_check_only_command"] == (
        "just wave-6-deploy-evidence-record --from-env --check-only --json"
    )
    assert "--check-only" in payload["record_check_only_command"]
    assert "--json" in payload["record_check_only_command"]
    assert payload["record_command"] == "just wave-6-deploy-evidence-record --from-env --json"
    assert payload["validate_command"] == "just wave-6-deploy-evidence-validate"
    assert _valid_env()["WAVE_6_RELEASE_PLAN_REF"] not in payload["record_command"]
    assert payload["deploy_audit_check_only_command"] == (
        "just wave-6-deploy-audit --from-env --check-only"
    )
    assert "--check-only" in payload["deploy_audit_check_only_command"]
    assert payload["deploy_audit_record_command"] == "just wave-6-deploy-audit --from-env"
    assert "--check-only" not in payload["deploy_audit_record_command"]


def test_wave6_deploy_materials_outputs_ordered_execution_plan(capsys):
    """materials worksheet 应明确 audit 写入先于依赖 audit_event_query_ref 的 readiness。"""
    import report_wave6_deploy_materials as materials

    assert materials.main(["--json"], env=_valid_env()) == 0
    payload = json.loads(capsys.readouterr().out)
    plan = payload["execution_plan"]

    assert [step["step"] for step in plan] == [
        "materials",
        "deploy_audit_check_only",
        "deploy_audit_record",
        "readiness",
        "evidence_record_check_only",
        "evidence_record",
        "validate",
    ]
    assert plan[1]["writes_audit_event"] is False
    assert plan[2]["writes_audit_event"] is True
    assert plan[3]["writes_runtime_evidence"] is False
    assert plan[5]["writes_runtime_evidence"] is True
    assert plan[6]["closes_gate"] is False
    assert plan[0]["requires_env"] == []
    assert "WAVE_6_AUDIT_EVENT_QUERY_REF" in plan[0]["checks_env"]
    assert "WAVE_6_AUDIT_EVENT_QUERY_REF" in plan[3]["requires_env"]


def test_wave6_deploy_materials_groups_missing_env_by_execution_stage(capsys):
    """materials JSON 必须区分 audit 前材料和 audit 后材料。"""
    import report_wave6_deploy_materials as materials

    assert materials.main(["--json"], env={}) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["missing_env_by_stage"]["pre_audit"] == [
        step
        for step in payload["execution_plan"][1]["requires_env"]
        if step != "WAVE_6_AUDIT_EVENT_QUERY_REF"
    ]
    assert "WAVE_6_AUDIT_EVENT_QUERY_REF" not in payload["missing_env_by_stage"]["pre_audit"]
    assert "WAVE_6_AUDIT_EVENT_QUERY_REF" in payload["missing_env_by_stage"]["post_audit"]
    assert payload["next_blocking_stage"] == "pre_audit"

    env = _valid_env()
    env["WAVE_6_AUDIT_EVENT_QUERY_REF"] = ""

    assert materials.main(["--json"], env=env) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["missing_env_by_stage"]["pre_audit"] == []
    assert payload["missing_env_by_stage"]["post_audit"] == ["WAVE_6_AUDIT_EVENT_QUERY_REF"]
    assert payload["next_blocking_stage"] == "post_audit"


def test_wave6_deploy_materials_text_prints_stage_summary(capsys):
    """文本模式必须优先提示下一阻塞阶段，避免执行人只看到全量缺口。"""
    import report_wave6_deploy_materials as materials

    assert materials.main([], env={}) == 1
    captured = capsys.readouterr()
    output = captured.out + captured.err

    assert "next blocking stage: pre_audit" in output
    assert "pre_audit missing:" in output
    assert "post_audit missing:" in output
    assert "do not fake WAVE_6_AUDIT_EVENT_QUERY_REF before deploy audit" in output
    assert "missing: WAVE_6_AUDIT_EVENT_QUERY_REF" in output


def test_wave6_deploy_materials_text_prints_stage_summary_before_full_missing_list(
    capsys,
):
    """文本模式必须先给执行阶段，再打印全量缺失变量。"""
    import report_wave6_deploy_materials as materials

    assert materials.main([], env={}) == 1
    captured = capsys.readouterr()
    output = captured.err

    assert output.index("next blocking stage: pre_audit") < output.index(
        "missing: WAVE_6_SERVICE_URL",
    )


def test_wave6_deploy_materials_rejects_invalid_deploy_actor_uuid(capsys):
    """deploy audit 所需 H1 actor / owner UUID 不合法时，materials 应提前提示。"""
    import report_wave6_deploy_materials as materials

    env = _valid_env()
    env["WAVE_6_DEPLOY_ACTOR_ID"] = "not-a-uuid"

    assert materials.main(["--json"], env=env) == 1
    payload = json.loads(capsys.readouterr().out)

    assert "WAVE_6_DEPLOY_ACTOR_ID must be a UUID" in payload["invalid_env_vars"]


def test_wave6_deploy_materials_export_template_lists_non_secret_sources(capsys):
    """materials worksheet 应能输出不含密钥的 export 模板和来源说明。"""
    import report_wave6_deploy_materials as materials

    assert materials.main(["--export-template"], env={}) == 0
    output = capsys.readouterr().out

    assert "Wave 6 deploy materials export template" in output
    assert "does not contain secrets" in output
    assert "export WAVE_6_RELEASE_PLAN_REF=" in output
    assert "export WAVE_6_DEPLOY_ACTOR_ID=" in output
    assert "# source: release ticket / operations evidence store" in output
    assert "# source: H1 release actor UUID" in output
    assert "sk-" not in output
    assert "WAVE_6_AUDIT_EVENT_QUERY_REF" in output
    assert "from wave-6-deploy-audit output" in output
    assert "leave blank until deploy_audit_record succeeds" in output
    assert "just wave-6-deploy-readiness --from-env --json" in output
    assert "just wave-6-deploy-evidence-record --from-env --check-only --json" in output


def test_wave6_deploy_runbook_documents_export_template():
    """W6.H runbook 必须提示先生成非密钥 export 模板。"""
    runbook = Path("docs/runbooks/wave-6-deploy-evidence.md").read_text(
        encoding="utf-8",
    )

    assert "just wave-6-deploy-materials --export-template" in runbook
    assert "不包含密钥" in runbook


def test_wave6_deploy_materials_rejects_invalid_flags(capsys):
    """布尔确认不是 true 时，不能进入 record。"""
    import report_wave6_deploy_materials as materials

    env = _valid_env()
    env["WAVE_6_CANARY_USED"] = "false"

    assert materials.main(["--json"], env=env) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert "WAVE_6_CANARY_USED must be true" in payload["invalid_env_vars"]


def test_wave6_deploy_materials_rejects_dev_environment(capsys):
    """W6.H materials worksheet 必须提示该 gate 只能用 staging 灰度发布材料。"""
    import report_wave6_deploy_materials as materials

    env = {
        key: value.replace("staging", "dev").replace("wms-staging", "wms-dev")
        for key, value in _valid_env().items()
    }

    assert materials.main(["--json"], env=env) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert (
        "WAVE_6_ENVIRONMENT must be staging; W6.H cannot be closed by dev evidence"
        in payload["invalid_env_vars"]
    )


def test_wave6_deploy_materials_rejects_non_staging_service_url(capsys):
    """materials worksheet 也必须拒绝 local/dev/prod/example service URL。"""
    import report_wave6_deploy_materials as materials

    for service_url in (
        "http://localhost:8080",
        "http://wms-dev.internal",
        "http://wms-prod.internal",
        "http://wms-staging.example",
    ):
        env = _valid_env()
        env["WAVE_6_SERVICE_URL"] = service_url

        assert materials.main(["--json"], env=env) == 1
        payload = json.loads(capsys.readouterr().out)

        assert payload["ok"] is False
        assert any(
            "WAVE_6_SERVICE_URL" in issue
            for issue in payload["invalid_env_vars"]
        )


def test_wave6_deploy_materials_just_entry_exists():
    """W6.H materials worksheet 必须能通过 just 调用。"""
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-6-deploy-materials *args:" in justfile
    assert "scripts/governance/report_wave6_deploy_materials.py" in justfile


def test_wave6_deploy_runbook_documents_audit_before_readiness():
    """W6.H runbook 必须说明 audit 输出 query ref 后再跑 readiness。"""
    runbook = Path("docs/runbooks/wave-6-deploy-evidence.md").read_text(
        encoding="utf-8",
    )

    assert "`execution_plan`" in runbook
    assert "deploy_audit_record" in runbook
    assert "取得 `audit_event_query_ref`" in runbook
    assert "再运行 readiness" in runbook
    assert "`missing_env_by_stage`" in runbook
    assert "`next_blocking_stage`" in runbook
    assert "不要在 deploy audit 之前手工伪造 `WAVE_6_AUDIT_EVENT_QUERY_REF`" in runbook
