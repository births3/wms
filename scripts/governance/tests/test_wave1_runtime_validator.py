"""Wave 1 runtime evidence validator 边界测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_runtime_test_helpers import (
    valid_h2_runtime_evidence,
    valid_w1d_runtime_evidence,
    write_json,
)


def test_validate_wave1_runtime_evidence_accepts_two_real_records(tmp_path, capsys):
    """定向 validator 应复用出口报告的 H2/W1.D runtime 证据规则。"""
    import validate_wave1_runtime_evidence as validator

    h2_file = tmp_path / "wave-1-h2-runtime-evidence.json"
    write_json(h2_file, valid_h2_runtime_evidence())
    w1d_file = tmp_path / "wave-1-runtime-evidence.json"
    write_json(w1d_file, valid_w1d_runtime_evidence())

    exit_code = validator.main([
        "--kind",
        "all",
        "--h2-file",
        str(h2_file),
        "--w1d-file",
        str(w1d_file),
    ])

    assert exit_code == 0
    out = capsys.readouterr().out
    assert "H2 runtime evidence 内容有效" in out
    assert "W1.D runtime evidence 内容有效" in out


def test_validate_wave1_runtime_evidence_rejects_fake_h2_boundary(tmp_path, capsys):
    """定向 validator 不能接受 fake/stub 边界。"""
    import validate_wave1_runtime_evidence as validator

    h2_evidence = valid_h2_runtime_evidence()
    h2_evidence["performance"]["benchmark_log_ref"] = (
        "s3://wms-dev-fake-evidence/wave1/h2/wrk.log"
    )
    h2_file = tmp_path / "wave-1-h2-runtime-evidence.json"
    write_json(h2_file, h2_evidence)

    exit_code = validator.main(["--kind", "h2", "--h2-file", str(h2_file)])

    assert exit_code == 1
    assert "benchmark_log_ref" in capsys.readouterr().out


def test_validate_wave1_runtime_evidence_rejects_staging_h2_refs(tmp_path, capsys):
    """H2 validator 不能接受 staging 命名的 dev evidence 引用。"""
    import validate_wave1_runtime_evidence as validator

    h2_evidence = valid_h2_runtime_evidence()
    h2_evidence["performance"]["benchmark_log_ref"] = (
        "s3://wms-staging-dev-evidence/wave1/h2/wrk.log"
    )
    h2_evidence["seal_cron"]["cron_log_ref"] = (
        "s3://wms-staging-dev-evidence/wave1/h2/seal-cron.log"
    )
    h2_file = tmp_path / "wave-1-h2-runtime-evidence.json"
    write_json(h2_file, h2_evidence)

    exit_code = validator.main(["--kind", "h2", "--h2-file", str(h2_file)])

    assert exit_code == 1
    assert "staging" in capsys.readouterr().out


def test_validate_wave1_runtime_evidence_rejects_example_refs_unless_explicitly_allowed(
    tmp_path,
):
    """正式 evidence 不能复制 .example.json 模板引用；模板自检必须显式豁免。"""
    import validate_wave1_runtime_evidence as validator

    w1d_file = tmp_path / "wave-1-runtime-evidence.example.json"
    w1d_file.write_text(
        json.dumps({
            "environment": "staging",
            "captured_at": "2026-06-03T12:00:00+08:00",
            "signal_type": "prometheus",
            "signal_url": "https://prometheus.staging.example.com/api/v1/query",
            "rollback_triggered": True,
            "rollback_exit_code": 0,
            "rollback_log_ref": "s3://wms-staging-evidence/wave1/rollback.log",
            "external_log_ref": "s3://wms-staging-evidence/wave1/monitoring-alert.log",
        }),
        encoding="utf-8",
    )

    assert validator.main(["--kind", "w1d", "--w1d-file", str(w1d_file)]) == 1
    assert validator.main([
        "--kind",
        "w1d",
        "--w1d-file",
        str(w1d_file),
        "--allow-example-refs",
    ]) == 0


def test_validate_wave1_runtime_evidence_rejects_placeholder_values(tmp_path, capsys):
    """Wave 1 runtime evidence 不能保留 YYYY / <...> / 待填等模板占位。"""
    import validate_wave1_runtime_evidence as validator

    h2_evidence = valid_h2_runtime_evidence()
    h2_evidence["performance"]["benchmark_log_ref"] = (
        "s3://wms-dev-evidence/wave1/h2/wrk-YYYYMMDD.log"
    )
    h2_file = tmp_path / "wave-1-h2-runtime-evidence.json"
    write_json(h2_file, h2_evidence)

    assert validator.main(["--kind", "h2", "--h2-file", str(h2_file)]) == 1
    h2_output = capsys.readouterr().out
    assert "占位" in h2_output
    assert "benchmark_log_ref" in h2_output

    w1d_evidence = valid_w1d_runtime_evidence()
    w1d_evidence["rollback_log_ref"] = (
        "s3://wms-staging-evidence/wave1/rollback-<run-id>.log"
    )
    w1d_file = tmp_path / "wave-1-runtime-evidence.json"
    write_json(w1d_file, w1d_evidence)

    assert validator.main(["--kind", "w1d", "--w1d-file", str(w1d_file)]) == 1
    w1d_output = capsys.readouterr().out
    assert "占位" in w1d_output
    assert "rollback_log_ref" in w1d_output
