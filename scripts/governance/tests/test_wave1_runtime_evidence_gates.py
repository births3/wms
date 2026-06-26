"""Wave 1 runtime evidence 预发布 gate 聚合测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


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
