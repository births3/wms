"""兼容入口：runtime evidence 测试已按治理主题拆分到同目录 test_* 文件。"""
from pathlib import Path


def test_runtime_evidence_split_targets_exist():
    """旧入口精确运行时校验拆分目标仍存在，而不是返回 skipped。"""
    test_dir = Path(__file__).resolve().parent
    split_targets = [
        "test_runtime_evidence_boundary_messages.py",
        "test_runtime_evidence_help_text.py",
        "test_runtime_evidence_placeholder_values.py",
        "test_runtime_evidence_recorders.py",
    ]

    missing = [name for name in split_targets if not (test_dir / name).is_file()]
    empty_targets = [
        name
        for name in split_targets
        if f"def test_" not in (test_dir / name).read_text(encoding="utf-8")
    ]

    assert missing == []
    assert empty_targets == []
