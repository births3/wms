"""PDA production gate 对 pnpm lockfile package / dependency 条目的阻断测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import write_pnpm_lockfile, write_proposed_adr


def test_pda_production_gate_blocks_orphan_pda_package_entries_in_pnpm_lockfile(
    tmp_path,
    monkeypatch,
):
    """pnpm lockfile 的 packages 区不能残留孤立 PDA 技术栈包。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_pnpm_lockfile(
        tmp_path,
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
            "",
            "packages:",
            "",
            "  '@capacitor/core@7.0.0':",
            "    resolution: {integrity: sha512-test}",
        ],
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.lockfile_package_entries == ["pnpm-lock.yaml:packages:@capacitor/core@7.0.0"]
    assert "@capacitor/core" in " ".join(result.errors)


def test_pda_production_gate_blocks_scoped_pda_package_entries_with_peer_suffix_in_lockfile(
    tmp_path,
    monkeypatch,
):
    """pnpm lockfile scoped 包带 peer suffix 时仍必须识别 PDA 技术栈包名。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_pnpm_lockfile(
        tmp_path,
        [
            "lockfileVersion: '9.0'",
            "",
            "packages:",
            "",
            "  '@capacitor/core@7.0.0(@types/node@20.0.0)':",
            "    resolution: {integrity: sha512-test}",
        ],
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.lockfile_package_entries == [
        "pnpm-lock.yaml:packages:@capacitor/core@7.0.0(@types/node@20.0.0)",
    ]


def test_pda_production_gate_blocks_pda_dev_dependency_entries_in_pnpm_lockfile(
    tmp_path,
    monkeypatch,
):
    """pnpm lockfile 的生产 importer devDependencies 也不能残留 PDA 工具链。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_pnpm_lockfile(
        tmp_path,
        [
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  .:",
            "    devDependencies:",
            "      '@capacitor/cli':",
            "        specifier: latest",
            "        version: 7.0.0",
        ],
    )

    result = check.collect_result()

    assert result.ok is False
    assert "pnpm-lock.yaml:.:devDependencies:@capacitor/cli" in result.lockfile_dependency_entries
    assert "@capacitor/cli" in " ".join(result.errors)
