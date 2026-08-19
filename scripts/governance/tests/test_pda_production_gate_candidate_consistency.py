"""PDA production gate 在 ADR-0027 Accepted 后的 runtime candidate 一致性测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import (
    write_accepted_adr,
    write_package_json,
    write_pnpm_lockfile,
    write_rn_spike,
    write_wave3_pda_evidence,
)


def test_pda_production_gate_blocks_dependencies_that_do_not_match_accepted_runtime_candidate(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 Accepted 后，生产依赖必须匹配 runtime evidence 选出的 PDA 技术栈候选。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_accepted_adr(tmp_path)
    write_wave3_pda_evidence(tmp_path)
    write_rn_spike(tmp_path)
    write_package_json(
        tmp_path,
        "apps/pda-mobile/package.json",
        {"dependencies": {"@capacitor/core": "latest"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert "pda_stack_candidate=react-native" in " ".join(result.errors)
    assert "@capacitor/core" in " ".join(result.errors)


def test_pda_production_gate_blocks_scripts_and_lockfile_dependencies_that_do_not_match_candidate(
    tmp_path,
    monkeypatch,
):
    """生产脚本与 lockfile 依赖也必须匹配 ADR-0027 Accepted 后的 runtime candidate。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_accepted_adr(tmp_path)
    write_wave3_pda_evidence(tmp_path)
    write_rn_spike(tmp_path)
    write_package_json(
        tmp_path,
        "apps/pda-mobile/package.json",
        {"scripts": {"android": "cap sync android"}},
    )
    write_pnpm_lockfile(
        tmp_path,
        [
            "lockfileVersion: '9.0'",
            "",
            "importers:",
            "",
            "  apps/pda-mobile:",
            "    dependencies:",
            "      '@capacitor/android':",
            "        specifier: latest",
            "        version: 7.0.0",
            "",
            "packages:",
            "",
            "  '@capacitor/android@7.0.0':",
            "    resolution: {integrity: sha512-test}",
        ],
    )

    result = check.collect_result()

    assert result.ok is False
    assert "apps/pda-mobile/package.json:scripts:android:cap sync android" in (
        result.incompatible_script_entries
    )
    assert "pnpm-lock.yaml:apps/pda-mobile:dependencies:@capacitor/android" in (
        result.incompatible_lockfile_dependency_entries
    )
    assert "pnpm-lock.yaml:packages:@capacitor/android@7.0.0" in (
        result.incompatible_lockfile_package_entries
    )
