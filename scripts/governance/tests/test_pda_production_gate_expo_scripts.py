"""PDA production gate 对 Expo / EAS 生产 scripts 的阻断测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import write_package_json, write_proposed_adr


def test_pda_production_gate_blocks_expo_android_build_scripts_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，生产 scripts 不能提前引入 Expo/EAS Android 打包链路。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {
            "scripts": {
                "pda:eas": "eas build --platform android --profile staging",
                "pda:prebuild": "expo prebuild --platform android",
            },
        },
    )

    result = check.collect_result()

    assert result.ok is False
    assert "package.json:scripts:pda:eas:eas build --platform android --profile staging" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:prebuild:expo prebuild --platform android" in (
        result.blocked_script_entries
    )


def test_pda_production_gate_blocks_expo_prebuild_without_platform_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，Expo native prebuild 不带 platform 也不能进入生产 scripts。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {"scripts": {"pda:prebuild": "expo prebuild --no-install"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert "package.json:scripts:pda:prebuild:expo prebuild --no-install" in (
        result.blocked_script_entries
    )


def test_pda_production_gate_blocks_expo_android_build_scripts_with_equals_platform(
    tmp_path,
    monkeypatch,
):
    """Expo/EAS Android 打包脚本使用 --platform=android 或 -p=android 时也必须阻断。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {
            "scripts": {
                "pda:eas": "eas build --platform=android --profile staging",
                "pda:prebuild": "expo prebuild -p=android",
            },
        },
    )

    result = check.collect_result()

    assert result.ok is False
    assert "package.json:scripts:pda:eas:eas build --platform=android --profile staging" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:prebuild:expo prebuild -p=android" in (
        result.blocked_script_entries
    )


def test_pda_production_gate_blocks_eas_cli_android_build_scripts_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """EAS Android 打包脚本通过 eas-cli 命令入口时也必须阻断。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {
            "scripts": {
                "pda:eas-cli": "eas-cli build --platform android --profile staging",
                "pda:eas-dlx": "pnpm dlx eas-cli build -p android --profile staging",
            },
        },
    )

    result = check.collect_result()

    assert result.ok is False
    assert "package.json:scripts:pda:eas-cli:eas-cli build --platform android --profile staging" in (
        result.blocked_script_entries
    )
    assert "package.json:scripts:pda:eas-dlx:pnpm dlx eas-cli build -p android --profile staging" in (
        result.blocked_script_entries
    )
