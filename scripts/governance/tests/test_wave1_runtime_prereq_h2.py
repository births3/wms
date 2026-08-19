"""Wave 1 H2 runtime evidence 前置检查测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_runtime_test_helpers import clear_wave1_prereq_env, wave1_prereq_module


def stub_dev_dns(monkeypatch, prereq, ips=None):
    monkeypatch.setattr(prereq, "resolve_host_ips", lambda host: ips or ["10.0.0.8"])


def test_wave1_runtime_prereq_h2_rejects_missing_env(monkeypatch, capsys):
    """H2 前置检查必须先拿到真实采集边界参数。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)

    exit_code = prereq.main(["--mode", "h2"])

    assert exit_code == 2
    err = capsys.readouterr().err
    assert "WAVE1_H2_DATABASE_URL" in err
    assert "WAVE1_H2_WRK_OUTPUT" in err


def test_wave1_runtime_prereq_h2_rejects_local_prod_or_stub_boundaries(
    tmp_path,
    monkeypatch,
    capsys,
):
    """H2 前置检查不能接受本机、生产或 stub 边界。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
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


def test_wave1_runtime_prereq_h2_rejects_local_named_boundary(
    tmp_path,
    monkeypatch,
    capsys,
):
    """H2 前置检查不能接受 local 命名的伪环境边界。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Requests/sec: 1001\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_H2_DATABASE_URL", "postgres://wms@pg-local.wms.internal:5432/wms_dev")
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 2
    assert "local/prod/production" in capsys.readouterr().err


def test_wave1_runtime_prereq_h2_rejects_staging_boundary(
    tmp_path,
    monkeypatch,
    capsys,
):
    """H2 前置检查必须是 dev DB，不能用 staging 边界冒充。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Requests/sec: 1001\n", encoding="utf-8")
    monkeypatch.setenv(
        "WAVE1_H2_DATABASE_URL",
        "postgres://wms@pg-staging.wms.internal:5432/wms_dev",
    )
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv(
        "WAVE1_H2_BENCHMARK_LOG_REF",
        "s3://wms-staging-dev-evidence/wave1/h2/wrk.log",
    )
    monkeypatch.setenv(
        "WAVE1_H2_CRON_LOG_REF",
        "s3://wms-staging-dev-evidence/wave1/h2/cron.log",
    )

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 2
    assert "staging" in capsys.readouterr().err


def test_wave1_runtime_prereq_h2_rejects_raw_ip_even_when_database_name_has_dev(
    tmp_path,
    monkeypatch,
    capsys,
):
    """H2 前置检查只能接受 dev DNS，不能靠数据库名里的 dev 绕过。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Latency Distribution\n  99%  123.45ms\nRequests/sec: 1001\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_H2_DATABASE_URL", "postgres://wms@10.0.0.8:5432/wms_dev")
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 2
    assert "raw IP" in capsys.readouterr().err


def test_wave1_runtime_prereq_h2_allows_dev_h2_alias_for_readiness(
    tmp_path,
    monkeypatch,
):
    """本机 dev-h2 alias 只能用于 readiness dry-run，不要求 wrk 日志已存在。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    monkeypatch.setenv(
        "WAVE1_H2_DATABASE_URL",
        "postgres://wms@dev-h2.wms.internal:15432/wms_dev_h2",
    )
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(tmp_path / "wrk-dev.log"))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")

    exit_code = prereq.main(["--mode", "h2"])

    assert exit_code == 0


def test_wave1_runtime_prereq_h2_rejects_dev_h2_alias_for_formal_evidence(
    tmp_path,
    monkeypatch,
    capsys,
):
    """正式 H2 evidence 采集前置检查必须拒绝本机 dry-run alias。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Latency Distribution\n  99%  123.45ms\nRequests/sec: 1001\n", encoding="utf-8")
    monkeypatch.setenv(
        "WAVE1_H2_DATABASE_URL",
        "postgres://wms@dev-h2.wms.internal:15432/wms_dev_h2",
    )
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 2
    err = capsys.readouterr().err
    assert "dev-h2.wms.internal" in err
    assert "readiness" in err


def test_wave1_runtime_prereq_h2_requires_formal_host_allowlist(
    tmp_path,
    monkeypatch,
    capsys,
):
    """正式 H2 前置检查必须命中 WMS_DEV_DB_HOST_ALLOWLIST。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Latency Distribution\n  99%  123.45ms\nRequests/sec: 1001\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_H2_DATABASE_URL", "postgres://wms@pg-dev-alt.wms.internal:5432/wms_dev")
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")
    monkeypatch.setenv("WMS_DEV_DB_HOST_ALLOWLIST", "pg-dev.wms.internal")

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 2
    assert "WMS_DEV_DB_HOST_ALLOWLIST" in capsys.readouterr().err

    monkeypatch.setenv("WMS_DEV_DB_HOST_ALLOWLIST", "pg-dev.wms.internal,pg-dev-alt.wms.internal")
    stub_dev_dns(monkeypatch, prereq)
    assert prereq.main(["--mode", "h2", "--require-wrk-output"]) == 0


def test_wave1_runtime_prereq_h2_rejects_allowlisted_dev_dns_resolving_to_loopback(
    tmp_path,
    monkeypatch,
    capsys,
):
    """正式 H2 前置检查不能接受解析到 loopback 的 dev DNS。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text("Latency Distribution\n  99%  123.45ms\nRequests/sec: 1001\n", encoding="utf-8")
    monkeypatch.setenv("WAVE1_H2_DATABASE_URL", "postgres://wms@pg-dev.wms.internal:5432/wms_dev")
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")
    monkeypatch.setenv("WMS_DEV_DB_HOST_ALLOWLIST", "pg-dev.wms.internal")
    stub_dev_dns(monkeypatch, prereq, ["127.0.0.1"])

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 2
    assert "loopback" in capsys.readouterr().err


def test_wave1_runtime_prereq_h2_accepts_valid_dev_inputs(tmp_path, monkeypatch):
    """H2 前置检查通过后才允许进入正式 collector。"""
    prereq = wave1_prereq_module(monkeypatch)
    clear_wave1_prereq_env(monkeypatch)
    wrk_output = tmp_path / "wrk-dev.log"
    wrk_output.write_text(
        "Latency Distribution\n  99%  123.45ms\nRequests/sec: 1001.23\n",
        encoding="utf-8",
    )
    monkeypatch.setenv("WAVE1_H2_DATABASE_URL", "postgres://wms@pg-dev.wms.internal:5432/wms_dev")
    monkeypatch.setenv("WAVE1_H2_WRK_OUTPUT", str(wrk_output))
    monkeypatch.setenv("WAVE1_H2_BENCHMARK_LOG_REF", "s3://wms-dev-evidence/wave1/h2/wrk.log")
    monkeypatch.setenv("WAVE1_H2_CRON_LOG_REF", "s3://wms-dev-evidence/wave1/h2/cron.log")
    stub_dev_dns(monkeypatch, prereq)

    exit_code = prereq.main(["--mode", "h2", "--require-wrk-output"])

    assert exit_code == 0
