"""Wave 3 completion report readiness 与 GSP 治理测试。

从 test_pda_production_gate.py 拆出，覆盖 Wave 3 completion report 的
PDA readiness 出口与 GSP 来源门禁。
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave3_key_path_report_keeps_l7_as_explicit_gap(monkeypatch):
    """Wave 3 11 层报告不能替用户发明 L7 性能/易用性阈值。"""
    import report_wave3_completion as report

    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "file_exists", lambda _path: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)

    layers = {layer.layer_id: layer for layer in report.collect_key_path_layers()}

    assert layers["L1"].complete is True
    assert layers["L2"].complete is True
    assert layers["L7"].complete is False
    assert layers["L7"].strict_blocking is False
    assert "预发布 gate" in " ".join(layers["L7"].gaps)


def test_wave3_gsp_source_uses_latest_decision(monkeypatch):
    """GSP 资质来源门禁应以后续最新决策覆盖历史占位记录。"""
    import report_wave3_completion as report

    monkeypatch.setattr(
        report,
        "read_text",
        lambda _path: "\n".join([
            "| 64 | GSP 资质有效期校验来源 | 继续保留接口占位，来源未冻结 |",
            "| 69 | GSP 资质有效期校验来源 | 冻结为 M1 本地资质档案 + M-VR 校验规则执行 |",
        ]),
    )

    assert report.gsp_qualification_source_frozen() is True


def test_wave3_gsp_gate_requires_recorded_chain(monkeypatch):
    """GSP 资质来源不能只靠澄清表一句话，M1/M2/M-VR/GSP 链路也要可追溯。"""
    import report_wave3_completion as report

    monkeypatch.setattr(report, "pda_readiness_recorded", lambda: True)
    monkeypatch.setattr(report, "pda_app_started", lambda: False)
    monkeypatch.setattr(report, "collect_key_path_layers", lambda: [])
    monkeypatch.setattr(report, "gsp_qualification_source_frozen", lambda: True)
    monkeypatch.setattr(report, "gsp_qualification_chain_recorded", lambda: False)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)

    item = {
        item.item_id: item
        for item in report.collect_items()
    }["W3-GSP-qualification-source"]

    assert item.blocks_strict is True


def test_wave3_pda_readiness_blocks_strict_when_missing(monkeypatch):
    """PDA production 可延后，但 readiness/runbook 本身必须阻断 Wave 3 出口。"""
    import report_wave3_completion as report

    monkeypatch.setattr(report, "pda_readiness_recorded", lambda: False)
    monkeypatch.setattr(report, "pda_app_started", lambda: False)
    monkeypatch.setattr(report, "collect_key_path_layers", lambda: [])
    monkeypatch.setattr(report, "gsp_qualification_source_frozen", lambda: True)
    monkeypatch.setattr(report, "file_contains", lambda _path, *needles: True)
    monkeypatch.setattr(report, "openapi_has", lambda _paths, _schemas: True)

    readiness = {
        item.item_id: item
        for item in report.collect_items()
    }["W3.A-PDA-readiness"]

    assert readiness.blocks_strict is True
