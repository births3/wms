"""Wave 1 runtime evidence JSON 出口校验测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_runtime_test_helpers import (
    valid_h2_runtime_evidence,
    valid_w1d_runtime_evidence,
    write_runtime_evidence,
)


def test_wave1_completion_report_h2_runtime_evidence_requires_real_dev_record(tmp_path, monkeypatch):
    """H2 runtime 出口证据必须是 dev 的 wrk 1h/60M baseline/7 天封档记录。"""
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "缺少" in message

    evidence = valid_h2_runtime_evidence()
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is True
    assert "真实 PostgreSQL" in message

    evidence["performance"]["duration_seconds"] = 3599
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "duration_seconds" in message


def test_wave1_completion_report_h2_runtime_evidence_rejects_local_or_prod_refs(tmp_path, monkeypatch):
    """H2 runtime 证据不能指向本机或生产边界。"""
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    evidence = valid_h2_runtime_evidence()
    evidence["performance"]["benchmark_log_ref"] = "http://127.0.0.1/wms-dev/wrk.log"
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "非本机 dev" in message


def test_wave1_completion_report_h2_runtime_evidence_requires_database_boundary_snapshot(
    tmp_path,
    monkeypatch,
):
    """H2 runtime JSON 必须事后可审计正式 dev DB host 与非 loopback 解析结果。"""
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    evidence = valid_h2_runtime_evidence()

    evidence.pop("database")
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "database.host" in message

    evidence = valid_h2_runtime_evidence()
    evidence["database"]["host"] = "dev-h2.wms.internal"
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "database.host" in message

    evidence = valid_h2_runtime_evidence()
    evidence["database"]["host"] = "pg-staging-dev.wms.internal"
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "database.host" in message

    evidence = valid_h2_runtime_evidence()
    evidence["database"]["resolved_ips"] = ["127.0.0.1"]
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "resolved_ips" in message

    evidence = valid_h2_runtime_evidence()
    evidence["performance"]["benchmark_log_ref"] = "s3://wms-local-evidence/wave1/h2/wrk.log"
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "非本机 dev" in message

    evidence["performance"]["benchmark_log_ref"] = "s3://wms-prod-evidence/wave1/h2/wrk.log"
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "非本机 dev" in message

    evidence["performance"]["benchmark_log_ref"] = "s3://wms-dev-stub-evidence/wave1/h2/wrk.log"
    write_runtime_evidence(tmp_path, "wave-1-h2-runtime-evidence.json", evidence)

    ok, message = report.valid_h2_runtime_evidence()
    assert ok is False
    assert "非本机 dev" in message


def test_wave1_completion_report_w1d_runtime_evidence_requires_real_signal_record(tmp_path, monkeypatch):
    """W1.D runtime 出口证据必须证明 dev/staging 失败信号触发回滚成功。"""
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is False
    assert "缺少" in message

    evidence = valid_w1d_runtime_evidence()
    write_runtime_evidence(tmp_path, "wave-1-runtime-evidence.json", evidence)

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is True
    assert "自动回滚证据" in message

    evidence["signal_url"] = "http://127.0.0.1/wms-staging/smoke"
    write_runtime_evidence(tmp_path, "wave-1-runtime-evidence.json", evidence)

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is False
    assert "localhost" in message

    evidence["signal_url"] = "https://smoke.local.wms.internal/wms/healthz"
    write_runtime_evidence(tmp_path, "wave-1-runtime-evidence.json", evidence)

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is False
    assert "local" in message

    evidence["signal_url"] = "https://smoke.staging.wms.internal/wms/healthz"
    evidence["rollback_log_ref"] = "s3://wms-evidence/wave1/rollback.log"
    write_runtime_evidence(tmp_path, "wave-1-runtime-evidence.json", evidence)

    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is False
    assert "rollback_log_ref" in message
