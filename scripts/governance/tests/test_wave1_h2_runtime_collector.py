"""Wave 1 H2 runtime collector 测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def stub_dev_dns(monkeypatch, collector):
    monkeypatch.setattr(collector, "resolve_host_ips", lambda host: ["10.0.0.8"])


def test_collect_wave1_h2_runtime_evidence_writes_valid_json(tmp_path, monkeypatch):
    """H2 collector 应从真实 wrk 输出和 DB 统计生成可被出口报告接受的 JSON。"""
    import collect_wave1_h2_runtime_evidence as collector
    import report_wave1_completion as report

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Running 1h test @ http://wms-dev.internal/api/v1/audit\n"
        "Latency Distribution\n"
        "  50%    10.00ms\n"
        "  99%   123.45ms\n"
        "Requests/sec:   1001.23\n",
        encoding="utf-8",
    )
    output = tmp_path / "docs" / "retros" / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)
    stub_dev_dns(monkeypatch, collector)

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
    payload = json.loads(output.read_text(encoding="utf-8"))
    assert payload["database"] == {
        "host": "pg-dev.wms.internal",
        "resolved_ips": ["10.0.0.8"],
    }
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    ok, message = report.valid_h2_runtime_evidence()
    assert ok is True
    assert "真实 PostgreSQL" in message


def test_collect_wave1_h2_runtime_evidence_requires_force_to_overwrite_existing_file(
    tmp_path,
    monkeypatch,
):
    """H2 collector 不能静默覆盖已有真实 runtime evidence。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Running 1h test @ http://wms-dev.internal/api/v1/audit\n"
        "Latency Distribution\n"
        "  99%   123.45ms\n"
        "Requests/sec:   1001.23\n",
        encoding="utf-8",
    )
    output = tmp_path / "wave-1-h2-runtime-evidence.json"
    output.write_text("{}", encoding="utf-8")
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)
    stub_dev_dns(monkeypatch, collector)

    command = [
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
    ]

    assert collector.main(command) == 2
    assert output.read_text(encoding="utf-8") == "{}"
    assert collector.main([*command, "--force"]) == 0
    assert '"environment": "dev"' in output.read_text(encoding="utf-8")


def test_collect_wave1_h2_runtime_evidence_rejects_short_or_slow_runs(
    tmp_path,
    monkeypatch,
):
    """H2 collector 不能为短跑或低吞吐生成出口证据。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Running 1h test @ http://wms-dev.internal/api/v1/audit\n"
        "Latency Distribution\n"
        "  99%   123.45ms\n"
        "Requests/sec:    999.99\n",
        encoding="utf-8",
    )
    output = tmp_path / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)
    stub_dev_dns(monkeypatch, collector)

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


def test_collect_wave1_h2_runtime_evidence_rejects_short_wrk_log_duration(
    tmp_path,
    monkeypatch,
):
    """H2 collector 必须从 wrk 原始输出反查 1 小时持续时间。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Running 1m test @ http://wms-dev.internal/api/v1/audit\n"
        "Latency Distribution\n"
        "  99%   123.45ms\n"
        "Requests/sec:   1001.23\n",
        encoding="utf-8",
    )
    output = tmp_path / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)
    stub_dev_dns(monkeypatch, collector)

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


def test_collect_wave1_h2_runtime_evidence_accepts_equivalent_one_hour_wrk_duration_units():
    """H2 collector 应识别 wrk 的 60m / 3600s 等价 1 小时写法。"""
    import collect_wave1_h2_runtime_evidence as collector

    assert (
        collector.parse_wrk_duration_seconds(
            "Running 60m test @ http://wms-dev.internal/api/v1/audit"
        )
        == 3600
    )
    assert (
        collector.parse_wrk_duration_seconds(
            "Running 3600s test @ http://wms-dev.internal/api/v1/audit"
        )
        == 3600
    )


