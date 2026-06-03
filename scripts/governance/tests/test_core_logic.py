"""核心逻辑单测：关键 helper 函数的边界行为

不依赖真实仓库内容，使用 fixture 构造测试输入。
"""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_glossary_word_boundary_chinese():
    """禁用词紧贴中文 → 视为更长词的一部分（不报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char("中") is True
    assert _is_word_char("位") is True


def test_glossary_word_boundary_alphanumeric():
    """禁用词紧贴英文/数字/下划线/连字符 → 视为复合词（不报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char("a") is True
    assert _is_word_char("Z") is True
    assert _is_word_char("0") is True
    assert _is_word_char("_") is True
    assert _is_word_char("-") is True


def test_glossary_word_boundary_separators():
    """空格 / 标点 → 视为词边界（应该报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char(" ") is False
    assert _is_word_char(",") is False
    assert _is_word_char("|") is False
    assert _is_word_char("。") is False
    assert _is_word_char("") is False


def test_check_data_appendix_loads():
    """check-data.toml 中的 appendix_references 能被加载。"""
    from _check_data import load_appendix_references
    refs = load_appendix_references()
    # Wave 0 至少有附录 A
    assert len(refs) >= 1
    appendix_a = next((r for r in refs if r.appendix == "附录 A"), None)
    assert appendix_a is not None
    assert appendix_a.defined_in == "user-stories-m1-master-data.md"
    assert "user-stories-m5-cold-chain.md" in appendix_a.expected_in


def test_check_data_exemptions_loads():
    """check-data.toml 中的 approval_source_exemptions 能被加载为 set。"""
    from _check_data import load_approval_source_exemptions
    exemptions = load_approval_source_exemptions()
    assert isinstance(exemptions, set)
    # Wave 0 至少有 M3-003（自身定义审批源）
    assert "US-M3-003" in exemptions
    # 至少 30 条
    assert len(exemptions) >= 25


def test_baseline_health_scan_empty_dir(tmp_path, monkeypatch):
    """baseline 目录为空时不应报错。"""
    import check_baseline_health as bh
    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    monkeypatch.setattr(bh, "SNAPSHOT_FILE", tmp_path / "baseline-health.json")
    counts, issues = bh.scan_baselines()
    assert counts == {}
    assert issues == []


def test_baseline_health_growth_detection(tmp_path, monkeypatch):
    """baseline 数量超过历史上限 → 报错。"""
    import json
    import check_baseline_health as bh

    # 构造一个有 5 个 ignored 的 baseline 文件
    bl = tmp_path / "test_check.json"
    bl.write_text(json.dumps({
        "check": "test_check",
        "version": 1,
        "ignored": [{"id": f"x{i}", "reason": "t", "added_at": "2026-01-01"} for i in range(5)],
    }), encoding="utf-8")

    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)

    snapshot = {"test_check": 3}  # 历史上限 3，当前 5 → 违规
    issues = bh.check_growth({"test_check": 5}, snapshot)
    assert len(issues) == 1
    assert issues[0].kind == "growth"


def test_baseline_health_expired_detection(tmp_path, monkeypatch):
    """expires_at 早于今天 + id 仍在 baseline → 报告过期。"""
    import json
    import check_baseline_health as bh

    bl = tmp_path / "test_check.json"
    bl.write_text(json.dumps({
        "check": "test_check",
        "version": 1,
        "ignored": [
            {"id": "old1", "reason": "t", "added_at": "2020-01-01", "expires_at": "2020-06-01"},
        ],
    }), encoding="utf-8")
    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    counts, issues = bh.scan_baselines()
    assert counts == {"test_check": 1}
    expired_issues = [i for i in issues if i.kind == "expired"]
    assert len(expired_issues) == 1


def test_baseline_health_default_does_not_write_snapshot(tmp_path, monkeypatch):
    """v0.4.1 行为：默认运行不应产生 snapshot 文件（避免 pre-commit 改 working tree）。"""
    import json
    import check_baseline_health as bh

    # 构造一个比 snapshot 更小的 baseline（理论上可触发自动收缩）
    snapshot_file = tmp_path / "baseline-health.json"
    snapshot_file.write_text(json.dumps({
        "version": 1, "max_counts": {"test_check": 5},
    }), encoding="utf-8")

    bl = tmp_path / "test_check.json"
    bl.write_text(json.dumps({
        "check": "test_check",
        "version": 1,
        "ignored": [{"id": "x", "reason": "t", "added_at": "2026-01-01"}],  # count=1 < 5
    }), encoding="utf-8")

    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    monkeypatch.setattr(bh, "SNAPSHOT_FILE", snapshot_file)

    snapshot_before = snapshot_file.read_text()
    bh.main([])  # 默认模式，无 --update-snapshot
    snapshot_after = snapshot_file.read_text()
    assert snapshot_before == snapshot_after, "默认运行不应修改 snapshot 文件"


def test_baseline_health_update_snapshot_writes(tmp_path, monkeypatch):
    """v0.4.1 行为：--update-snapshot 显式调用应写入 snapshot 文件。"""
    import check_baseline_health as bh

    snapshot_file = tmp_path / "baseline-health.json"
    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    monkeypatch.setattr(bh, "SNAPSHOT_FILE", snapshot_file)

    assert not snapshot_file.exists()
    bh.main(["--update-snapshot"])
    assert snapshot_file.exists(), "--update-snapshot 应创建 snapshot 文件"


def test_governance_consistency_doc_parser():
    """check_governance_consistency 能正确解析 §4.6 表格。"""
    from check_governance_consistency import parse_doc_section
    scripts = parse_doc_section()
    # Wave 1+ 必需脚本
    assert "check_layer_dependency" in scripts
    assert scripts["check_layer_dependency"] == "T2"
    assert "check_unsafe_and_unwrap" in scripts
    # CI 全量的脚本不应被纳入（如 perf_baseline）
    assert "check_perf_baseline" not in scripts


def test_governance_consistency_gate_rules_parser():
    """check_governance_consistency 能正确解析 gate-rules.toml 占位规则。"""
    from check_governance_consistency import parse_gate_rules
    scripts = parse_gate_rules()
    # 已实现的脚本不应出现（如 check_doc_links）
    assert "check_doc_links" not in scripts
    # 占位脚本应出现
    assert "check_layer_dependency" in scripts


def test_governance_checks_t2_includes_openapi_full_entrypoint():
    """T2 全量入口必须覆盖 OpenAPI 同步链路，不能只依赖 diff gate。"""
    from governance_checks import expand_tier_scripts

    scripts = expand_tier_scripts("T2")
    assert "check_openapi_in_sync.py" in scripts
    assert "validate_openapi_artifacts.py" in scripts
    assert "check_openapi_contract.py" in scripts


def test_feature_flags_empty_registry_passes():
    """空 Feature Flag 注册表允许存在，不引入默认业务开关。"""
    from datetime import date
    from check_feature_flags import check_flags
    assert check_flags({"flags": []}, today=date(2026, 6, 2)) == []


def test_feature_flags_expired_cleanup_fails():
    """cleanup_by 早于当前日期 → 必须清理。"""
    from datetime import date
    from check_feature_flags import check_flags
    issues = check_flags({
        "flags": [{
            "key": "h1_auth_login_v1",
            "owner": "platform",
            "created_at": "2026-01-01",
            "cleanup_by": "2026-03-01",
            "enabled": False,
        }],
    }, today=date(2026, 6, 2))
    assert any(i.kind == "expired" for i in issues)


def test_feature_flags_lifetime_over_90_days_fails():
    """Feature Flag 清理期不能超过 created_at + 90 天。"""
    from datetime import date
    from check_feature_flags import check_flags
    issues = check_flags({
        "flags": [{
            "key": "h1_auth_login_v1",
            "owner": "platform",
            "created_at": "2026-06-01",
            "cleanup_by": "2026-09-30",
            "enabled": False,
        }],
    }, today=date(2026, 6, 2))
    assert any(i.kind == "lifetime_too_long" for i in issues)


def test_field_coding_standards_accepts_current_type_shapes():
    """字段编码规范脚本应接受现有词典使用的 PostgreSQL 类型形态。"""
    from check_field_coding_standards import is_valid_data_type

    for data_type in ["VARCHAR(32)", "NUMERIC(15,3)", "TIMESTAMPTZ", "JSONB", "TEXT[]", "INT"]:
        assert is_valid_data_type(data_type)
    for data_type in ["FLOAT", "DOUBLE", "TIMESTAMP", "CHAR(8)", "ARRAY[]"]:
        assert not is_valid_data_type(data_type)


def test_field_coding_standards_rejects_int_for_id_fields():
    """INT 仅允许有界计数/配置阈值，ID 类字段必须使用 BIGINT。"""
    from check_gsp_field_traceability import FieldEntry
    from check_field_coding_standards import validate_entries

    issues = validate_entries([
        FieldEntry(
            canonical="customer_id",
            aliases=["客户 ID"],
            data_type="INT",
            validation=">0",
            nullable=False,
            encryption="none",
            audit_required=True,
            field_class="business",
        )
    ], [])

    assert any(issue.rule == "int_id_type" for issue in issues)


def test_business_rules_registry_parser_requires_detail_fields():
    """业务规则注册表详情段必须含字段表。"""
    from check_business_rules_registry import parse_business_rules

    text = "\n".join([
        "| ID | 规则名 | 涉及字段 | 触发场景 | GSP 关联 |",
        "|----|----|----|----|----|",
        "| BR-1 | FIFO | 批号 | 拣货 | 7.94 |",
        "",
        "## 3. BR-1: FIFO",
        "",
        "### 3.1 规则描述",
        "按批号排序。",
    ])

    rules, issues = parse_business_rules(text)

    assert [rule.rule_id for rule in rules] == ["BR-1"]
    assert any(issue.rule == "missing_fields_section" for issue in issues)
    assert any(issue.rule == "missing_detail_fields" for issue in issues)


def test_business_rules_registry_state_enum_exception_only_applies_to_br8():
    """只有 BR-8 状态机可用“状态枚举”替代“规则描述”。"""
    from check_business_rules_registry import parse_business_rules

    text = "\n".join([
        "| ID | 规则名 | 涉及字段 | 触发场景 | GSP 关联 |",
        "|----|----|----|----|----|",
        "| BR-1 | FIFO | 批号 | 拣货 | 7.94 |",
        "| BR-8 | 库存状态机 | 状态 | 状态变更 | 7.95 |",
        "",
        "## 3. BR-1: FIFO",
        "",
        "### 3.1 状态枚举",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `batch_no` | VARCHAR(20) | 批号 |",
        "",
        "## 10. BR-8: 库存状态机",
        "",
        "### 10.1 状态枚举",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `status` | VARCHAR(20) | 当前状态 |",
        "",
        "### 10.3 涉及字段",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `previous_status` | VARCHAR(20) | 上一状态 |",
    ])

    _, issues = parse_business_rules(text)

    assert any(issue.rule_id == "BR-1" and issue.rule == "missing_description" for issue in issues)
    assert not any(issue.rule_id == "BR-8" and issue.rule == "missing_description" for issue in issues)


def test_business_rules_registry_parses_text_array_field_type():
    """业务规则字段表类型解析应与字段编码脚本一致支持 TEXT[]。"""
    from check_business_rules_registry import parse_business_rules

    text = "\n".join([
        "| ID | 规则名 | 涉及字段 | 触发场景 | GSP 关联 |",
        "|----|----|----|----|----|",
        "| BR-1 | FIFO | 经营范围 | 拣货 | 7.94 |",
        "",
        "## 3. BR-1: FIFO",
        "",
        "### 3.1 规则描述",
        "按经营范围过滤。",
        "",
        "### 3.2 涉及字段",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `business_scopes` | TEXT[] | 经营范围 |",
    ])

    rules, issues = parse_business_rules(text)

    assert rules[0].detail_fields == ["business_scopes"]
    assert not any(issue.rule == "missing_detail_fields" for issue in issues)


def test_wave1_completion_report_status_from_checks():
    """Wave 1 证据报告的状态聚合应区分静态证明和待确认。"""
    from report_wave1_completion import (
        MISSING_OR_NEEDS_CONFIRMATION,
        PROVED_BY_STATIC_FILES,
        status_from_checks,
    )

    assert status_from_checks([(True, "a"), (True, "b")]) == (PROVED_BY_STATIC_FILES, ["a", "b"], [])
    assert status_from_checks([(True, "a"), (False, "b")]) == (MISSING_OR_NEEDS_CONFIRMATION, ["a"], ["b"])
    assert status_from_checks([(False, "a")]) == (MISSING_OR_NEEDS_CONFIRMATION, [], ["a"])


def test_wave1_completion_report_manual_items_do_not_make_strict_complete(monkeypatch):
    """外部不可脚本判断项可标记不阻塞，但缺口项必须阻塞 strict。"""
    import report_wave1_completion as report

    monkeypatch.setattr(report, "evaluate_wave1", lambda: [
        report.EvidenceItem("done", "done", report.PROVED_BY_STATIC_FILES),
        report.EvidenceItem("external", "external", report.NOT_SCRIPT_JUDGEABLE, strict_blocking=False),
    ])
    assert report.main(["--strict"]) == 0

    monkeypatch.setattr(report, "evaluate_wave1", lambda: [
        report.EvidenceItem("done", "done", report.PROVED_BY_STATIC_FILES),
        report.EvidenceItem("missing", "missing", report.MISSING_OR_NEEDS_CONFIRMATION, gaps=["x"]),
        report.EvidenceItem("external", "external", report.NOT_SCRIPT_JUDGEABLE, strict_blocking=False),
    ])
    assert report.main(["--strict"]) == 1


def test_wave1_completion_report_default_never_blocks(monkeypatch):
    """默认报告只输出证据，存在阻塞缺口也必须 exit 0。"""
    import report_wave1_completion as report

    monkeypatch.setattr(report, "evaluate_wave1", lambda: [
        report.EvidenceItem("missing", "missing", report.MISSING_OR_NEEDS_CONFIRMATION, gaps=["x"]),
    ])

    assert report.main([]) == 0


def test_wave1_completion_report_h1_story_alignment_accepts_adr_0024_terms(tmp_path, monkeypatch):
    """H1 用户故事必须对齐 ADR-0024 的 TTL、owner_id 与 RLS 延后口径。"""
    import report_wave1_completion as report

    story = tmp_path / "docs" / "domain" / "user-stories-h1-auth-tenant.md"
    story.parent.mkdir(parents=True)
    story.write_text(
        "\n".join([
            "关联 ADR：ADR-0024",
            "Access Token 默认 1 小时",
            "Refresh Token 默认 24 小时",
            "AuthContext.owner_id",
            "PostgreSQL RLS 延后",
            "AUTH-009",
        ]),
        encoding="utf-8",
    )
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    ok, message = report.valid_h1_auth_story_alignment()

    assert ok is True
    assert "ADR-0024" in message


def test_wave1_completion_report_h1_story_alignment_rejects_legacy_terms(tmp_path, monkeypatch):
    """H1 用户故事不能继续保留旧 TTL、RLS 起步或 PG 黑名单降级口径。"""
    import report_wave1_completion as report

    story = tmp_path / "docs" / "domain" / "user-stories-h1-auth-tenant.md"
    story.parent.mkdir(parents=True)
    story.write_text(
        "\n".join([
            "关联 ADR：ADR-0024",
            "Access Token 默认 1 小时",
            "Refresh Token 默认 24 小时",
            "AuthContext.owner_id",
            "PostgreSQL RLS 延后",
            "AUTH-009",
            "JWT，有效期默认 8 小时（可配置）；Refresh Token 默认 7 天",
            "PostgreSQL Row-Level Security (RLS)",
            "Redis 不可用 → 降级到 PG 黑名单表",
        ]),
        encoding="utf-8",
    )
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    ok, message = report.valid_h1_auth_story_alignment()

    assert ok is False
    assert "旧口径" in message
    assert "8 小时" in message


def test_wave1_completion_report_h2_collection_assets_are_checked(tmp_path, monkeypatch):
    """H2 runtime 采集器、just 入口和 runbook 都要在出口报告中可见。"""
    import report_wave1_completion as report

    collector = tmp_path / "scripts" / "governance" / "collect_wave1_h2_runtime_evidence.py"
    collector.parent.mkdir(parents=True)
    collector.write_text("# collector\n", encoding="utf-8")
    prereq = tmp_path / "scripts" / "governance" / "check_wave1_runtime_evidence_prereqs.py"
    prereq.write_text("# prereq\n", encoding="utf-8")
    readiness = tmp_path / "scripts" / "governance" / "check_wave1_h2_runtime_readiness.py"
    readiness.write_text("# readiness\n", encoding="utf-8")
    validator = tmp_path / "scripts" / "governance" / "validate_wave1_runtime_evidence.py"
    validator.write_text("# validator\n", encoding="utf-8")
    (tmp_path / "justfile").write_text(
        "\n".join([
            "wave-1-runtime-evidence-validate:",
            "    @true",
            "wave-1-runtime-prereq-h2:",
            "    @true",
            "wave-1-h2-runtime-readiness:",
            "    @true",
            "wave-1-h2-runtime-evidence:",
            "    @true",
        ]),
        encoding="utf-8",
    )
    runbook = tmp_path / "docs" / "runbooks" / "wave-1-runtime-evidence.md"
    runbook.parent.mkdir(parents=True)
    runbook.write_text(
        "\n".join([
            "just wave-1-runtime-evidence-validate",
            "just wave-1-runtime-prereq-h2",
            "just wave-1-h2-runtime-readiness",
            "just wave-1-h2-runtime-evidence",
        ]),
        encoding="utf-8",
    )
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    ok, message = report.valid_h2_runtime_collection_assets()

    assert ok is True
    assert "H2 runtime" in message


def test_wave1_completion_report_w1d_collection_assets_are_checked(tmp_path, monkeypatch):
    """W1.D runtime probe、just 入口和 runbook 都要在出口报告中可见。"""
    import report_wave1_completion as report

    probe = tmp_path / "deploy" / "scripts" / "wave1_auto_rollback_probe.sh"
    probe.parent.mkdir(parents=True)
    probe.write_text("#!/usr/bin/env bash\n# --check-only\n", encoding="utf-8")
    prereq = tmp_path / "scripts" / "governance" / "check_wave1_runtime_evidence_prereqs.py"
    prereq.parent.mkdir(parents=True)
    prereq.write_text("# prereq\n", encoding="utf-8")
    validator = tmp_path / "scripts" / "governance" / "validate_wave1_runtime_evidence.py"
    validator.write_text("# validator\n", encoding="utf-8")
    (tmp_path / "justfile").write_text(
        "\n".join([
            "wave-1-runtime-evidence-validate:",
            "    @true",
            "wave-1-runtime-prereq-rollback-k8s:",
            "    @true",
            "wave-1-runtime-prereq-rollback-compose:",
            "    @true",
            "wave-1-rollback-runtime-readiness-k8s:",
            "    @true",
            "wave-1-rollback-runtime-readiness-compose:",
            "    @true",
            "wave-1-rollback-runtime-evidence-k8s:",
            "    @true",
            "wave-1-rollback-runtime-evidence-compose:",
            "    @true",
        ]),
        encoding="utf-8",
    )
    runbook = tmp_path / "docs" / "runbooks" / "wave-1-runtime-evidence.md"
    runbook.parent.mkdir(parents=True)
    runbook.write_text(
        "\n".join([
            "just wave-1-runtime-evidence-validate",
            "just wave-1-runtime-prereq-rollback-k8s",
            "just wave-1-runtime-prereq-rollback-compose",
            "just wave-1-rollback-runtime-readiness-k8s",
            "just wave-1-rollback-runtime-readiness-compose",
            "just wave-1-rollback-runtime-evidence-k8s",
            "just wave-1-rollback-runtime-evidence-compose",
        ]),
        encoding="utf-8",
    )
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    ok, message = report.valid_w1d_runtime_collection_assets()

    assert ok is True
    assert "W1.D runtime" in message


def test_wave1_completion_report_h2_runtime_evidence_requires_real_dev_record(tmp_path, monkeypatch):
    """H2 runtime 出口证据必须是 dev 的 wrk 1h/60M baseline/7 天封档记录。"""
    import json
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    evidence_dir = tmp_path / "docs" / "retros"
    evidence_dir.mkdir(parents=True)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "缺少" in message

    evidence = {
        "environment": "dev",
        "captured_at": "2026-06-02T12:00:00+08:00",
        "performance": {
            "tool": "wrk",
            "baseline_rows": 60_000_000,
            "target_qps": 1000,
            "observed_qps": 1001.5,
            "duration_seconds": 3600,
            "p99_ms": 199.5,
            "benchmark_log_ref": "s3://wms-dev-evidence/wave1/h2/wrk-20260602.log",
        },
        "seal_cron": {
            "consecutive_success_days": 7,
            "failure_count": 0,
            "last_seal_verified": True,
            "cron_log_ref": "s3://wms-dev-evidence/wave1/h2/audit-seal-cron.log",
        },
    }
    (evidence_dir / "wave-1-h2-runtime-evidence.json").write_text(
        json.dumps(evidence),
        encoding="utf-8",
    )

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is True
    assert "真实 PostgreSQL" in message

    evidence["performance"]["duration_seconds"] = 3599
    (evidence_dir / "wave-1-h2-runtime-evidence.json").write_text(
        json.dumps(evidence),
        encoding="utf-8",
    )

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "duration_seconds" in message


def test_wave1_completion_report_h2_runtime_evidence_rejects_local_or_prod_refs(tmp_path, monkeypatch):
    """H2 runtime 证据不能指向本机或生产边界。"""
    import json
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    evidence_dir = tmp_path / "docs" / "retros"
    evidence_dir.mkdir(parents=True)
    evidence = {
        "environment": "dev",
        "captured_at": "2026-06-02T12:00:00+08:00",
        "performance": {
            "tool": "wrk",
            "baseline_rows": 60_000_000,
            "target_qps": 1000,
            "observed_qps": 1001.5,
            "duration_seconds": 3600,
            "p99_ms": 199.5,
            "benchmark_log_ref": "http://127.0.0.1/wms-dev/wrk.log",
        },
        "seal_cron": {
            "consecutive_success_days": 7,
            "failure_count": 0,
            "last_seal_verified": True,
            "cron_log_ref": "s3://wms-dev-evidence/wave1/h2/audit-seal-cron.log",
        },
    }
    (evidence_dir / "wave-1-h2-runtime-evidence.json").write_text(
        json.dumps(evidence),
        encoding="utf-8",
    )

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "非本机 dev" in message

    evidence["performance"]["benchmark_log_ref"] = "s3://wms-prod-evidence/wave1/h2/wrk.log"
    (evidence_dir / "wave-1-h2-runtime-evidence.json").write_text(
        json.dumps(evidence),
        encoding="utf-8",
    )

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "非本机 dev" in message

    evidence["performance"]["benchmark_log_ref"] = "s3://wms-dev-stub-evidence/wave1/h2/wrk.log"
    (evidence_dir / "wave-1-h2-runtime-evidence.json").write_text(
        json.dumps(evidence),
        encoding="utf-8",
    )

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "非本机 dev" in message


def test_collect_wave1_h2_runtime_evidence_writes_valid_json(tmp_path, monkeypatch):
    """H2 collector 应从真实 wrk 输出和 DB 统计生成可被出口报告接受的 JSON。"""
    import report_wave1_completion as report
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Latency Distribution\n"
        "  50%    10.00ms\n"
        "  99%   123.45ms\n"
        "Requests/sec:   1001.23\n",
        encoding="utf-8",
    )
    output = tmp_path / "docs" / "retros" / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)

    exit_code = collector.main([
        "--database-url",
        "postgres://wms@pg-dev.wms.internal/wms_dev",
        "--wrk-output",
        str(wrk_output),
        "--benchmark-log-ref",
        "s3://wms-dev-evidence/wave1/h2/wrk.log",
        "--cron-log-ref",
        "s3://wms-dev-evidence/wave1/h2/seal-cron.log",
        "--duration-seconds",
        "3600",
        "--output",
        str(output),
    ])

    assert exit_code == 0
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    ok, message = report.valid_h2_runtime_evidence()
    assert ok is True
    assert "真实 PostgreSQL" in message


def test_collect_wave1_h2_runtime_evidence_rejects_short_or_slow_runs(tmp_path, monkeypatch):
    """H2 collector 不能为短跑或低吞吐生成出口证据。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Latency Distribution\n"
        "  99%   123.45ms\n"
        "Requests/sec:    999.99\n",
        encoding="utf-8",
    )
    output = tmp_path / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)

    exit_code = collector.main([
        "--database-url",
        "postgres://wms@pg-dev.wms.internal/wms_dev",
        "--wrk-output",
        str(wrk_output),
        "--benchmark-log-ref",
        "s3://wms-dev-evidence/wave1/h2/wrk.log",
        "--cron-log-ref",
        "s3://wms-dev-evidence/wave1/h2/seal-cron.log",
        "--duration-seconds",
        "3600",
        "--output",
        str(output),
    ])

    assert exit_code == 2
    assert not output.exists()


