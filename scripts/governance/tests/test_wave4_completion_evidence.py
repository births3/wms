"""Wave 4 completion report 与外部 evidence 决策测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave4_short_pick_decision_blocks_when_pending(monkeypatch):
    """M4 文档要求 #43 关闭后才能进入 TDD，待定状态必须阻断 Wave 4。"""
    import report_wave4_completion as report

    monkeypatch.setattr(
        report,
        "read_text",
        lambda _path: "| 43 | 短拣后是否允许少量发货 | 待定，进入 M4 TDD 前必须确认 |",
    )

    assert report.short_pick_decision_closed() is False


def test_wave4_scope_alignment_detects_w4f_mismatch(monkeypatch):
    """ROADMAP 和依赖图对 W4.F 是否纳入 Wave 4 必须一致。"""
    import report_wave4_completion as report

    def fake_read_text(path):
        if path == "ROADMAP.md":
            return "W4.A W4.B W4.C W4.D W4.E"
        if path == "docs/architecture-dependencies.md":
            return "W4.A W4.B W4.C W4.D W4.E W4.F"
        return ""

    monkeypatch.setattr(report, "read_text", fake_read_text)

    assert report.wave4_scope_aligned() is False


def test_wave4_startup_item_accepts_archived_todo(monkeypatch):
    """Wave 4 已归档后，完成门禁仍应可复跑。"""
    import report_wave4_completion as report

    def fake_read_text(path):
        if path == "TODO.md":
            return "## 已归档：Wave 4\nW4.A W4.B W4.C W4.D W4.E"
        return ""

    monkeypatch.setattr(report, "read_text", fake_read_text)

    assert report.wave4_todo_started() is True


def test_wave4_startup_item_blocks_when_todo_not_recorded(monkeypatch):
    """Wave 4 未登记时，TODO 和 just 入口缺失不能被报告成完成。"""
    import report_wave4_completion as report

    monkeypatch.setattr(report, "wave4_todo_started", lambda: False)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "wave4_scope_aligned", lambda: True)
    monkeypatch.setattr(report, "short_pick_decision_closed", lambda: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)
    monkeypatch.setattr(report, "file_exists", lambda _path: True)

    startup = {
        item.item_id: item
        for item in report.collect_items()
    }["W4-startup"]

    assert startup.blocks_strict is True


def test_wave4_traceability_external_contract_blocks_when_unconfirmed(monkeypatch):
    """M-TC 内部三元组契约不能替代码上放心正式接口确认。"""
    import report_wave4_completion as report

    def fake_read_text(path):
        if path == "ROADMAP.md":
            return '"码上放心"正式接口文档 / 鉴权方式 / 错误码 / 频率限制确认 | M-TC（Wave 4） | Wave 2 启动时 | 未确认 |'
        if path == "docs/domain/user-stories-mtc-traceability-code.md":
            return "待接口确认"
        return ""

    monkeypatch.setattr(report, "read_text", fake_read_text)
    monkeypatch.setattr(report, "file_exists", lambda _path: True)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)
    monkeypatch.setattr(report, "wave4_scope_aligned", lambda: True)
    monkeypatch.setattr(report, "short_pick_decision_closed", lambda: True)

    traceability = {
        item.item_id: item
        for item in report.collect_items()
    }["W4.D-traceability-code-reporting"]

    assert traceability.blocks_strict is True
    assert "正式接口文档" in " ".join(traceability.gaps)
    assert "wave-4-external-dependencies.md" in " ".join(traceability.gaps)


def test_wave4_traceability_external_contract_requires_evidence_json(monkeypatch, tmp_path):
    """ROADMAP 关闭外部依赖文字后，仍必须有真实 evidence JSON 才能过 W4.D。"""
    import report_wave4_completion as report

    def fake_read_text(path):
        if path == "ROADMAP.md":
            return '"码上放心"正式接口文档 / 鉴权方式 / 错误码 / 频率限制确认 | M-TC（Wave 4） | 已确认 |'
        if path == "docs/domain/user-stories-mtc-traceability-code.md":
            return "正式接口已确认"
        return ""

    monkeypatch.setattr(report, "read_text", fake_read_text)
    monkeypatch.setattr(report, "WAVE4_EXTERNAL_EVIDENCE", tmp_path / "missing.json")

    assert report.traceability_external_contract_ready() is False


def test_wave4_traceability_deferred_decision_allows_completion_without_fake_evidence(monkeypatch):
    """仅在澄清记录明确延期时，W4.D 可不因外部 evidence 阻塞 Wave 4。"""
    import report_wave4_completion as report

    monkeypatch.setattr(report, "file_exists", lambda _path: True)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)
    monkeypatch.setattr(report, "traceability_external_contract_ready", lambda: False)
    monkeypatch.setattr(report, "traceability_external_evidence_deferred", lambda: True)

    items = report.collect_items()
    traceability = next(
        item for item in items if item.item_id == "W4.D-traceability-code-reporting"
    )

    assert traceability.status == report.ACCEPTED_DEFERRED
    assert traceability.complete is True
    assert traceability.blocks_strict is False
    assert "不伪造 evidence" in " ".join(traceability.gaps)


def test_wave4_traceability_deferred_requires_explicit_clarification(monkeypatch):
    """没有明确 #50 决策时，不能把缺失 evidence 自动视为延期。"""
    import report_wave4_completion as report

    monkeypatch.setattr(
        report,
        "latest_clarification_row",
        lambda _question: "| 50 | W4.D 码上放心外部 evidence 延期 | 后续处理 |",
    )

    assert report.traceability_external_evidence_deferred() is False


def test_wave4_traceability_ignores_unrelated_open_external_dependencies(monkeypatch):
    """W4.D 只检查码上放心两行状态，不能被其他外部依赖的未启动误阻塞。"""
    import report_wave4_completion as report

    def fake_read_text(path):
        if path == "ROADMAP.md":
            return "\n".join([
                '| "码上放心"账号开通 | M-TC（Wave 4） | Wave 2 启动时 | 已开通 |',
                '| "码上放心"正式接口文档 / 鉴权方式 / 错误码 / 频率限制确认 | M-TC（Wave 4） | Wave 2 启动时 | 已确认 |',
                "| 车辆 GPS / 电子地图 API | M10（Wave 5）| Wave 4 启动时 | 未启动 |",
            ])
        if path == "docs/domain/user-stories-mtc-traceability-code.md":
            return "正式接口已确认"
        return ""

    monkeypatch.setattr(report, "read_text", fake_read_text)
    monkeypatch.setattr(
        report,
        "validate_wave4_external_evidence",
        lambda _path, *, allow_example_refs: (True, "valid"),
    )

    assert report.traceability_external_contract_ready() is True