def test_collect_wave1_h2_runtime_evidence_rejects_local_database_url(
    tmp_path,
    monkeypatch,
):
    """H2 collector 不能从本机 PostgreSQL 生成 Wave 1 runtime 出口证据。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Running 1h test @ http://wms-dev.internal/api/v1/audit\n"
        "Latency Distribution\n"
        "  99%   123.45ms\n"
        "Requests/sec:   1001.23\n",
        encoding="utf-8",
    )
    output = tmp_path / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)
    stub_dev_dns(monkeypatch, collector)

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


def test_collect_wave1_h2_runtime_evidence_rejects_dev_h2_dry_run_alias(
    tmp_path,
    monkeypatch,
):
    """H2 collector 不能把本机 readiness alias 写成正式 runtime evidence。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Running 1h test @ http://wms-dev.internal/api/v1/audit\n"
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
        "postgres://wms@dev-h2.wms.internal:15432/wms_dev_h2",
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


def test_collect_wave1_h2_runtime_evidence_rejects_raw_ip_even_when_database_name_has_dev(
    tmp_path,
    monkeypatch,
):
    """H2 collector 只能接受 dev DNS，不能靠 /wms_dev 绕过 host 边界。"""
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
        "postgres://wms@10.0.0.8:5432/wms_dev",
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


def test_collect_wave1_h2_runtime_evidence_requires_formal_host_allowlist(
    tmp_path,
    monkeypatch,
):
    """正式 H2 collector 必须命中 WMS_DEV_DB_HOST_ALLOWLIST。"""
    import collect_wave1_h2_runtime_evidence as collector

    wrk_output = tmp_path / "wrk.log"
    wrk_output.write_text(
        "Running 1h test @ http://wms-dev.internal/api/v1/audit\n"
        "Latency Distribution\n"
        "  99%   123.45ms\n"
        "Requests/sec:   1001.23\n",
        encoding="utf-8",
    )
    output = tmp_path / "wave-1-h2-runtime-evidence.json"
    monkeypatch.setattr(collector, "count_audit_rows", lambda database_url: 60_000_000)
    monkeypatch.setattr(collector, "count_recent_seals", lambda database_url: 7)
    monkeypatch.setenv("WMS_DEV_DB_HOST_ALLOWLIST", "pg-dev.wms.internal")
    stub_dev_dns(monkeypatch, collector)

    command = [
        "--database-url",
        "postgres://wms@pg-dev-alt.wms.internal/wms_dev",
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
    ]

    assert collector.main(command) == 2
    assert not output.exists()

    monkeypatch.setenv("WMS_DEV_DB_HOST_ALLOWLIST", "pg-dev.wms.internal,pg-dev-alt.wms.internal")
    assert collector.main(command) == 0


def test_collect_wave1_h2_runtime_evidence_rejects_allowlisted_dev_dns_resolving_to_loopback(
    tmp_path,
    monkeypatch,
):
    """正式 H2 collector 不能接受解析到 loopback 的 dev DNS。"""
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
    stub_dev_dns(monkeypatch, collector)
    monkeypatch.setattr(collector, "resolve_host_ips", lambda host: ["127.0.0.1"])
    monkeypatch.setenv("WMS_DEV_DB_HOST_ALLOWLIST", "pg-dev.wms.internal")

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


def test_collect_wave1_h2_runtime_evidence_rejects_local_named_log_ref(
    tmp_path,
    monkeypatch,
):
    """H2 collector 不能写入 local 命名的证据引用。"""
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
        "postgres://wms@pg-dev.wms.internal/wms_dev",
        "--wrk-output",
        str(wrk_output),
        "--benchmark-log-ref",
        "s3://wms-local-evidence/wave1/h2/wrk.log",
        "--cron-log-ref",
        "s3://wms-dev-evidence/wave1/h2/seal-cron.log",
        "--duration-seconds",
        "3600",
        "--output",
        str(output),
    ])

    assert exit_code == 2
    assert not output.exists()


def test_collect_wave1_h2_runtime_evidence_rejects_staging_boundary(
    tmp_path,
    monkeypatch,
):
    """H2 collector 不能从 staging DB/log 边界生成 dev runtime evidence。"""
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
        "postgres://wms@pg-staging.wms.internal/wms_dev",
        "--wrk-output",
        str(wrk_output),
        "--benchmark-log-ref",
        "s3://wms-staging-dev-evidence/wave1/h2/wrk.log",
        "--cron-log-ref",
        "s3://wms-staging-dev-evidence/wave1/h2/seal-cron.log",
        "--duration-seconds",
        "3600",
        "--output",
        str(output),
    ])

    assert exit_code == 2
    assert not output.exists()