def test_collect_wave1_h2_runtime_evidence_rejects_local_database_url(tmp_path, monkeypatch):
    """H2 collector 不能从本机 PostgreSQL 生成 Wave 1 runtime 出口证据。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Latency Distribution\n"
        "  99%   123.45ms\n"
        "Requests/sec:   1001.23\n",
        encoding="utf-8",
    )
    output = tmp_path / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)

    exit_code = collector.main([
        "--database-url",
        "postgres://wms@127.0.0.1:5432/wms_dev",
        "--wrk-output",
        str(wrk_output),
        "--benchmark-log-ref",
        "s3://wms-dev-evidence/wave1/h2/wrk.log",
        "--cron-log-ref",
        "s3://wms-dev-evidence/wave1/h2/seal-cron.log",
        "--duration-seconds",
        "3600",
        "--output",
        str(output),
    ])

    assert exit_code == 2
    assert not output.exists()


def test_wave1_h2_runtime_readiness_accepts_ready_dev_database(monkeypatch):
    """H2 readiness 应在跑 1 小时 wrk 前确认 dev DB 基线与封档达标。"""
    import check_wave1_h2_runtime_readiness as readiness

    monkeypatch.setattr(readiness, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(readiness, "count_recent_seals", lambda database_url: 7)

    ok, facts, issues = readiness.check_readiness(
        "postgres://wms@pg-dev.wms.internal:5432/wms_dev",
        "dev",
        60_000_000,
        7,
    )

    assert ok is True
    assert facts["baseline_rows"] == 60_000_000
    assert facts["consecutive_success_days"] == 7
    assert issues == []


def test_wave1_h2_runtime_readiness_rejects_small_or_unsealed_database(monkeypatch):
    """H2 readiness 不达标时不能进入长时间 wrk 压测。"""
    import check_wave1_h2_runtime_readiness as readiness

    monkeypatch.setattr(readiness, "count_audit_rows", lambda database_url: 59_999_999)
    monkeypatch.setattr(readiness, "count_recent_seals", lambda database_url: 6)

    ok, facts, issues = readiness.check_readiness(
        "postgres://wms@pg-dev.wms.internal:5432/wms_dev",
        "dev",
        60_000_000,
        7,
    )

    assert ok is False
    assert facts["baseline_rows"] == 59_999_999
    assert facts["consecutive_success_days"] == 6
    assert any("baseline_rows" in issue for issue in issues)
    assert any("consecutive_success_days" in issue for issue in issues)


def test_wave1_h2_runtime_readiness_rejects_local_database(monkeypatch):
    """H2 readiness 本身也不能接受本机 PostgreSQL。"""
    import check_wave1_h2_runtime_readiness as readiness

    monkeypatch.setattr(readiness, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(readiness, "count_recent_seals", lambda database_url: 7)

    exit_code = readiness.main([
        "--database-url",
        "postgres://wms@127.0.0.1:5432/wms_dev",
    ])

    assert exit_code == 2


def _wave1_prereq_module(monkeypatch):
    import check_wave1_runtime_evidence_prereqs as prereq

    monkeypatch.setattr(prereq.shutil, "which", lambda command: f"/usr/bin/{command}")
    return prereq


def _clear_wave1_prereq_env(monkeypatch):
    for name in [
        "WAVE1_H2_DATABASE_URL",
        "WAVE1_H2_WRK_OUTPUT",
        "WAVE1_H2_BENCHMARK_LOG_REF",
        "WAVE1_H2_CRON_LOG_REF",
        "WAVE1_H2_DURATION_SECONDS",
        "WAVE1_H2_TARGET_QPS",
        "WAVE1_H2_SEAL_FAILURE_COUNT",
        "WAVE1_ROLLBACK_ENVIRONMENT",
        "WAVE1_K8S_CONTEXT",
        "WAVE1_K8S_NAMESPACE",
        "WAVE1_PREVIOUS_VERSION",
        "WAVE1_COMPOSE_FILE",
        "WAVE1_ROLLBACK_LOG_REF",
        "WAVE1_EXTERNAL_LOG_REF",
        "SMOKE_URL",
        "PROMETHEUS_URL",
        "PROMETHEUS_QUERY",
    ]:
        monkeypatch.delenv(name, raising=False)


def test_wave1_runtime_prereq_h2_rejects_missing_env(monkeypatch, capsys):
    """H2 前置检查必须先拿到真实采集边界参数。"""
    prereq = _wave1_prereq_module(monkeypatch)
    _clear_wave1_prereq_env(monkeypatch)

    exit_code = prereq.main(["--mode", "h2"])

    assert exit_code == 2
    err = capsys.readouterr().err
    assert "WAVE1_H2_DATABASE_URL" in err
    assert "WAVE1_H2_WRK_OUTPUT" in err


def test_wave1_runtime_prereq_h2_rejects_local_prod_or_stub_boundaries(
    tmp_path, monkeypatch, capsys
):
    """H2 前置检查不能接受本机、生产或 stub 边界。"""
    prereq = _wave1_prereq_module(monkeypatch)
    _clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Requests/sec: 1001\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_H2_DATABASE_URL", "postgres://wms@127.0.0.1:5432/wms_dev")
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-prod-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-stub-evidence/wave1/h2/cron.log")

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 2
    err = capsys.readouterr().err
    assert "WAVE1_H2_DATABASE_URL" in err
    assert "WAVE1_H2_BENCHMARK_LOG_REF" in err
    assert "WAVE1_H2_CRON_LOG_REF" in err


def test_wave1_runtime_prereq_h2_accepts_valid_dev_inputs(tmp_path, monkeypatch):
    """H2 前置检查通过后才允许进入正式 collector。"""
    prereq = _wave1_prereq_module(monkeypatch)
    _clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Latency Distribution\n  99%  123.45ms\nRequests/sec: 1001.23\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_H2_DATABASE_URL", "postgres://wms@pg-dev.wms.internal:5432/wms_dev")
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 0


def test_wave1_runtime_prereq_rollback_rejects_missing_signal(monkeypatch, capsys):
    """自动回滚前置检查必须有真实 smoke 或 Prometheus 信号。"""
    prereq = _wave1_prereq_module(monkeypatch)
    _clear_wave1_prereq_env(monkeypatch)
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_K8S_CONTEXT", "wms-staging")
    monkeypatch.setenv("WAVE1_K8S_NAMESPACE", "wms-staging")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-staging-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")

    exit_code = prereq.main(["--mode", "rollback-k8s"])

    assert exit_code == 2
    assert "missing runtime signal" in capsys.readouterr().err


def test_wave1_runtime_prereq_rollback_rejects_local_prod_or_stub_boundaries(
    monkeypatch, capsys
):
    """自动回滚前置检查不能接受本机、生产或 stub 信号/日志。"""
    prereq = _wave1_prereq_module(monkeypatch)
    _clear_wave1_prereq_env(monkeypatch)
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "staging")
    monkeypatch.setenv("WAVE1_K8S_CONTEXT", "wms-staging")
    monkeypatch.setenv("WAVE1_K8S_NAMESPACE", "wms-staging")
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-prod-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-staging-evidence/wave1/smoke.log")
    monkeypatch.setenv("SMOKE_URL", "http://127.0.0.1/staging-stub/healthz")

    exit_code = prereq.main(["--mode", "rollback-k8s"])

    assert exit_code == 2
    err = capsys.readouterr().err
    assert "WAVE1_ROLLBACK_LOG_REF" in err
    assert "SMOKE_URL" in err


def test_wave1_runtime_prereq_rollback_compose_accepts_valid_prometheus_signal(
    tmp_path, monkeypatch
):
    """docker-compose 前置检查接受真实 dev Prometheus 信号配置。"""
    prereq = _wave1_prereq_module(monkeypatch)
    _clear_wave1_prereq_env(monkeypatch)
    compose_dir = tmp_path / "wms-dev"
    compose_dir.mkdir()
    compose_file = compose_dir / "docker-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_ROLLBACK_ENVIRONMENT", "dev")
    monkeypatch.setenv("WAVE1_PREVIOUS_VERSION", "previous-dev-sha")
    monkeypatch.setenv("WAVE1_COMPOSE_FILE", str(compose_file))
    monkeypatch.setenv("PROMETHEUS_URL", "https://prometheus.dev.wms.internal")
    monkeypatch.setenv("PROMETHEUS_QUERY", 'wms_wave1_rollback_signal{environment="dev"}')
    monkeypatch.setenv("WAVE1_ROLLBACK_LOG_REF", "s3://wms-dev-evidence/wave1/rollback.log")
    monkeypatch.setenv("WAVE1_EXTERNAL_LOG_REF", "s3://wms-dev-evidence/wave1/prometheus.log")

    exit_code = prereq.main(["--mode", "rollback-compose"])

    assert exit_code == 0


def test_wave1_completion_report_w1d_runtime_evidence_requires_real_signal_record(tmp_path, monkeypatch):
    """W1.D runtime 出口证据必须证明 dev/staging 失败信号触发回滚成功。"""
    import json
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    evidence_dir = tmp_path / "docs" / "retros"
    evidence_dir.mkdir(parents=True)

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is False
    assert "缺少" in message

    evidence = {
        "environment": "staging",
        "captured_at": "2026-06-02T12:00:00+08:00",
        "signal_type": "prometheus",
        "signal_url": "https://prometheus.staging.wms.internal/api/v1/query",
        "rollback_triggered": True,
        "rollback_exit_code": 0,
        "rollback_log_ref": "s3://wms-staging-evidence/wave1/rollback.log",
        "external_log_ref": "s3://wms-staging-evidence/wave1/monitoring-alert.log",
    }
    (evidence_dir / "wave-1-runtime-evidence.json").write_text(
        json.dumps(evidence),
        encoding="utf-8",
    )

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is True
    assert "自动回滚证据" in message

    evidence["signal_url"] = "http://127.0.0.1/wms-staging/smoke"
    (evidence_dir / "wave-1-runtime-evidence.json").write_text(
        json.dumps(evidence),
        encoding="utf-8",
    )

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is False
    assert "localhost" in message


def test_validate_wave1_runtime_evidence_accepts_two_real_records(tmp_path, capsys):
    """定向 validator 应复用出口报告的 H2/W1.D runtime 证据规则。"""
    import json
    import validate_wave1_runtime_evidence as validator

    h2_file = tmp_path / "wave-1-h2-runtime-evidence.json"
    h2_file.write_text(
        json.dumps({
            "environment": "dev",
            "captured_at": "2026-06-03T12:00:00+08:00",
            "performance": {
                "tool": "wrk",
                "baseline_rows": 60_000_000,
                "target_qps": 1000,
                "observed_qps": 1001.5,
                "duration_seconds": 3600,
                "p99_ms": 199.5,
                "benchmark_log_ref": "s3://wms-dev-evidence/wave1/h2/wrk.log",
            },
            "seal_cron": {
                "consecutive_success_days": 7,
                "failure_count": 0,
                "last_seal_verified": True,
                "cron_log_ref": "s3://wms-dev-evidence/wave1/h2/seal-cron.log",
            },
        }),
        encoding="utf-8",
    )
    w1d_file = tmp_path / "wave-1-runtime-evidence.json"
    w1d_file.write_text(
        json.dumps({
            "environment": "staging",
            "captured_at": "2026-06-03T12:00:00+08:00",
            "signal_type": "http",
            "signal_url": "https://smoke.staging.wms.internal/wms/healthz",
            "rollback_triggered": True,
            "rollback_exit_code": 0,
            "rollback_log_ref": "s3://wms-staging-evidence/wave1/rollback.log",
            "external_log_ref": "s3://wms-staging-evidence/wave1/smoke-alert.log",
        }),
        encoding="utf-8",
    )

    exit_code = validator.main([
        "--kind",
        "all",
        "--h2-file",
        str(h2_file),
        "--w1d-file",
        str(w1d_file),
    ])

    assert exit_code == 0
    out = capsys.readouterr().out
    assert "H2 runtime evidence 内容有效" in out
    assert "W1.D runtime evidence 内容有效" in out


def test_validate_wave1_runtime_evidence_rejects_fake_h2_boundary(tmp_path, capsys):
    """定向 validator 不能接受 fake/stub 边界。"""
    import json
    import validate_wave1_runtime_evidence as validator

    h2_file = tmp_path / "wave-1-h2-runtime-evidence.json"
    h2_file.write_text(
        json.dumps({
            "environment": "dev",
            "captured_at": "2026-06-03T12:00:00+08:00",
            "performance": {
                "tool": "wrk",
                "baseline_rows": 60_000_000,
                "target_qps": 1000,
                "observed_qps": 1001.5,
                "duration_seconds": 3600,
                "p99_ms": 199.5,
                "benchmark_log_ref": "s3://wms-dev-fake-evidence/wave1/h2/wrk.log",
            },
            "seal_cron": {
                "consecutive_success_days": 7,
                "failure_count": 0,
                "last_seal_verified": True,
                "cron_log_ref": "s3://wms-dev-evidence/wave1/h2/seal-cron.log",
            },
        }),
        encoding="utf-8",
    )

    exit_code = validator.main(["--kind", "h2", "--h2-file", str(h2_file)])

    assert exit_code == 1
    assert "benchmark_log_ref" in capsys.readouterr().out


def test_validate_wave1_runtime_evidence_rejects_example_refs_unless_explicitly_allowed(
    tmp_path,
):
    """正式 evidence 不能复制 .example.json 模板引用；模板自检必须显式豁免。"""
    import json
    import validate_wave1_runtime_evidence as validator

    w1d_file = tmp_path / "wave-1-runtime-evidence.example.json"
    w1d_file.write_text(
        json.dumps({
            "environment": "staging",
            "captured_at": "2026-06-03T12:00:00+08:00",
            "signal_type": "prometheus",
            "signal_url": "https://prometheus.staging.example.com/api/v1/query",
            "rollback_triggered": True,
            "rollback_exit_code": 0,
            "rollback_log_ref": "s3://wms-staging-evidence/wave1/rollback.log",
            "external_log_ref": "s3://wms-staging-evidence/wave1/monitoring-alert.log",
        }),
        encoding="utf-8",
    )

    assert validator.main(["--kind", "w1d", "--w1d-file", str(w1d_file)]) == 1
    assert validator.main([
        "--kind",
        "w1d",
        "--w1d-file",
        str(w1d_file),
        "--allow-example-refs",
    ]) == 0


def test_wave1_completion_report_h1_h2_partial_static_hits_do_not_complete(monkeypatch):
    """H1/H2 缺少 handler/helper 测试证据时不能完成。"""
    import report_wave1_completion as report

    def fake_any_file_contains(root, pattern):
        if "auth_context_extractor_is_demo_items_handler_compatible" in pattern:
            return False
        if "two_mutation_handlers_reuse_commit_with_audit" in pattern:
            return False
        return True

    monkeypatch.setattr(report, "accepted_adr", lambda path: True)
    monkeypatch.setattr(report, "any_file_contains", fake_any_file_contains)
    monkeypatch.setattr(report, "file_exists", lambda path: True)
    monkeypatch.setattr(report, "file_contains", lambda path, needle: True)

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.A"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert items["W1.A"].blocks_strict
    assert any("示例业务 handler" in gap for gap in items["W1.A"].gaps)
    assert items["W1.B"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert items["W1.B"].blocks_strict
    assert any("mutation handler" in gap for gap in items["W1.B"].gaps)


def test_wave1_completion_report_missing_runtime_evidence_is_pre_release_gate(monkeypatch):
    """缺真实 runtime evidence 不阻塞 Wave 1 开发完成，但必须作为预发布 gate 留痕。"""
    import report_wave1_completion as report

    def fake_file_contains(path, needle):
        if needle == "stub kubectl/docker":
            return False
        if needle.startswith('id: "h1"') or needle.startswith('id: "h2"'):
            return False
        return True

    monkeypatch.setattr(report, "accepted_adr", lambda path: True)
    monkeypatch.setattr(report, "any_file_contains", lambda root, pattern: True)
    monkeypatch.setattr(report, "file_exists", lambda path: True)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "valid_h2_runtime_evidence", lambda: (False, "缺少 H2 真实 runtime evidence"))
    monkeypatch.setattr(report, "valid_w1d_runtime_evidence", lambda: (False, "缺少 W1.D 真实 runtime evidence"))
    monkeypatch.setattr(report, "valid_h2_runtime_collection_assets", lambda: (True, "H2 runtime 采集资产已就绪"))
    monkeypatch.setattr(report, "valid_w1d_runtime_collection_assets", lambda: (True, "W1.D runtime 采集资产已就绪"))

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.B"].status == report.PROVED_BY_STATIC_FILES
    assert items["W1.D-runtime"].status == report.PROVED_BY_STATIC_FILES
    assert items["W1.B-pre-release-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert items["W1.B-pre-release-runtime"].strict_blocking is False
    assert items["W1.D-pre-release-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert items["W1.D-pre-release-runtime"].strict_blocking is False
    assert not any(item.blocks_strict for item in items.values())


def test_wave1_completion_report_does_not_add_non_roadmap_decision_gate():
    """出口报告只覆盖 ROADMAP Wave 1 完成标准，不加入额外决策门禁。"""
    import report_wave1_completion as report

    item_ids = {item.item_id for item in report.evaluate_wave1()}

    assert "W1-decisions" not in item_ids


def test_wave1_completion_report_w1d_backend_evidence_is_not_keyword_only(monkeypatch):
    """W1.D 后端证据不能只因注释里出现 feature flag 字样而通过。"""
    import report_wave1_completion as report

    def fake_any_file_contains(root, pattern):
        if root == "backend" and "FeatureFlagRegistry" in pattern:
            return False
        if root == "deploy":
            return True
        return False

    monkeypatch.setattr(report, "any_file_contains", fake_any_file_contains)
    monkeypatch.setattr(report, "file_exists", lambda path: path == "docs/retros/wave-1-retro.md")
    monkeypatch.setattr(report, "file_contains", lambda path, needle: needle == "dev/staging")
    monkeypatch.setattr(report, "accepted_adr", lambda path: True)

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.D-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert any("FeatureFlagRegistry" in gap for gap in items["W1.D-runtime"].gaps)


def test_wave1_completion_report_w1d_signal_entry_without_runtime_record_is_pre_release_gap(monkeypatch):
    """W1.D 真实信号入口可完成开发门禁，真实运行记录仍进入预发布 gate。"""
    import report_wave1_completion as report

    def fake_any_file_contains(root, pattern):
        return root == "backend" and "FeatureFlagRegistry" in pattern

    def fake_file_exists(path):
        return path in {
            "deploy/scripts/wave1_rollback.sh",
            "deploy/scripts/wave1_auto_rollback_probe.sh",
            "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
            "scripts/governance/validate_wave1_runtime_evidence.py",
            "docs/retros/wave-1-retro.md",
        }

    def fake_file_contains(path, needle):
        if path == "justfile":
            return needle in {
                "wave-1-runtime-prereq-rollback-k8s",
                "wave-1-runtime-prereq-rollback-compose",
                "wave-1-rollback-runtime-readiness-k8s",
                "wave-1-rollback-runtime-readiness-compose",
                "wave-1-rollback-runtime-evidence-k8s",
                "wave-1-rollback-runtime-evidence-compose",
                "wave-1-runtime-evidence-validate",
            }
        if path == "docs/runbooks/wave-1-runtime-evidence.md":
            return needle in {
                "just wave-1-runtime-prereq-rollback-k8s",
                "just wave-1-runtime-prereq-rollback-compose",
                "just wave-1-rollback-runtime-readiness-k8s",
                "just wave-1-rollback-runtime-readiness-compose",
                "just wave-1-rollback-runtime-evidence-k8s",
                "just wave-1-rollback-runtime-evidence-compose",
                "just wave-1-runtime-evidence-validate",
            }
        if path == "deploy/scripts/wave1_rollback.sh":
            return needle in {
                "kubectl rollout undo",
                "docker compose",
                "--execute",
                "validate_environment_boundary",
                'validate_environment_boundary "--context" "$context"',
                'validate_environment_boundary "--namespace" "$namespace"',
                'validate_environment_boundary "--compose-file" "$compose_file_abs"',
                "must include the selected environment token",
                "must not point to a production boundary",
            }
        if path == "deploy/scripts/wave1_auto_rollback_probe.sh":
            return needle in {
                "missing runtime evidence",
                "--smoke-url",
                "PROMETHEUS_URL",
                "wave1_rollback.sh",
                "--execute",
                "--check-only",
            }
        if path == "docs/retros/wave-1-retro.md":
            return needle == "dev/staging"
        return False

    monkeypatch.setattr(report, "any_file_contains", fake_any_file_contains)
    monkeypatch.setattr(report, "file_exists", fake_file_exists)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "accepted_adr", lambda path: True)

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.D-runtime"].status == report.PROVED_BY_STATIC_FILES
    assert not items["W1.D-runtime"].blocks_strict
    assert items["W1.D-pre-release-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert not items["W1.D-pre-release-runtime"].blocks_strict
    assert any("真实 dev/staging" in gap for gap in items["W1.D-pre-release-runtime"].gaps)


def test_wave1_completion_report_w1d_deploy_keyword_is_not_enough(monkeypatch):
    """deploy 文件里只有 rollback 字样不能证明 W1.D 回滚链路。"""
    import report_wave1_completion as report

    def fake_any_file_contains(root, pattern):
        return root == "backend" and "FeatureFlagRegistry" in pattern

    def fake_file_exists(path):
        return path in {
            "deploy/scripts/wave1_rollback.sh",
            "docs/retros/wave-1-retro.md",
        }

    def fake_file_contains(path, needle):
        if path == "deploy/scripts/wave1_rollback.sh":
            return needle == "rollback"
        if path == "docs/retros/wave-1-retro.md":
            return needle == "dev/staging"
        return False

    monkeypatch.setattr(report, "any_file_contains", fake_any_file_contains)
    monkeypatch.setattr(report, "file_exists", fake_file_exists)
    monkeypatch.setattr(report, "file_contains", fake_file_contains)
    monkeypatch.setattr(report, "accepted_adr", lambda path: True)

    items = {item.item_id: item for item in report.evaluate_wave1()}

    assert items["W1.D-runtime"].status == report.MISSING_OR_NEEDS_CONFIRMATION
    assert any("回滚执行资产" in gap for gap in items["W1.D-runtime"].gaps)


def test_wave1_rollback_script_dry_run_paths():
    """Wave 1 回滚脚本默认只 dry-run 两条 ADR-0016 路径。"""
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_rollback.sh"

    k8s = subprocess.run(
        [str(script), "--target", "k8s", "--environment", "dev"],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert k8s.returncode == 0
    assert "dry-run:" in k8s.stdout
    assert "kubectl rollout undo deployment/wms-api" in k8s.stdout

    compose = subprocess.run(
        [
            str(script),
            "--target",
            "docker-compose",
            "--environment",
            "staging",
            "--previous-version",
            "abc123",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert compose.returncode == 0
    assert "dry-run:" in compose.stdout
    assert "WMS_VERSION=abc123 docker compose up -d" in compose.stdout


def test_wave1_rollback_script_requires_previous_version():
    """docker-compose 回滚必须显式给出上一稳定版本。"""
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_rollback.sh"

    result = subprocess.run(
        [str(script), "--target", "docker-compose", "--environment", "dev"],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    assert result.returncode == 2
    assert "--previous-version is required" in result.stderr


def test_wave1_rollback_script_rejects_execute_without_boundary(tmp_path):
    """真实执行必须显式给出可审计的目标边界。"""
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_rollback.sh"
    compose_file = tmp_path / "compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")

    k8s = subprocess.run(
        [
            str(script),
            "--target",
            "k8s",
            "--environment",
            "dev",
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert k8s.returncode == 2
    assert "--context and --namespace are required" in k8s.stderr

    compose = subprocess.run(
        [
            str(script),
            "--target",
            "docker-compose",
            "--environment",
            "dev",
            "--previous-version",
            "abc123",
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert compose.returncode == 2
    assert "--compose-file is required" in compose.stderr

    missing_compose_file = subprocess.run(
        [
            str(script),
            "--target",
            "docker-compose",
            "--environment",
            "dev",
            "--previous-version",
            "abc123",
            "--compose-file",
            str(compose_file.with_name("missing.yml")),
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert missing_compose_file.returncode == 2
    assert "--compose-file must point to an existing file" in missing_compose_file.stderr


def test_wave1_rollback_script_execute_k8s_uses_explicit_boundary(tmp_path):
    """k8s 执行路径必须把 context/namespace 传给 kubectl。"""
    import os
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_rollback.sh"
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    log_file = tmp_path / "kubectl.log"
    kubectl = bin_dir / "kubectl"
    kubectl.write_text(
        f"#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" > {log_file}\n",
        encoding="utf-8",
    )
    kubectl.chmod(0o755)
    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}

    result = subprocess.run(
        [
            str(script),
            "--target",
            "k8s",
            "--environment",
            "staging",
            "--context",
            "wms-staging",
            "--namespace",
            "wms-staging",
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    assert result.returncode == 0
    assert "context=wms-staging namespace=wms-staging" in result.stdout
    assert log_file.read_text(encoding="utf-8").strip() == (
        "rollout undo deployment/wms-api --context wms-staging --namespace wms-staging"
    )


def test_wave1_rollback_script_execute_compose_uses_explicit_file(tmp_path):
    """docker-compose 执行路径必须把 compose file 和 WMS_VERSION 传给 docker。"""
    import os
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_rollback.sh"
    compose_file = tmp_path / "staging-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    log_file = tmp_path / "docker.log"
    docker = bin_dir / "docker"
    docker.write_text(
        f"#!/usr/bin/env bash\nprintf 'WMS_VERSION=%s args=%s\\n' \"$WMS_VERSION\" \"$*\" > {log_file}\n",
        encoding="utf-8",
    )
    docker.chmod(0o755)
    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}

    result = subprocess.run(
        [
            str(script),
            "--target",
            "docker-compose",
            "--environment",
            "staging",
            "--previous-version",
            "abc123",
            "--compose-file",
            str(compose_file),
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    assert result.returncode == 0
    assert f"compose_file={compose_file}" in result.stdout
    assert log_file.read_text(encoding="utf-8").strip() == (
        f"WMS_VERSION=abc123 args=compose -f {compose_file} up -d"
    )


def test_wave1_rollback_script_execute_rejects_environment_boundary_mismatch(tmp_path):
    """真实执行拒绝 environment 与实际执行边界不一致的参数。"""
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_rollback.sh"
    prod_compose = tmp_path / "prod-compose.yml"
    prod_compose.write_text("services: {}\n", encoding="utf-8")
    staging_compose = tmp_path / "staging-compose.yml"
    staging_compose.write_text("services: {}\n", encoding="utf-8")

    k8s_prod_boundary = subprocess.run(
        [
            str(script),
            "--target",
            "k8s",
            "--environment",
            "dev",
            "--context",
            "wms-prod",
            "--namespace",
            "prod",
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert k8s_prod_boundary.returncode == 2
    assert "must not point to a production boundary" in k8s_prod_boundary.stderr

    k8s_wrong_environment = subprocess.run(
        [
            str(script),
            "--target",
            "k8s",
            "--environment",
            "dev",
            "--context",
            "wms-staging",
            "--namespace",
            "wms-staging",
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert k8s_wrong_environment.returncode == 2
    assert "must include the selected environment token (dev)" in k8s_wrong_environment.stderr

    compose_prod_boundary = subprocess.run(
        [
            str(script),
            "--target",
            "docker-compose",
            "--environment",
            "dev",
            "--previous-version",
            "abc123",
            "--compose-file",
            str(prod_compose),
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert compose_prod_boundary.returncode == 2
    assert "must not point to a production boundary" in compose_prod_boundary.stderr

    compose_wrong_environment = subprocess.run(
        [
            str(script),
            "--target",
            "docker-compose",
            "--environment",
            "dev",
            "--previous-version",
            "abc123",
            "--compose-file",
            str(staging_compose),
            "--execute",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert compose_wrong_environment.returncode == 2
    assert "must include the selected environment token (dev)" in compose_wrong_environment.stderr


@pytest.mark.parametrize(
    ("args", "expected"),
    [
        (["--target", "k8s", "--environment", "prod"], "--environment must be dev or staging"),
        (["--target", "vm", "--environment", "dev"], "--target must be k8s or docker-compose"),
    ],
)
def test_wave1_rollback_script_rejects_invalid_arguments(args, expected):
    """无效环境和目标应在执行前拒绝。"""
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_rollback.sh"

    result = subprocess.run(
        [str(script), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    assert result.returncode == 2
    assert expected in result.stderr


def test_wave1_auto_rollback_probe_requires_runtime_signal():
    """没有真实 smoke/Prometheus 信号时，自动回滚 probe 必须拒绝产出证据。"""
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_auto_rollback_probe.sh"

    result = subprocess.run(
        [
            str(script),
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    assert result.returncode == 2
    assert "missing runtime evidence" in result.stderr


def test_wave1_auto_rollback_probe_check_only_does_not_call_signal_or_rollback(tmp_path):
    """check-only 只校验边界与引用，不请求 signal、不执行 rollback、不写 evidence。"""
    import os
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_auto_rollback_probe.sh"
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    command_log = tmp_path / "commands.log"
    evidence_file = tmp_path / "docs" / "retros" / "wave-1-runtime-evidence.json"

    for command in ["curl", "kubectl", "docker"]:
        stub = bin_dir / command
        stub.write_text(
            f"#!/usr/bin/env bash\nprintf '{command} called %s\\n' \"$*\" >> {command_log}\nexit 7\n",
            encoding="utf-8",
        )
        stub.chmod(0o755)

    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}
    result = subprocess.run(
        [
            str(script),
            "--check-only",
            "--environment",
            "staging",
            "--target",
            "k8s",
            "--context",
            "wms-staging",
            "--namespace",
            "wms-staging",
            "--smoke-url",
            "https://smoke.staging.wms.internal/wms/healthz",
            "--evidence-file",
            str(evidence_file),
            "--rollback-log-ref",
            "s3://wms-staging-evidence/wave1/rollback.log",
            "--external-log-ref",
            "s3://wms-staging-evidence/wave1/smoke-alert.log",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    assert result.returncode == 0
    assert "readiness ok environment=staging target=k8s signal=http" in result.stdout
    assert not command_log.exists()
    assert not evidence_file.exists()


def test_wave1_auto_rollback_probe_check_only_rejects_bad_evidence_refs(tmp_path):
    """check-only 也必须拒绝 prod/stub evidence 引用。"""
    import os
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_auto_rollback_probe.sh"
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    for command in ["curl", "kubectl"]:
        stub = bin_dir / command
        stub.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
        stub.chmod(0o755)

    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}
    result = subprocess.run(
        [
            str(script),
            "--check-only",
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
            "--smoke-url",
            "https://smoke.dev.wms.internal/wms/healthz",
            "--evidence-file",
            str(tmp_path / "wave-1-runtime-evidence.json"),
            "--rollback-log-ref",
            "s3://wms-prod-evidence/wave1/rollback.log",
            "--external-log-ref",
            "s3://wms-dev-stub-evidence/wave1/smoke-alert.log",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    assert result.returncode == 2
    assert "--rollback-log-ref" in result.stderr


@pytest.mark.parametrize(
    ("environment", "target", "expected"),
    [
        ("dev", "k8s", "kubectl rollout undo deployment/wms-api --context wms-dev --namespace wms-dev"),
        ("staging", "docker-compose", "docker WMS_VERSION=previous-staging-sha args=compose -f"),
    ],
)
def test_wave1_auto_rollback_probe_enters_execute_path_on_real_signal_failure(
    tmp_path, monkeypatch, environment, target, expected
):
    """真实 HTTP signal 失败时，probe 才能进入 rollback --execute 路径。"""
    import json
    import os
    import subprocess

    repo_root = Path(__file__).resolve().parents[3]
    script = repo_root / "deploy" / "scripts" / "wave1_auto_rollback_probe.sh"
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    command_log = tmp_path / "rollback-command.log"
    evidence_file = tmp_path / "docs" / "retros" / "wave-1-runtime-evidence.json"
    (bin_dir / "curl").write_text(
        "#!/usr/bin/env bash\nexit 7\n",
        encoding="utf-8",
    )
    (bin_dir / "kubectl").write_text(
        f"#!/usr/bin/env bash\nprintf 'kubectl %s\\n' \"$*\" >> {command_log!s}\n",
        encoding="utf-8",
    )
    (bin_dir / "docker").write_text(
        f"#!/usr/bin/env bash\nprintf 'docker WMS_VERSION=%s args=%s\\n' \"${{WMS_VERSION:-}}\" \"$*\" >> {command_log!s}\n",
        encoding="utf-8",
    )
    (bin_dir / "curl").chmod(0o755)
    (bin_dir / "kubectl").chmod(0o755)
    (bin_dir / "docker").chmod(0o755)

    args = [
        str(script),
        "--environment",
        environment,
        "--target",
        target,
        "--smoke-url",
        f"https://smoke.{environment}.wms.internal/wms/healthz",
        "--curl-max-time",
        "1",
        "--evidence-file",
        str(evidence_file),
        "--rollback-log-ref",
        f"s3://wms-{environment}-evidence/wave1/rollback.log",
        "--external-log-ref",
        f"s3://wms-{environment}-evidence/wave1/smoke-alert.log",
    ]
    if target == "k8s":
        args += ["--context", f"wms-{environment}", "--namespace", f"wms-{environment}"]
    else:
        compose_dir = tmp_path / f"wms-{environment}"
        compose_dir.mkdir()
        compose_file = compose_dir / "compose.yml"
        compose_file.write_text("services: {}\n", encoding="utf-8")
        args += [
            "--previous-version",
            f"previous-{environment}-sha",
            "--compose-file",
            str(compose_file),
        ]

    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}:{env['PATH']}"
    env["WAVE1_ALLOW_LOCAL_TEST_SIGNAL"] = "true"
    result = subprocess.run(
        args,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    assert result.returncode == 1
    assert f"environment={environment} target={target}" in result.stdout
    assert "runtime signal failed; invoking rollback" in result.stdout
    assert expected in command_log.read_text(encoding="utf-8")
    evidence = json.loads(evidence_file.read_text(encoding="utf-8"))
    assert evidence["environment"] == environment
    assert evidence["signal_type"] == "http"
    assert evidence["signal_url"] == f"https://smoke.{environment}.wms.internal/wms/healthz"
    assert evidence["rollback_triggered"] is True
    assert evidence["rollback_exit_code"] == 0
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is True
    assert "自动回滚证据" in message


def test_commit_convention_valid_message():
    """合法的 conventional commit message 应通过。"""
    from check_commit_convention import validate_message
    msg = "功能(入库)：新增 ASN 状态机校验"
    issue = validate_message("abc1234", msg)
    assert issue.issues == []


def test_commit_convention_unknown_type():
    """未知 type 应报错。"""
    from check_commit_convention import validate_message
    msg = "未知类型(入库)：xxx"
    issue = validate_message("abc1234", msg)
    assert any("unknown type" in i for i in issue.issues)


def test_commit_convention_unknown_scope():
    """未知 scope 应报错。"""
    from check_commit_convention import validate_message
    msg = "功能(未知模块)：xxx"
    issue = validate_message("abc1234", msg)
    assert any("unknown scope" in i for i in issue.issues)


def test_commit_convention_too_long_header():
    """超长 header 应报错。"""
    from check_commit_convention import validate_message
    msg = "功能(入库)：" + "x" * 200
    issue = validate_message("abc1234", msg)
    assert any("too long" in i for i in issue.issues)


def test_environment_python_packages_check():
    """validate_environment 的 Python 包检查能跑（不抛异常）。"""
    from validate_environment import check_python_packages
    results = check_python_packages()
    # 至少检查 pathspec / markdown
    names = [r.name for r in results]
    assert "pathspec" in names
    assert "markdown" in names


def test_file_naming_rust_snake():
    """Rust 文件必须 snake_case。"""
    from check_file_naming import check_file
    assert check_file("backend/crates/api/src/main.rs") is None
    assert check_file("backend/crates/api/src/inbound_handler.rs") is None
    v = check_file("backend/crates/api/src/InboundHandler.rs")
    assert v is not None
    assert v.rule == "rust-file-snake"


def test_layer_dependency_detects_forbidden_refs():
    """domain 层禁止引用 api / infra / axum / sqlx。"""
    from check_layer_dependency import find_domain_dependency_issues

    issues = find_domain_dependency_issues(
        "\n".join([
            "use wms_api::ApiDoc;",
            "use axum::Router;",
            "use sqlx::PgPool;",
            "use crate::infra::repo::Repo;",
        ]),
        path="backend/crates/domain/src/lib.rs",
    )

    assert [issue.kind for issue in issues] == ["api", "axum", "sqlx", "infra"]


def test_unsafe_and_unwrap_ignores_comments_and_test_shortcuts():
    """注释中的关键字不误报，测试代码允许 unwrap/expect/panic。"""
    from check_unsafe_and_unwrap import find_unsafe_unwrap_issues

    issues = find_unsafe_unwrap_issues(
        "\n".join([
            "// unsafe { ptr.read() }",
            "/* another unwrap() mention */",
            "#[cfg(test)]",
            "mod tests {",
            "  #[test]",
            "  fn allows_test_shortcuts() {",
            "    let value = result.expect(\"test setup\");",
            "    let other = option.unwrap();",
            "    panic!(\"expected test failure\");",
            "  }",
            "}",
        ]),
        path="backend/crates/api/src/lib.rs",
    )

    assert issues == []


def test_unsafe_and_unwrap_detects_real_production_usage():
    """生产路径 unsafe / unwrap / expect / panic 必须报错。"""
    from check_unsafe_and_unwrap import find_unsafe_unwrap_issues

    issues = find_unsafe_unwrap_issues(
        "\n".join([
            "unsafe { core::ptr::read(p) };",
            "let value = option.unwrap();",
            "let value = result.expect(\"must exist\");",
            "panic!(\"unreachable\");",
        ]),
        path="backend/crates/api/src/lib.rs",
    )

    assert [issue.kind for issue in issues] == ["unsafe", "unwrap", "expect", "panic"]


def test_handler_test_coverage_extracts_unique_paths():
    """utoipa path 抽取应去重。"""
    from check_handler_test_coverage import extract_utoipa_paths

    paths = extract_utoipa_paths(
        '\n'.join([
            '#[utoipa::path(path = "/api/v1/healthz")]',
            '#[utoipa::path(path = "/api/v1/auth/login")]',
            '#[utoipa::path(path = "/api/v1/healthz")]',
        ])
    )

    assert paths == ["/api/v1/healthz", "/api/v1/auth/login"]


def test_handler_test_coverage_requires_path_literals(tmp_path):
    """有 utoipa path 但测试未覆盖 path 字面量时应失败。"""
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate = tmp_path / "api"
    src = api_crate / "src"
    src.mkdir(parents=True)
    (src / "lib.rs").write_text(
        '\n'.join([
            '#[utoipa::path(path = "/api/v1/auth/login")]',
            '#[cfg(test)]',
            'mod tests {',
            '    #[test]',
            '    fn smoke() { assert!(true); }',
            '}',
        ]),
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(src / "lib.rs", api_crate)
    assert [issue.kind for issue in issues] == ["missing_path_coverage"]
    assert stats["path_count"] == 1


def test_handler_test_coverage_requires_every_path(tmp_path):
    """新增 handler 时，不能只靠已覆盖的旧 path 通过 T2。"""
    from check_handler_test_coverage import check_handler_test_coverage

    api_crate = tmp_path / "api"
    src = api_crate / "src"
    src.mkdir(parents=True)
    (src / "lib.rs").write_text(
        '\n'.join([
            '#[utoipa::path(path = "/api/v1/auth/login")]',
            'fn login() {}',
            '#[utoipa::path(path = "/api/v1/auth/me")]',
            'fn me() {}',
            '#[cfg(test)]',
            'mod tests {',
            '    #[test]',
            '    fn covers_login() { assert_eq!("/api/v1/auth/login", "/api/v1/auth/login"); }',
            '}',
        ]),
        encoding="utf-8",
    )

    issues, stats = check_handler_test_coverage(src / "lib.rs", api_crate)
    assert [issue.kind for issue in issues] == ["partial_path_coverage"]
    assert stats["covered_paths"] == ["/api/v1/auth/login"]
    assert stats["missing_paths"] == ["/api/v1/auth/me"]


def test_validate_openapi_document_requires_version_paths_and_schemas():
    """OpenAPI 产物最小结构缺一不可。"""
    from validate_openapi_artifacts import validate_openapi_document

    issues = validate_openapi_document({
        "openapi": "3.1.0",
        "paths": {},
        "components": {"schemas": {}},
    })

    assert [issue.kind for issue in issues] == ["openapi_version", "paths", "schemas"]


def test_check_openapi_contract_detects_missing_401_error_response():
    """非 healthz path 缺少 401 ErrorResponse 应失败。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract({
        "paths": {
            "/api/v1/healthz": {"get": {"responses": {"200": {"description": "ok"}}}},
            "/api/v1/auth/login": {"post": {"responses": {"200": {"description": "ok"}}}},
            "/api/v1/auth/me": {
                "get": {
                    "responses": {
                        "200": {"description": "ok"},
                        "401": {
                            "description": "unauthorized",
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/ErrorResponse"}
                                }
                            },
                        },
                    }
                }
            },
            "/api/v1/audit/events": {
                "get": {
                    "responses": {
                        "200": {"description": "ok"},
                        "401": {
                            "description": "unauthorized",
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/ErrorResponse"}
                                }
                            },
                        },
                    }
                }
            },
        }
    })

    assert any(issue.kind == "missing_401_error_response" for issue in issues)


