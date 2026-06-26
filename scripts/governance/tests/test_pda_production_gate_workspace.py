"""PDA production gate 对生产 scripts 的阻断测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import write_package_json, write_proposed_adr


def test_pda_production_gate_blocks_production_pda_scripts_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，生产 scripts 不能提前启动 PDA app 或 native 打包链路。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {"scripts": {"pda:build": "pnpm --dir apps/pda-mobile build"}},
    )
    write_package_json(
        tmp_path,
        "spikes/spike-005b-webview-capacitor-pda/package.json",
        {"scripts": {"poc:android": "capacitor build android"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.blocked_script_files == ["package.json"]
    assert any(
        entry.startswith("package.json:scripts:pda:build:")
        for entry in result.blocked_script_entries
    )
    assert "apps/pda-mobile" in " ".join(result.errors)
    assert "spikes/spike-005b-webview-capacitor-pda/package.json" not in result.blocked_script_files

    write_package_json(tmp_path, "package.json", {"scripts": {}})
    write_package_json(
        tmp_path,
        "apps/web-admin/package.json",
        {"scripts": {"android": "capacitor build android"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.blocked_script_files == ["apps/web-admin/package.json"]
    assert "capacitor build android" in " ".join(result.errors)


def test_pda_production_gate_blocks_wrapped_native_build_scripts_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """native build 命令经 npx/pnpm exec 包装或省略 EAS platform 时也必须阻断。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {
            "scripts": {
                "pda:eas-default": "pnpm exec eas build --profile staging",
                "pda:cap-cli": "npx @capacitor/cli sync android",
                "pda:rn-cli": "pnpm exec @react-native-community/cli run-android",
            },
        },
    )

    result = check.collect_result()

    assert result.ok is False
    assert "package.json:scripts:pda:eas-default:pnpm exec eas build --profile staging" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:cap-cli:npx @capacitor/cli sync android" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:rn-cli:pnpm exec @react-native-community/cli run-android" in (
        result.blocked_script_entries
    )


def test_pda_production_gate_blocks_versioned_native_cli_scripts_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """native CLI 通过 npx/pnpm dlx 的 @version 入口执行时也必须阻断。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {
            "scripts": {
                "pda:eas-versioned": "pnpm dlx eas-cli@latest build --profile staging",
                "pda:cap-versioned": "npx @capacitor/cli@latest sync android",
                "pda:rn-versioned": "npx react-native@latest run-android",
                "pda:expo-versioned": "pnpm dlx @expo/cli@latest prebuild",
            },
        },
    )

    result = check.collect_result()

    assert result.ok is False
    assert "package.json:scripts:pda:eas-versioned:pnpm dlx eas-cli@latest build --profile staging" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:cap-versioned:npx @capacitor/cli@latest sync android" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:rn-versioned:npx react-native@latest run-android" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:expo-versioned:pnpm dlx @expo/cli@latest prebuild" in (
        result.blocked_script_entries
    )
