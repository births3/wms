"""错误码概览与字典事实源一致性测试。"""

import runpy
from pathlib import Path


MODULE = runpy.run_path("scripts/governance/check_error_codes.py")


def test_error_code_overview_matches_dictionary():
    entries, parse_errors = MODULE["parse_error_codes"]()

    assert not parse_errors
    assert not MODULE["check_overview_counts"](
        entries,
        Path("docs/error-codes.md").read_text(encoding="utf-8"),
    )


def test_error_code_overview_drift_is_reported():
    entries, parse_errors = MODULE["parse_error_codes"]()
    text = Path("docs/error-codes.md").read_text(encoding="utf-8")
    drifted = text.replace(
        f"| **合计** | **{len(entries)}** |",
        "| **合计** | **999** |",
    )

    assert not parse_errors
    assert MODULE["check_overview_counts"](entries, drifted)


def test_error_code_severity_overview_drift_is_reported():
    entries, parse_errors = MODULE["parse_error_codes"]()
    text = Path("docs/error-codes.md").read_text(encoding="utf-8")
    drifted = text.replace("| info | 1 |", "| info | 999 |")

    assert not parse_errors
    assert MODULE["check_overview_counts"](entries, drifted)


def test_error_code_module_overview_drift_is_reported():
    entries, parse_errors = MODULE["parse_error_codes"]()
    text = Path("docs/error-codes.md").read_text(encoding="utf-8")
    drifted = text.replace("| H4 | 14 |", "| H4 | 999 |")

    assert not parse_errors
    assert MODULE["check_overview_counts"](entries, drifted)
