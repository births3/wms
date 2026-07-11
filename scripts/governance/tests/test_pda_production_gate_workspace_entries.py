"""PDA production gate 对 pnpm workspace 条目的阻断测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import write_proposed_adr


def test_pda_production_gate_blocks_spikes_in_pnpm_workspace(tmp_path, monkeypatch):
    """SPIKE-005B PoC 依赖不能被加入 pnpm 生产 workspace。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    workspace = tmp_path / "pnpm-workspace.yaml"
    workspace.write_text(
        "\n".join([
            "packages:",
            '  - "packages/*"',
            '  - "apps/*"',
            '  - "spikes/*"',
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.workspace_errors == ["pnpm-workspace.yaml:spikes/*"]
    assert "spikes/*" in " ".join(result.errors)

    workspace.write_text(
        "\n".join([
            "packages:",
            '  - "packages/*"',
            '  - "apps/*"',
            '  - "spikes/spike-005b-webview-capacitor-pda"',
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.workspace_errors == ["pnpm-workspace.yaml:spikes/spike-005b-webview-capacitor-pda"]


def test_pda_production_gate_blocks_exact_spikes_workspace_and_lockfile_paths(
    tmp_path,
    monkeypatch,
):
    """pnpm workspace / lockfile 中的精确 spikes 路径也不能纳入生产 workspace。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    workspace = tmp_path / "pnpm-workspace.yaml"
    workspace.write_text(
        "\n".join([
            "packages:",
            '  - "./spikes/"',
        ]),
        encoding="utf-8",
    )
    lockfile = tmp_path / "pnpm-lock.yaml"
    lockfile.write_text(
        "\n".join([
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  spikes:",
            "    dependencies:",
            "      react:",
            "        specifier: ^18.3.1",
            "        version: 18.3.1",
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.workspace_errors == ["pnpm-workspace.yaml:./spikes/"]
    assert result.lockfile_spike_importers == ["pnpm-lock.yaml:spikes"]
    assert "spikes" in " ".join(result.errors)


def test_pda_production_gate_blocks_explicit_pda_app_in_pnpm_workspace_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，pnpm workspace 不能显式加入 apps/pda-mobile。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    workspace = tmp_path / "pnpm-workspace.yaml"
    workspace.write_text(
        "\n".join([
            "packages:",
            '  - "packages/*"',
            '  - "apps/pda-mobile"',
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.workspace_pda_entries == ["pnpm-workspace.yaml:apps/pda-mobile"]
    assert "apps/pda-mobile" in " ".join(result.errors)


def test_pda_production_gate_blocks_normalized_pda_app_workspace_entries_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """pnpm workspace 中 ./apps/pda-mobile 或尾斜杠也视为显式加入 PDA app。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    workspace = tmp_path / "pnpm-workspace.yaml"
    workspace.write_text(
        "\n".join([
            "packages:",
            '  - "./apps/pda-mobile"',
            '  - "apps/pda-mobile/"',
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.workspace_pda_entries == [
        "pnpm-workspace.yaml:./apps/pda-mobile",
        "pnpm-workspace.yaml:apps/pda-mobile/",
    ]
    assert "apps/pda-mobile" in " ".join(result.errors)


def test_pda_production_gate_blocks_workspace_pda_app_entries_with_inline_comments_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """pnpm workspace 中带 YAML 行尾注释的 apps/pda-mobile 也必须阻断。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    workspace = tmp_path / "pnpm-workspace.yaml"
    workspace.write_text(
        "\n".join([
            "packages:",
            '  - "apps/pda-mobile" # PDA production app',
        ]),
        encoding="utf-8",
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.workspace_pda_entries == ["pnpm-workspace.yaml:apps/pda-mobile"]
    assert "apps/pda-mobile" in " ".join(result.errors)
