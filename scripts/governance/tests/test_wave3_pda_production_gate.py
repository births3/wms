"""Wave 3 PDA production gate 治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave3_pda_production_requires_adr0027_accepted(monkeypatch):
    """生产 PDA app 即使已出现，也必须等 ADR-0027 Accepted 后才能标记完成。"""
    import report_wave3_completion as report

    monkeypatch.setattr(report, "pda_readiness_recorded", lambda: True)
    monkeypatch.setattr(report, "pda_app_started", lambda: True)
    monkeypatch.setattr(report, "collect_key_path_layers", lambda: [])
    monkeypatch.setattr(report, "gsp_qualification_source_frozen", lambda: True)
    monkeypatch.setattr(report, "gsp_qualification_chain_recorded", lambda: True)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)
    monkeypatch.setattr(
        report,
        "read_text",
        lambda path: "- 状态：Proposed\n" if path == "docs/adr/0027-pda-offline-model.md" else "",
    )

    production = {
        item.item_id: item
        for item in report.collect_items()
    }["W3.A-PDA-production"]

    assert production.status == report.PRE_RELEASE_GATE
    assert any("ADR-0027" in gap for gap in production.gaps)


def test_wave3_pda_production_allows_completion_after_adr0027_accepted(monkeypatch):
    """ADR-0027 Accepted、生产 app 已启动且真 PDA evidence 通过时，PDA production 才能完成。"""
    import report_wave3_completion as report

    monkeypatch.setattr(report, "pda_readiness_recorded", lambda: True)
    monkeypatch.setattr(report, "pda_app_started", lambda: True)
    monkeypatch.setattr(
        report,
        "pda_runtime_evidence_status",
        lambda: (True, "docs/retros/wave-3-pda-runtime-evidence.json: 内容有效"),
    )
    monkeypatch.setattr(report, "collect_key_path_layers", lambda: [])
    monkeypatch.setattr(report, "gsp_qualification_source_frozen", lambda: True)
    monkeypatch.setattr(report, "gsp_qualification_chain_recorded", lambda: True)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)
    monkeypatch.setattr(
        report,
        "read_text",
        lambda path: "- 状态：Accepted\n" if path == "docs/adr/0027-pda-offline-model.md" else "",
    )

    production = {
        item.item_id: item
        for item in report.collect_items()
    }["W3.A-PDA-production"]

    assert production.status == report.PROVED_BY_STATIC_FILES
    assert "docs/adr/0027-pda-offline-model.md" in production.evidence
    assert "docs/retros/wave-3-pda-runtime-evidence.json" in production.evidence


def test_wave3_pda_production_requires_runtime_evidence_after_adr0027_accepted(monkeypatch):
    """ADR-0027 Accepted 和 app 文件不足以证明 PDA production，必须有 validator 通过的真机 evidence。"""
    import report_wave3_completion as report

    monkeypatch.setattr(report, "pda_readiness_recorded", lambda: True)
    monkeypatch.setattr(report, "pda_app_started", lambda: True)
    monkeypatch.setattr(
        report,
        "pda_runtime_evidence_status",
        lambda: (False, "missing file: docs/retros/wave-3-pda-runtime-evidence.json"),
    )
    monkeypatch.setattr(report, "collect_key_path_layers", lambda: [])
    monkeypatch.setattr(report, "gsp_qualification_source_frozen", lambda: True)
    monkeypatch.setattr(report, "gsp_qualification_chain_recorded", lambda: True)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)
    monkeypatch.setattr(
        report,
        "read_text",
        lambda path: "- 状态：Accepted\n" if path == "docs/adr/0027-pda-offline-model.md" else "",
    )

    production = {
        item.item_id: item
        for item in report.collect_items()
    }["W3.A-PDA-production"]

    assert production.status == report.PRE_RELEASE_GATE
    assert any("runtime evidence" in gap for gap in production.gaps)
