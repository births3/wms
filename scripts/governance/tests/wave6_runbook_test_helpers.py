"""Wave 6 evidence preflight runbook test helpers."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import write_single_gate_preflight_fixture


def collect_single_gate_errors(tmp_path, monkeypatch, runbook_lines: list[str]):
    import check_wave6_evidence_preflight as check

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        runbook_lines=runbook_lines,
    )

    top_errors, gate_results = check.collect_results()
    return top_errors, " ".join(gate_results[0].errors)
