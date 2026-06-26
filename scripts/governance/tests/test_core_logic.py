"""兼容入口：core logic 测试已按治理主题拆分到同目录 test_* 文件。"""
from pathlib import Path


def test_core_logic_split_targets_exist():
    """旧入口精确运行时校验拆分目标仍存在，而不是返回 skipped。"""
    test_dir = Path(__file__).resolve().parent
    split_targets = [
        "test_glossary_and_check_data.py",
        "test_baseline_health_governance.py",
        "test_governance_dispatch.py",
        "test_feature_flags_and_changelog.py",
        "test_field_and_business_rules.py",
        "test_openapi_governance.py",
        "test_openapi_sync_governance.py",
        "test_rust_code_quality_governance.py",
        "test_matrix_e2e_governance.py",
        "test_wave1_runtime_evidence.py",
        "test_wave3_pda_readiness.py",
        "test_wave4_completion_evidence.py",
        "test_wave5_completion_gates.py",
        "test_wave6_evidence.py",
        "test_wave6_tooling_evidence.py",
    ]

    missing = [name for name in split_targets if not (test_dir / name).is_file()]
    empty_targets = [
        name
        for name in split_targets
        if f"def test_" not in (test_dir / name).read_text(encoding="utf-8")
    ]

    assert missing == []
    assert empty_targets == []
