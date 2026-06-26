"""PDA production gate 的 WebView/Capacitor 候选一致性测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import (
    write_accepted_adr,
    write_rn_spike,
    write_wave3_pda_evidence,
    write_webview_spike,
)


def test_pda_production_gate_blocks_react_native_dependencies_for_webview_runtime_candidate(
    tmp_path,
    monkeypatch,
):
    """WebView/Capacitor runtime evidence 通过后，生产依赖不能混入 RN/Expo 技术栈。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_accepted_adr(tmp_path)
    write_wave3_pda_evidence(tmp_path, "webview-capacitor")
    write_webview_spike(tmp_path)

    pda_manifest = tmp_path / "apps/pda-mobile/package.json"
    pda_manifest.parent.mkdir(parents=True)
    pda_manifest.write_text(
        json.dumps({
            "dependencies": {
                "eas-cli": "latest",
                "expo": "latest",
                "react-native": "latest",
            },
        }),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert "pda_stack_candidate=webview-capacitor" in " ".join(result.errors)
    assert "eas-cli" in " ".join(result.errors)
    assert "expo" in " ".join(result.errors)
    assert "react-native" in " ".join(result.errors)


def test_pda_production_gate_blocks_eas_cli_script_for_webview_runtime_candidate(
    tmp_path,
    monkeypatch,
):
    """WebView/Capacitor runtime evidence 通过后，EAS Android 脚本不能混入生产链路。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_accepted_adr(tmp_path)
    write_wave3_pda_evidence(tmp_path, "webview-capacitor")
    write_webview_spike(tmp_path)

    pda_manifest = tmp_path / "apps/pda-mobile/package.json"
    pda_manifest.parent.mkdir(parents=True)
    pda_manifest.write_text(
        json.dumps({"scripts": {"android": "eas-cli build --platform android --profile staging"}}),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert "pda_stack_candidate=webview-capacitor" in " ".join(result.errors)
    assert "apps/pda-mobile/package.json:scripts:android:eas-cli build --platform android --profile staging" in (
        result.incompatible_script_entries
    )


def test_pda_production_gate_requires_comparison_when_both_pda_spikes_accepted(
    tmp_path,
    monkeypatch,
):
    """两条 PDA Spike 都 accepted 时，ADR-0027 Accepted 必须记录同口径对比结论。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(
        check,
        "validate_wave3_evidence",
        lambda: (True, "docs/retros/wave-3-pda-runtime-evidence.json: 内容有效"),
    )

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Accepted\n\n## 决策\n\n选择 react-native。\n", encoding="utf-8")

    rn_spike = tmp_path / "docs/spikes/spike-005-rn-scanner.md"
    rn_spike.parent.mkdir(parents=True)
    rn_spike.write_text("- 状态：accepted\n", encoding="utf-8")
    webview_spike = tmp_path / "docs/spikes/spike-005b-webview-capacitor-pda.md"
    webview_spike.write_text("- 状态：accepted\n", encoding="utf-8")

    result = check.collect_result()

    assert result.ok is False
    assert result.spike_statuses == {
        "spike005": "accepted",
        "spike005b": "accepted",
    }
    assert any("同口径对比结论" in error for error in result.errors)

    adr.write_text(
        "\n".join([
            "- 状态：Accepted",
            "",
            "## 同口径对比结论",
            "",
            "SPIKE-005 react-native 与 SPIKE-005B webview-capacitor 使用同一真 PDA、"
            "同一条码样本、同一 M2/M3 dev/staging 测试数据完成对比。",
        ]),
        encoding="utf-8",
    )
    write_rn_spike(tmp_path)
    write_webview_spike(tmp_path)

    result = check.collect_result()

    assert result.ok is True
