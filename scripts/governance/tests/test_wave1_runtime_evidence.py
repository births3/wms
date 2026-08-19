"""Wave 1 完成报告与 runtime evidence 出口聚合治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


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
