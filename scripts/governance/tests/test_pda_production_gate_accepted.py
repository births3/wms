"""PDA production gate 在 ADR-0027 Accepted 后的基础门禁测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import (
    write_accepted_adr,
    write_rn_spike,
    write_wave3_pda_evidence,
)


def test_pda_production_gate_allows_app_after_adr0027_accepted(tmp_path, monkeypatch):
    """ADR-0027 Accepted 后，apps/pda-mobile 生产文件才允许存在。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(
        check,
        "validate_wave3_evidence",
        lambda: (True, "docs/retros/wave-3-pda-runtime-evidence.json: 内容有效"),
    )

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Accepted\n", encoding="utf-8")

    pda = tmp_path / "apps/pda-mobile"
    pda.mkdir(parents=True)
    (pda / "package.json").write_text("{}", encoding="utf-8")

    result = check.collect_result()

    assert result.ok is True
    assert result.pda_files == ["apps/pda-mobile/package.json"]


def test_pda_production_gate_blocks_adr0027_acceptance_without_runtime_evidence(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 不能在缺少真 PDA runtime evidence 时改成 Accepted。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Accepted\n", encoding="utf-8")

    result = check.collect_result()

    assert result.ok is False
    assert any("runtime evidence" in error for error in result.errors)


def test_pda_production_gate_blocks_adr0027_acceptance_when_runtime_evidence_invalid(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 Accepted 还必须依赖 validator 通过的真 PDA runtime evidence。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(
        check,
        "validate_wave3_evidence",
        lambda: (False, "docs/retros/wave-3-pda-runtime-evidence.json: missing file"),
    )

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Accepted\n", encoding="utf-8")

    result = check.collect_result()

    assert result.ok is False
    assert any("missing file" in error for error in result.errors)


def test_pda_production_gate_requires_matching_spike_accepted_for_runtime_candidate(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 Accepted 时，runtime evidence 指向的 PDA 候选 Spike 必须 accepted。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    write_accepted_adr(tmp_path)
    write_wave3_pda_evidence(tmp_path)

    rn_spike = tmp_path / "docs/spikes/spike-005-rn-scanner.md"
    rn_spike.parent.mkdir(parents=True)
    rn_spike.write_text("- 状态：deferred\n", encoding="utf-8")

    result = check.collect_result()

    assert result.ok is False
    assert any("SPIKE-005" in error and "accepted" in error for error in result.errors)

    write_rn_spike(tmp_path)

    result = check.collect_result()

    assert result.ok is True
