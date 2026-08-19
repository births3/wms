"""Wave 1 H2 runtime readiness 测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


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


def test_wave1_h2_runtime_readiness_rejects_raw_ip_with_dev_database_name(monkeypatch):
    """H2 readiness 也只能接受 dev DNS，不能靠 /wms_dev 通过。"""
    import check_wave1_h2_runtime_readiness as readiness

    monkeypatch.setattr(readiness, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(readiness, "count_recent_seals", lambda database_url: 7)

    exit_code = readiness.main([
        "--database-url",
        "postgres://wms@10.0.0.8:5432/wms_dev",
    ])

    assert exit_code == 2


def test_wave1_h2_runtime_readiness_rejects_staging_database(monkeypatch):
    """H2 readiness 必须连接 dev DB，不能接受 staging 命名边界。"""
    import check_wave1_h2_runtime_readiness as readiness

    monkeypatch.setattr(readiness, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(readiness, "count_recent_seals", lambda database_url: 7)

    exit_code = readiness.main([
        "--database-url",
        "postgres://wms@pg-staging.wms.internal:5432/wms_dev",
    ])

    assert exit_code == 2


def test_wave1_h2_runtime_readiness_json_marks_dev_h2_as_dry_run_only(
    monkeypatch,
    capsys,
):
    """dev-h2 alias 只可输出 dry-run 状态，不能被误判为正式 W6.A evidence。"""
    import check_wave1_h2_runtime_readiness as readiness

    monkeypatch.setattr(readiness, "count_audit_rows", lambda database_url: 0)
    monkeypatch.setattr(readiness, "count_recent_seals", lambda database_url: 0)

    exit_code = readiness.main([
        "--database-url",
        "postgres://wms@dev-h2.wms.internal:15432/wms_dev_h2",
        "--dry-run-alias-ok",
        "--json",
    ])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 2
    assert payload["ok"] is False
    assert payload["mode"] == "wave1-h2-runtime-readiness"
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["evidence_file"] == "docs/retros/wave-1-h2-runtime-evidence.json"
    assert payload["dry_run_only"] is True
    assert payload["formal_evidence_allowed"] is False
    assert payload["facts"]["baseline_rows"] == 0
    assert payload["facts"]["consecutive_success_days"] == 0
    assert any("dry-run only" in issue for issue in payload["issues"])
