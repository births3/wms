"""PDA production gate 对 pnpm lockfile 条目的阻断测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import write_proposed_adr


def _collect_lockfile_result(tmp_path, monkeypatch, lines: list[str]):
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    lockfile = tmp_path / "pnpm-lock.yaml"
    lockfile.write_text("\n".join(lines), encoding="utf-8")

    return check.collect_result()


def test_pda_production_gate_blocks_pda_entries_in_pnpm_lockfile(tmp_path, monkeypatch):
    """pnpm lockfile 也不能残留生产 PDA 依赖或 spikes importer。"""
    result = _collect_lockfile_result(
        tmp_path,
        monkeypatch,
        [
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  apps/web-admin:",
            "    dependencies:",
            "      '@capacitor/core':",
            "        specifier: latest",
            "        version: 7.0.0",
            "",
            "  spikes/spike-005b-webview-capacitor-pda:",
            "    dependencies:",
            "      react:",
            "        specifier: ^18.3.1",
            "        version: 18.3.1",
            "",
            "packages:",
            "",
            "  '@capacitor/core@7.0.0':",
            "    resolution: {integrity: sha512-test}",
        ],
    )

    assert result.ok is False
    assert "pnpm-lock.yaml:apps/web-admin:dependencies:@capacitor/core" in result.lockfile_dependency_entries
    assert result.lockfile_spike_importers == ["pnpm-lock.yaml:spikes/spike-005b-webview-capacitor-pda"]
    assert "@capacitor/core" in " ".join(result.errors)
    assert "spikes/spike-005b-webview-capacitor-pda" in " ".join(result.errors)

    result = _collect_lockfile_result(
        tmp_path,
        monkeypatch,
        [
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  apps/web-admin:",
            "    dependencies:",
            "      react:",
            "        specifier: ^18.3.1",
            "        version: 18.3.1",
        ],
    )

    assert result.ok is True
    assert result.lockfile_dependency_entries == []
    assert result.lockfile_spike_importers == []


def test_pda_production_gate_blocks_normalized_pda_app_importer_in_pnpm_lockfile_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """pnpm lockfile 中 ./apps/pda-mobile 或尾斜杠 importer 也不能提前出现。"""
    result = _collect_lockfile_result(
        tmp_path,
        monkeypatch,
        [
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  ./apps/pda-mobile:",
            "    dependencies:",
            "      react:",
            "        specifier: ^18.3.1",
            "        version: 18.3.1",
            "",
            "  apps/pda-mobile/:",
            "    dependencies:",
            "      react:",
            "        specifier: ^18.3.1",
            "        version: 18.3.1",
        ],
    )

    assert result.ok is False
    assert result.lockfile_pda_importers == [
        "pnpm-lock.yaml:./apps/pda-mobile",
        "pnpm-lock.yaml:apps/pda-mobile/",
    ]
    assert "apps/pda-mobile" in " ".join(result.errors)


def test_pda_production_gate_blocks_pda_app_lockfile_importer_with_inline_comment_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """pnpm lockfile 中 importer 行带 YAML 行尾注释时也不能提前出现 PDA app。"""
    result = _collect_lockfile_result(
        tmp_path,
        monkeypatch,
        [
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  apps/pda-mobile: # generated before ADR accepted",
            "    dependencies:",
            "      react:",
            "        specifier: ^18.3.1",
            "        version: 18.3.1",
        ],
    )

    assert result.ok is False
    assert result.lockfile_pda_importers == ["pnpm-lock.yaml:apps/pda-mobile"]
    assert "apps/pda-mobile" in " ".join(result.errors)


def test_pda_production_gate_blocks_pda_app_importer_in_pnpm_lockfile_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，lockfile 不能提前出现 apps/pda-mobile importer。"""
    result = _collect_lockfile_result(
        tmp_path,
        monkeypatch,
        [
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  apps/pda-mobile:",
            "    dependencies:",
            "      react:",
            "        specifier: ^18.3.1",
            "        version: 18.3.1",
        ],
    )

    assert result.ok is False
    assert result.lockfile_pda_importers == ["pnpm-lock.yaml:apps/pda-mobile"]
    assert "apps/pda-mobile" in " ".join(result.errors)
