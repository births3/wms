"""PDA production gate 基础文档测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_pda_production_gate_help_documents_full_pda_dependency_scope():
    """PDA production gate 的帮助文案必须记录 RN / Expo / EAS / Capacitor 全范围。"""
    import check_pda_production_gate as check

    assert check.__doc__ is not None
    assert "RN / Expo / EAS / Capacitor" in check.__doc__
    assert "RN/Capacitor" not in check.__doc__
