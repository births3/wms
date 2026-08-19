"""Wave 6 evidence preflight runbook export 示例测试。"""

from wave6_runbook_test_helpers import collect_single_gate_errors


def test_wave6_evidence_preflight_rejects_hardcoded_export_ref_literals(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 必须发现采集前 export 的硬编码证据引用。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        [
            "```bash",
            "export WAVE_X_LOG_REF='s3://wms-staging-evidence/wave-x/run-20260603.log'",
            "export WAVE_X_DYNAMIC_REF=\"s3://wms-staging-evidence/wave-x/run-$(date -u +%Y%m%d).log\"",
            "```",
        ],
    )

    assert top_errors == []
    assert "export" in joined_errors
    assert "WAVE_X_LOG_REF" in joined_errors
    assert "WAVE_X_DYNAMIC_REF" not in joined_errors


def test_wave6_evidence_preflight_rejects_hardcoded_export_url_file_and_output_literals(
    tmp_path,
    monkeypatch,
):
    """采集前 export 的 URL / FILE / OUTPUT 也必须变量化，不能只检查 REF。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        [
            "```bash",
            "export WAVE_X_DATABASE_URL='postgres://USER:PASS@pg-dev.wms.internal:5432/wms_dev'",
            "export WAVE_X_WRK_OUTPUT='/tmp/wave-x-wrk-dev.log'",
            "export WAVE_X_COMPOSE_FILE='/srv/wms-dev/docker-compose.yml'",
            "export WAVE_X_DYNAMIC_OUTPUT=\"/tmp/wave-x-$(date -u +%Y%m%d).log\"",
            "```",
        ],
    )

    assert top_errors == []
    assert "export" in joined_errors
    assert "WAVE_X_DATABASE_URL" in joined_errors
    assert "WAVE_X_WRK_OUTPUT" in joined_errors
    assert "WAVE_X_COMPOSE_FILE" in joined_errors
    assert "WAVE_X_DYNAMIC_OUTPUT" not in joined_errors


def test_wave6_evidence_preflight_rejects_unquoted_hardcoded_export_ref_literals(
    tmp_path,
    monkeypatch,
):
    """未加引号的 export 证据引用也不能作为可照抄示例。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        [
            "```bash",
            "export WAVE_X_LOG_REF=s3://wms-staging-evidence/wave-x/run-20260603.log",
            "export WAVE_X_DYNAMIC_REF=s3://wms-staging-evidence/wave-x/run-$(date -u +%Y%m%d).log",
            "```",
        ],
    )

    assert top_errors == []
    assert "export" in joined_errors
    assert "WAVE_X_LOG_REF" in joined_errors
    assert "WAVE_X_DYNAMIC_REF" not in joined_errors
