"""Wave 6 evidence preflight runbook record 示例测试。"""
import pytest

from wave6_runbook_test_helpers import collect_single_gate_errors


@pytest.mark.parametrize(
    ("runbook_lines", "expected_env_vars"),
    [
        pytest.param(
            [
                "```bash",
                "just wave-x-record \\",
                "  --smoke-log-ref 'ci/staging/wave-x-smoke/123' \\",
                "  --audit-event-query-ref \"$WAVE_X_AUDIT_EVENT_QUERY_REF\"",
                "```",
            ],
            ["WAVE_X_SMOKE_LOG_REF"],
            id="quoted-ref",
        ),
        pytest.param(
            [
                "```bash",
                "just wave-x-record \\",
                "  --smoke-log-ref ci/staging/wave-x-smoke/123 \\",
                "  --audit-event-query-ref \"$WAVE_X_AUDIT_EVENT_QUERY_REF\"",
                "```",
            ],
            ["WAVE_X_SMOKE_LOG_REF"],
            id="unquoted-ref",
        ),
        pytest.param(
            [
                "```bash",
                "just wave-x-record \\",
                "  --service-url https://wms-staging.example/api \\",
                "  --wrk-output docs/retros/wave-x-wrk.log \\",
                "  --audit-event-query-ref \"$WAVE_X_AUDIT_EVENT_QUERY_REF\"",
                "```",
            ],
            ["WAVE_X_SERVICE_URL", "WAVE_X_WRK_OUTPUT"],
            id="url-file-output",
        ),
    ],
)
def test_wave6_evidence_preflight_rejects_hardcoded_record_ref_literals(
    tmp_path,
    monkeypatch,
    runbook_lines,
    expected_env_vars,
):
    """Wave 6 preflight 必须发现 record 命令中可被照抄的硬编码证据引用。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        runbook_lines,
    )

    assert top_errors == []
    assert "record 命令" in joined_errors
    for env_var in expected_env_vars:
        assert env_var in joined_errors


def test_wave6_evidence_preflight_accepts_record_url_file_and_output_env_vars(
    tmp_path,
    monkeypatch,
):
    """record 命令使用现场环境变量传入 URL、文件和输出引用时不报硬编码错误。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        [
            "```bash",
            "just wave-x-record \\",
            "  --service-url \"$WAVE_X_SERVICE_URL\" \\",
            "  --wrk-output \"$WAVE_X_WRK_OUTPUT\" \\",
            "  --audit-event-query-ref \"$WAVE_X_AUDIT_EVENT_QUERY_REF\"",
            "```",
        ],
    )

    assert top_errors == []
    assert "record 命令" not in joined_errors


def test_wave6_evidence_preflight_ignores_commented_record_examples(
    tmp_path,
    monkeypatch,
):
    """注释掉的 record 示例不能触发硬编码引用或 --force 检查。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        [
            "```bash",
            "# just wave-x-record --force \\",
            "#   --smoke-log-ref ci/staging/wave-x-smoke/123",
            "just wave-x-record \\",
            "  --smoke-log-ref \"$WAVE_X_SMOKE_LOG_REF\"",
            "```",
        ],
    )

    assert top_errors == []
    assert "--force" not in joined_errors
    assert "record 命令" not in joined_errors


def test_wave6_evidence_preflight_rejects_force_in_record_examples(
    tmp_path,
    monkeypatch,
):
    """Wave 6 runbook 示例不能鼓励用 --force 覆盖真实 evidence。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        [
            "```bash",
            "just wave-x-record --force \\",
            "  --smoke-log-ref \"$WAVE_X_SMOKE_LOG_REF\"",
            "```",
        ],
    )

    assert top_errors == []
    assert "--force" in joined_errors
    assert "不能使用" in joined_errors
