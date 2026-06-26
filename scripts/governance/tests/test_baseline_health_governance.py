"""baseline health 快照与增长检测治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
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
