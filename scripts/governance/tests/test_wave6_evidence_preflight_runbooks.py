"""Wave 6 evidence preflight runbook 示例命令测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_runbook_test_helpers import collect_single_gate_errors


def test_wave6_evidence_preflight_rejects_placeholder_refs_in_runbook_code(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 必须发现 runbook 示例命令里的证据引用占位符。"""
    top_errors, joined_errors = collect_single_gate_errors(
        tmp_path,
        monkeypatch,
        [
            "```json",
            '{"evidence_ref": "s3://wms-staging-evidence/wave-x/run-YYYYMMDD.log"}',
            "```",
        ],
    )

    assert top_errors == []
    assert "示例证据引用" in joined_errors
    assert "YYYY" in joined_errors
