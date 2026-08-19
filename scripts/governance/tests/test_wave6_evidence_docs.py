"""Wave 6 范围文档与证据边界治理测试。"""
from pathlib import Path


def test_wave6_scope_docs_reject_prod_and_production_boundaries():
    """Wave 6 范围与预发布 gate 文档必须同时拒绝 prod / production 证据。"""
    roadmap = Path("ROADMAP.md").read_text(encoding="utf-8")
    clarifications = Path("docs/domain/clarifications.md").read_text(encoding="utf-8")

    roadmap_gate_lines = [
        line
        for line in roadmap.splitlines()
        if line.startswith("**预发布 gate**")
        and (
            "不得使用 localhost / stub / mock / fake / example / prod" in line
            or "不得用 localhost / stub / mock / fake / example / prod" in line
        )
    ]

    assert roadmap_gate_lines
    for line in roadmap_gate_lines:
        assert "prod / production" in line

    clarification_70 = next(
        line for line in clarifications.splitlines()
        if "| 70 | Wave 6 范围 |" in line
    )
    assert "prod / production" in clarification_70


def test_wave6_evidence_boundary_docs_do_not_mention_prod_without_production():
    """Wave 6 evidence 边界句不能只写 prod 漏掉 production。"""
    docs = [
        Path("TODO.md"),
        Path("ROADMAP.md"),
        Path("docs/adr/0035-wave-6-pre-release-evidence-closeout.md"),
        Path("docs/retros/wave-1-retro.md"),
    ]

    checked_lines = []
    for path in docs:
        for line in path.read_text(encoding="utf-8").splitlines():
            normalized = line.replace("`", "")
            if "prod" not in normalized:
                continue
            if not any(token in normalized for token in ("localhost", "mock", "fake", "stub", "example")):
                continue
            if not any(token in normalized for token in ("evidence", "证据", "gate")):
                continue
            checked_lines.append((path, line))
            assert "production" in normalized, f"{path}: {line}"

    assert checked_lines


def test_wave6_task_completion_audit_counts_remaining_evidence_gates():
    """Wave 6 审计报告必须对齐 W6.A-C 已关闭、W6.D-H 仍缺的当前状态。"""
    text = Path("docs/reviews/wave-6-task-completion-audit-2026-06-04.md").read_text(
        encoding="utf-8",
    )

    assert "剩 W6.D-H 5 个真实 evidence gate 未关闭" in text
    assert "Wave 6 当前缺 5 个 evidence gate" in text
    assert "W6.A / W6.B / W6.C 的真实 evidence 已补齐" in text
    assert "8 个真实 evidence gate 未关闭" not in text
    assert "Wave 6 当前缺 8 个 evidence gate" not in text


def test_wave6_deploy_runbook_has_staging_dry_run_worksheet():
    """W6.H runbook 必须给出 staging 预演 worksheet，避免把 readiness 当 evidence。"""
    text = Path("docs/runbooks/wave-6-deploy-evidence.md").read_text(
        encoding="utf-8",
    )

    assert "## Staging 预演 Worksheet" in text
    assert "环境必须为真实 `staging`" in text
    assert "`dev` 只能用于前序 smoke / 工具链验证，不能关闭 W6.H" in text
    assert "| `environment` | 只能是 `staging` |" in text
    assert "可由当前 staging readiness 证明" in text
    assert "必须由外部系统归档" in text
    assert "不能把 `/healthz` 200 当成 smoke gate" in text
    assert "不能把本机 image id 当成 artifact ref" in text
    assert "## 发布审计写入" in text
    assert "wms-deploy-audit" in text
    assert "--check-only" in text
    assert "writes_audit_event=false" in text
    assert "writes_runtime_evidence=false" in text
    assert "closes_gate=false" in text
    assert "不要求 `DATABASE_URL`" in text
    assert "不连接数据库，不写 `audit_event`" in text
    assert "--actor-id" in text
    assert "--owner-id" in text
    assert "不要手工 `INSERT INTO audit_event`" in text
    for field in (
        "WAVE_6_RELEASE_PLAN_REF",
        "WAVE_6_ARTIFACT_REF",
        "WAVE_6_CANARY_CONFIG_REF",
        "WAVE_6_SMOKE_GATE_REF",
        "WAVE_6_OBSERVABILITY_DASHBOARD_REF",
        "WAVE_6_ROLLBACK_DRILL_LOG_REF",
        "WAVE_6_APPROVAL_RECORD_REF",
        "WAVE_6_AUDIT_EVENT_QUERY_REF",
    ):
        assert field in text


def test_wave6_staging_dry_run_records_materials_stage_blockers():
    """W6.H dry-run 必须记录 materials 阶段阻塞，避免把未验证布尔值预填为 true。"""
    text = Path("docs/runbooks/wave-6-staging-deploy-dry-run.md").read_text(
        encoding="utf-8",
    )

    assert "## Materials 阶段化阻塞复核" in text
    assert "next_blocking_stage = `pre_audit`" in text
    assert "`pre_audit` 当前仍缺发布版本、外部 ref、审计语义和 H1 actor / owner" in text
    assert "`post_audit` 必须等待正式 deploy audit 输出 `WAVE_6_AUDIT_EVENT_QUERY_REF`" in text
    assert "不能把 `WAVE_6_AUDIT_EVENT_VERIFIED` 预填为 `true`" in text
    assert "不能把 `WAVE_6_DUAL_APPROVAL_RECORDED` 预填为 `true`" in text
    for field in (
        "WAVE_6_RELEASE_VERSION",
        "WAVE_6_DEPLOY_ACTOR_ID",
        "WAVE_6_APPROVAL_RECORD_REF",
        "WAVE_6_AUDIT_EVENT_QUERY_REF",
    ):
        assert field in text


def test_wave6_closeout_w6h_includes_deploy_audit_before_record():
    """W6.H closeout 顺序必须先写 deploy audit_event，再记录 evidence JSON。"""
    text = Path("docs/runbooks/wave-6-closeout.md").read_text(encoding="utf-8")

    w6h_gate_line = next(
        line for line in text.splitlines()
        if line.startswith("| W6.H |")
    )
    assert "just wave-6-deploy-audit" in w6h_gate_line

    w6h_section_start = text.index("### 8. Wave 6 gray release evidence")
    final_closeout_start = text.index("## 最终关闭")
    w6h_section = text[w6h_section_start:final_closeout_start]
    materials = w6h_section.index("just wave-6-deploy-materials")
    readiness = w6h_section.index("just wave-6-deploy-readiness")
    audit = w6h_section.index("just wave-6-deploy-audit")
    record = w6h_section.index("just wave-6-deploy-evidence-record")
    validate = w6h_section.index("just wave-6-deploy-evidence-validate")
    assert materials < audit < readiness < record < validate
