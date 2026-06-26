"""Runtime evidence validator help 文案边界测试。"""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_validate_wave3_pda_runtime_help_keeps_placeholder_boundary_clear(capsys):
    """--allow-example-refs 只放行 example token，不能暗示模板占位可放行。"""
    import validate_wave3_pda_runtime_evidence as validator

    with pytest.raises(SystemExit) as exit_info:
        validator.main(["--help"])

    assert exit_info.value.code == 0
    help_text = capsys.readouterr().out
    normalized_help = " ".join(help_text.split())
    assert "Allow refs containing example domain tokens" in normalized_help
    assert "template placeholders are still rejected" in normalized_help
    assert "example placeholder references" not in normalized_help


@pytest.mark.parametrize(
    "module_name",
    [
        "validate_wave1_runtime_evidence",
        "validate_wave3_pda_runtime_evidence",
        "validate_wave4_external_dependencies",
        "validate_wave5_hardware_evidence",
        "validate_wave5_tms_evidence",
        "validate_wave6_deploy_evidence",
    ],
)
def test_runtime_evidence_allow_example_help_only_mentions_example_domain_tokens(
    module_name,
    capsys,
):
    """runtime evidence validator 的 example 豁免文案不能暗示模板占位会被放行。"""
    validator = __import__(module_name)

    with pytest.raises(SystemExit) as exit_info:
        validator.main(["--help"])

    assert exit_info.value.code == 0
    normalized_help = " ".join(capsys.readouterr().out.split())
    assert "Allow refs containing example domain tokens" in normalized_help
    assert "template placeholders are still rejected" in normalized_help
    assert "placeholder references" not in normalized_help
