"""兼容入口：Wave 6 report JSON 测试已按治理主题拆分。"""
from pathlib import Path


def test_wave6_report_json_split_targets_exist():
    """旧入口精确运行时校验拆分目标仍存在，而不是返回 skipped。"""
    test_dir = Path(__file__).resolve().parent
    split_targets = [
        "test_wave6_report_json_contract.py",
        "test_wave6_report_json_complete_contract.py",
        "test_wave6_report_json_gap_normalization.py",
        "test_wave6_report_json_runbook.py",
        "test_wave6_report_gate_metadata.py",
        "test_wave6_report_commands_only.py",
        "test_wave6_report_text.py",
    ]

    missing = [name for name in split_targets if not (test_dir / name).is_file()]
    empty_targets = [
        name
        for name in split_targets
        if f"def test_" not in (test_dir / name).read_text(encoding="utf-8")
    ]

    assert missing == []
    assert empty_targets == []
