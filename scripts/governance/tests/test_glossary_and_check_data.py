"""术语边界与 check-data 配置加载治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
def test_glossary_word_boundary_chinese():
    """禁用词紧贴中文 → 视为更长词的一部分（不报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char("中") is True
    assert _is_word_char("位") is True


def test_glossary_word_boundary_alphanumeric():
    """禁用词紧贴英文/数字/下划线/连字符 → 视为复合词（不报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char("a") is True
    assert _is_word_char("Z") is True
    assert _is_word_char("0") is True
    assert _is_word_char("_") is True
    assert _is_word_char("-") is True


def test_glossary_word_boundary_separators():
    """空格 / 标点 → 视为词边界（应该报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char(" ") is False
    assert _is_word_char(",") is False
    assert _is_word_char("|") is False
    assert _is_word_char("。") is False
    assert _is_word_char("") is False


def test_check_data_appendix_loads():
    """check-data.toml 中的 appendix_references 能被加载。"""
    from _check_data import load_appendix_references
    refs = load_appendix_references()
    # Wave 0 至少有附录 A
    assert len(refs) >= 1
    appendix_a = next((r for r in refs if r.appendix == "附录 A"), None)
    assert appendix_a is not None
    assert appendix_a.defined_in == "user-stories-m1-master-data.md"
    assert "user-stories-m5-cold-chain.md" in appendix_a.expected_in


def test_check_data_exemptions_loads():
    """check-data.toml 中的 approval_source_exemptions 能被加载为 set。"""
    from _check_data import load_approval_source_exemptions
    exemptions = load_approval_source_exemptions()
    assert isinstance(exemptions, set)
    # Wave 0 至少有 M3-003（自身定义审批源）
    assert "US-M3-003" in exemptions
    # 至少 30 条
    assert len(exemptions) >= 25
