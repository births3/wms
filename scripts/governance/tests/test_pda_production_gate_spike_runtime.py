"""PDA Spike accepted 实测结果门禁测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_pda_production_gate_blocks_pda_spike_accepted_without_runtime_result(
    tmp_path,
    monkeypatch,
):
    """PDA Spike 不能只改状态为 accepted，必须追加真机实测结果段。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Proposed\n", encoding="utf-8")

    rn_spike = tmp_path / "docs/spikes/spike-005-rn-scanner.md"
    rn_spike.parent.mkdir(parents=True)
    rn_spike.write_text(
        "\n".join([
            "# SPIKE-005: React Native 扫枪 + 离线队列",
            "",
            "- 状态：accepted",
            "",
            "## 7. 决策记录",
            "",
            "结论：accepted",
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.spike_statuses["spike005"] == "accepted"
    assert any("SPIKE-005" in error and "实测结果" in error for error in result.errors)

    rn_spike.write_text(
        "\n".join([
            "# SPIKE-005: React Native 扫枪 + 离线队列",
            "",
            "- 状态：accepted",
            "",
            "## 实测结果",
            "",
            "SPIKE-005 react-native 在真 PDA 上使用 dev/staging runtime evidence 验证。",
            "证据引用：docs/retros/wave-3-pda-runtime-evidence.json",
            "扫码方式：physical scan-key intent，覆盖 offline replay、Idempotency-Key replay、audit_event 和 L7 usability review。",
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is True


def test_pda_production_gate_rejects_placeholder_pda_spike_runtime_result(
    tmp_path,
    monkeypatch,
):
    """PDA Spike accepted 的实测结果不能只是待填模板。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Proposed\n", encoding="utf-8")

    rn_spike = tmp_path / "docs/spikes/spike-005-rn-scanner.md"
    rn_spike.parent.mkdir(parents=True)
    rn_spike.write_text(
        "\n".join([
            "# SPIKE-005: React Native 扫枪 + 离线队列",
            "",
            "- 状态：accepted",
            "",
            "## 实测结果",
            "",
            "待填：SPIKE-005 react-native 在真 PDA 上使用 dev/staging runtime evidence 验证。",
            "证据引用：docs/retros/wave-3-pda-runtime-evidence.json",
            "覆盖 offline replay、Idempotency-Key replay、audit_event、L7 和 usability review。",
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert any("占位" in error for error in result.errors)


def test_pda_production_gate_rejects_lowercase_placeholder_pda_spike_result(
    tmp_path,
    monkeypatch,
):
    """PDA Spike accepted 的实测结果也不能保留小写 todo/yyyy/tbd 占位。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Proposed\n", encoding="utf-8")

    rn_spike = tmp_path / "docs/spikes/spike-005-rn-scanner.md"
    rn_spike.parent.mkdir(parents=True)
    rn_spike.write_text(
        "\n".join([
            "# SPIKE-005: React Native 扫枪 + 离线队列",
            "",
            "- 状态：accepted",
            "",
            "## 实测结果",
            "",
            "todo yyyy tbd: SPIKE-005 react-native 在真 PDA 上使用 dev/staging runtime evidence 验证。",
            "证据引用：docs/retros/wave-3-pda-runtime-evidence.json。",
            "覆盖 offline replay、Idempotency-Key replay、audit_event、L7 和 usability review。",
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert any("todo" in error.lower() and "yyyy" in error.lower() for error in result.errors)


def test_pda_production_gate_allows_metric_comparison_in_pda_spike_result(
    tmp_path,
    monkeypatch,
):
    """PDA Spike accepted 的实测结果允许记录 P95 < 500ms 这类真实指标结论。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True)
    adr.write_text("- 状态：Proposed\n", encoding="utf-8")

    rn_spike = tmp_path / "docs/spikes/spike-005-rn-scanner.md"
    rn_spike.parent.mkdir(parents=True)
    rn_spike.write_text(
        "\n".join([
            "# SPIKE-005: React Native 扫枪 + 离线队列",
            "",
            "- 状态：accepted",
            "",
            "## 实测结果",
            "",
            "SPIKE-005 react-native 在真 PDA 上使用 dev/staging runtime evidence 验证。",
            "证据引用：docs/retros/wave-3-pda-runtime-evidence.json。",
            "扫码延迟结论：P95 < 500ms。",
            "覆盖 offline replay、Idempotency-Key replay、audit_event、L7 和 usability review。",
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is True
