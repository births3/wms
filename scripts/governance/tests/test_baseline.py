"""tests for _baseline.py"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from _baseline import (
    Baseline,
    BaselineEntry,
    EvaluateResult,
    evaluate,
    format_report,
    load_baseline,
    save_baseline,
)


def test_load_missing_baseline(tmp_path, monkeypatch):
    monkeypatch.setattr("_baseline.BASELINE_DIR", tmp_path)
    b = load_baseline("nonexistent")
    assert b.check == "nonexistent"
    assert b.ignored == []


def test_save_and_load_roundtrip(tmp_path, monkeypatch):
    monkeypatch.setattr("_baseline.BASELINE_DIR", tmp_path)
    b = Baseline(check="test_check", ignored=[
        BaselineEntry(id="file.rs:10", reason="temp", added_at="2026-01-01"),
        BaselineEntry(id="a.rs:1", reason="x", added_at="2026-01-01"),
    ])
    save_baseline(b)
    loaded = load_baseline("test_check")
    assert loaded.check == "test_check"
    # save sorts by id
    assert loaded.ignored[0].id == "a.rs:1"
    assert loaded.ignored[1].id == "file.rs:10"


def test_evaluate_new_violations(tmp_path, monkeypatch):
    monkeypatch.setattr("_baseline.BASELINE_DIR", tmp_path)
    result = evaluate("my_check", ["a.rs:1", "b.rs:2"])
    assert result.new_violations == ["a.rs:1", "b.rs:2"]
    assert result.has_failure


def test_evaluate_all_in_baseline(tmp_path, monkeypatch):
    monkeypatch.setattr("_baseline.BASELINE_DIR", tmp_path)
    b = Baseline(check="my_check", ignored=[
        BaselineEntry(id="a.rs:1", reason="ok", added_at="2026-01-01"),
    ])
    save_baseline(b)
    result = evaluate("my_check", ["a.rs:1"])
    assert result.new_violations == []
    assert result.still_present == ["a.rs:1"]
    assert not result.has_failure


def test_evaluate_auto_shrink(tmp_path, monkeypatch):
    monkeypatch.setattr("_baseline.BASELINE_DIR", tmp_path)
    b = Baseline(check="my_check", ignored=[
        BaselineEntry(id="old.rs:1", reason="fixed", added_at="2026-01-01"),
    ])
    save_baseline(b)
    result = evaluate("my_check", [])  # old.rs:1 已修复
    assert result.resolved == ["old.rs:1"]
    assert not result.has_failure
    # baseline 文件应已收缩
    reloaded = load_baseline("my_check")
    assert reloaded.ignored == []


def test_evaluate_expired_entry(tmp_path, monkeypatch):
    monkeypatch.setattr("_baseline.BASELINE_DIR", tmp_path)
    b = Baseline(check="my_check", ignored=[
        BaselineEntry(id="x.rs:1", reason="temp", added_at="2020-01-01", expires_at="2020-06-01"),
    ])
    save_baseline(b)
    result = evaluate("my_check", ["x.rs:1"])
    assert result.expired == ["x.rs:1"]
    assert result.has_failure


def test_format_report_no_violations():
    result = EvaluateResult(new_violations=[], resolved=[], still_present=[], expired=[])
    report = format_report("test", result)
    assert "no new violations" in report
