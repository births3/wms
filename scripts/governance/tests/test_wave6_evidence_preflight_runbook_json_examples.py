"""Wave 6 evidence preflight runbook JSON 示例测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import (
    COMMON_TERMS,
    PLACEHOLDER_TERMS,
    write_single_gate_preflight_fixture,
)


def test_wave6_evidence_preflight_requires_json_example_disclaimer(
    tmp_path,
    monkeypatch,
):
    """Evidence JSON 示例含真实形态引用时，必须声明结构示例且不得复制。"""
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        runbook_lines=[
            "## Evidence JSON",
            "```json",
            '{"smoke_log_ref": "ci/staging/wave-x-smoke/123"}',
            "```",
        ],
    )

    top_errors, gate_results = check.collect_results()

    assert top_errors == []
    assert gate_results[0].ok is False
    assert "结构示例" in " ".join(gate_results[0].errors)

    runbook = tmp_path / "docs/runbooks/wave-x-evidence.md"
    runbook.write_text(
        "\n".join([
            "docs/retros/wave-x-evidence.json",
            "wave-x-record",
            "wave-x-validate",
            *COMMON_TERMS,
            *PLACEHOLDER_TERMS,
            "## Evidence JSON",
            "以下 JSON 仅为字段结构示例，不得复制为真实 evidence；真实 evidence 必须由 record 命令生成。",
            "```json",
            '{"smoke_log_ref": "ci/staging/wave-x-smoke/123"}',
            "```",
        ]),
        encoding="utf-8",
    )

    top_errors, gate_results = check.collect_results()

    assert top_errors == []
    assert gate_results[0].ok is True
