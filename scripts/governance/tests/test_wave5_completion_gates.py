"""Wave 5 completion gate 治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave5_startup_accepts_current_todo_and_just_targets(monkeypatch):
    """Wave 5 启动证据必须同时有当前 TODO 和 just 入口。"""
    import report_wave5_completion as report

    def fake_read_text(path):
        if path == "TODO.md":
            return "当前 Wave：Wave 5\nW5.A\nW5.B\nW5.C\nW5.D\n"
        if path == "justfile":
            return "wave-5-status:\nwave-5-complete-check:\n"
        return ""

    monkeypatch.setattr(report, "read_text", fake_read_text)

    assert report.wave5_todo_recorded() is True
    startup = report.collect_items()[0]
    assert startup.item_id == "W5-startup"
    assert startup.status == report.PROVED_BY_STATIC_FILES


def test_wave5_startup_accepts_archived_todo_and_just_targets(monkeypatch):
    """Wave 5 完成后切到下一波时，归档 TODO 仍是有效完成证据。"""
    import report_wave5_completion as report

    def fake_read_text(path):
        if path == "TODO.md":
            return "当前 Wave：Wave 6\n已归档：Wave 5\nW5.A\nW5.B\nW5.C\nW5.D\n"
        if path == "justfile":
            return "wave-5-status:\nwave-5-complete-check:\n"
        return ""

    monkeypatch.setattr(report, "read_text", fake_read_text)

    assert report.wave5_todo_recorded() is True
    startup = report.collect_items()[0]
    assert startup.item_id == "W5-startup"
    assert startup.status == report.PROVED_BY_STATIC_FILES


def test_wave5_strict_blocks_when_value_modules_missing(monkeypatch):
    """W5.A-D 未落地时，strict blocking 必须保留。"""
    import report_wave5_completion as report

    monkeypatch.setattr(report, "file_contains", lambda _path, *_needles: False)
    monkeypatch.setattr(report, "file_exists", lambda _path: False)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: False)

    blocking = [item for item in report.collect_items() if item.blocks_strict]

    assert any(item.item_id == "W5.A-packing-station" for item in blocking)
    assert any(item.item_id == "W5.B-retail-chain" for item in blocking)
    assert any(item.item_id == "W5.C-billing-rules" for item in blocking)
    assert any(item.item_id == "W5.D-tms-plus" for item in blocking)