def test_check_openapi_contract_requires_free_form_json_properties():
    """serde_json::Value 契约必须导出为可承载任意对象属性。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract({
        "paths": {
            "/api/v1/healthz": {"get": {"responses": {"200": {"description": "ok"}}}},
            "/api/v1/auth/login": {
                "post": {
                    "responses": {
                        "200": {"description": "ok"},
                        "401": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/ErrorResponse"}
                                }
                            }
                        },
                    }
                }
            },
            "/api/v1/auth/me": {
                "get": {
                    "responses": {
                        "200": {"description": "ok"},
                        "401": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/ErrorResponse"}
                                }
                            }
                        },
                    }
                }
            },
            "/api/v1/audit/events": {
                "get": {
                    "responses": {
                        "200": {"description": "ok"},
                        "401": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/ErrorResponse"}
                                }
                            }
                        },
                    }
                }
            },
        },
        "components": {
            "schemas": {
                "AuditEvent": {"properties": {"diff": {"type": "object"}}},
                "ErrorResponse": {
                    "properties": {
                        "details": {"type": "object", "additionalProperties": True}
                    }
                },
            }
        },
    })

    assert any(issue.kind == "missing_free_form_object" for issue in issues)


def test_check_openapi_contract_requires_wave2_and_wave3_paths_and_schemas():
    """Wave 2/3 核心合同缺失时必须失败。"""
    from check_openapi_contract import check_openapi_contract

    issues, stats = check_openapi_contract({
        "paths": {
            "/api/v1/healthz": {"get": {"responses": {"200": {"description": "ok"}}}},
            "/api/v1/auth/login": {"post": {"responses": {"200": {"description": "ok"}}}},
            "/api/v1/auth/me": {"get": {"responses": {"200": {"description": "ok"}}}},
            "/api/v1/audit/events": {"get": {"responses": {"200": {"description": "ok"}}}},
        },
        "components": {"schemas": {"ErrorResponse": {"properties": {}}}},
    })

    assert "/api/v1/master-data/products" in stats["required_paths"]
    assert "/api/v1/inbound/receiving-orders/{id}/receive" in stats["required_paths"]
    assert "/api/v1/inventory/batches/status" in stats["required_paths"]
    assert "/api/v1/cold-chain/excursions" in stats["required_paths"]
    assert "/api/v1/billing/contracts" in stats["required_paths"]
    assert "Product" in stats["required_schemas"]
    assert "InventoryBatch" in stats["required_schemas"]
    assert "TemperatureExcursionEvent" in stats["required_schemas"]
    assert "BillingContract" in stats["required_schemas"]
    assert any(issue.kind == "missing_path" for issue in issues)
    assert any(issue.kind == "missing_schema" for issue in issues)


def test_openapi_in_sync_strict_cargo_timeout_fails(tmp_path, monkeypatch, capsys):
    """严格模式下 cargo 超时不能被当作同步通过。"""
    import json
    import subprocess
    import check_openapi_in_sync as check

    backend = tmp_path / "backend"
    backend.mkdir()
    openapi = tmp_path / "openapi.json"
    openapi.write_text('{"openapi":"3.1.0","paths":{},"components":{"schemas":{}}}', encoding="utf-8")
    schema = tmp_path / "schema.ts"
    schema.write_text("export type paths = {};\n", encoding="utf-8")

    monkeypatch.setattr(check, "BACKEND_DIR", backend)
    monkeypatch.setattr(check, "SHARED_OPENAPI", openapi)
    monkeypatch.setattr(check, "API_CLIENT_SCHEMA", schema)

    def fake_run(cmd, **kwargs):
        raise subprocess.TimeoutExpired(cmd=cmd, timeout=kwargs["timeout"])

    monkeypatch.setattr(check.subprocess, "run", fake_run)

    exit_code = check.main(["--strict", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 2
    assert payload["ok"] is False
    assert payload["status"] == "cargo_unavailable"


def test_openapi_in_sync_non_strict_timeout_is_explicitly_degraded(tmp_path, monkeypatch, capsys):
    """非严格模式仍保留本地降级语义，但 JSON 状态必须明确标注未验证。"""
    import json
    import subprocess
    import check_openapi_in_sync as check

    backend = tmp_path / "backend"
    backend.mkdir()
    openapi = tmp_path / "openapi.json"
    openapi.write_text('{"openapi":"3.1.0","paths":{},"components":{"schemas":{}}}', encoding="utf-8")
    schema = tmp_path / "schema.ts"
    schema.write_text("export type paths = {};\n", encoding="utf-8")

    monkeypatch.setattr(check, "BACKEND_DIR", backend)
    monkeypatch.setattr(check, "SHARED_OPENAPI", openapi)
    monkeypatch.setattr(check, "API_CLIENT_SCHEMA", schema)

    def fake_run(cmd, **kwargs):
        raise subprocess.TimeoutExpired(cmd=cmd, timeout=kwargs["timeout"])

    monkeypatch.setattr(check.subprocess, "run", fake_run)

    exit_code = check.main(["--json"])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 0
    assert payload["ok"] is True
    assert payload["status"] == "cargo_unavailable"


def test_openapi_in_sync_strict_schema_timeout_fails(tmp_path, monkeypatch, capsys):
    """严格模式下 schema.ts 生成超时必须阻塞。"""
    import json
    import subprocess
    import check_openapi_in_sync as check

    backend = tmp_path / "backend"
    backend.mkdir()
    document = {"openapi": "3.1.0", "paths": {}, "components": {"schemas": {}}}
    openapi = tmp_path / "openapi.json"
    openapi.write_text(json.dumps(document), encoding="utf-8")
    schema = tmp_path / "schema.ts"
    schema.write_text("export type paths = {};\n", encoding="utf-8")

    monkeypatch.setattr(check, "BACKEND_DIR", backend)
    monkeypatch.setattr(check, "SHARED_OPENAPI", openapi)
    monkeypatch.setattr(check, "API_CLIENT_SCHEMA", schema)

    def fake_run(cmd, **kwargs):
        if cmd[0] == "cargo":
            return subprocess.CompletedProcess(cmd, 0, stdout=json.dumps(document), stderr="")
        raise subprocess.TimeoutExpired(cmd=cmd, timeout=kwargs["timeout"])

    monkeypatch.setattr(check.subprocess, "run", fake_run)

    exit_code = check.main(["--strict", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 2
    assert payload["ok"] is False
    assert payload["status"] == "schema_generator_unavailable"


def test_governance_checks_passes_strict_to_openapi(monkeypatch):
    """T2 全量入口运行 OpenAPI 同步脚本时必须传 --strict。"""
    import subprocess
    import governance_checks as checks

    captured = {}

    def fake_run(cmd, **kwargs):
        captured["cmd"] = cmd
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

    monkeypatch.setattr(checks.subprocess, "run", fake_run)

    result = checks.run_script("check_openapi_in_sync.py", json_mode=True)

    assert result.exit_code == 0
    assert "--strict" in captured["cmd"]
    assert "--json" in captured["cmd"]


def test_governance_checks_t2_failure_is_aggregated(monkeypatch):
    """T2 子脚本失败必须向调度器总退出码传播。"""
    import governance_checks as checks

    monkeypatch.setattr(
        checks,
        "expand_tier_scripts",
        lambda tier: ["check_doc_links.py", "check_openapi_in_sync.py"],
    )

    def fake_run_script(name, *, json_mode):
        exit_code = 1 if name == "check_openapi_in_sync.py" else 0
        return checks.ScriptResult(name=name, exit_code=exit_code, duration_ms=1)

    monkeypatch.setattr(checks, "run_script", fake_run_script)

    assert checks.main(["--tier", "T2"]) == 1


def test_task_check_strict_passes_strict_to_openapi(monkeypatch):
    """diff-driven strict 模式也必须把严格语义传给 OpenAPI 同步脚本。"""
    import subprocess
    import task_check

    captured = {}

    def fake_run(cmd, **kwargs):
        captured["cmd"] = cmd
        return subprocess.CompletedProcess(cmd, 0)

    monkeypatch.setattr(task_check.subprocess, "run", fake_run)

    result = task_check.run_one("check_openapi_in_sync", json_mode=True, strict_mode=True)

    assert result.exit_code == 0
    assert "--strict" in captured["cmd"]
    assert "--json" in captured["cmd"]


def test_file_naming_adr():
    """ADR 必须 NNNN-slug.md。"""
    from check_file_naming import check_file
    assert check_file("docs/adr/0001-tech-stack.md") is None
    v = check_file("docs/adr/tech-stack.md")  # 缺数字编号
    assert v is not None
    assert v.rule == "adr-naming"


def test_file_naming_compliance():
    """合规文档必须 gsp-*.md 或 README.md。"""
    from check_file_naming import check_file
    assert check_file("docs/compliance/gsp-ch5-warehouse.md") is None
    assert check_file("docs/compliance/README.md") is None
    v = check_file("docs/compliance/random-doc.md")
    assert v is not None
    assert v.rule == "compliance-naming"
