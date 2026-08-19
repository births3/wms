"""ADR-0027 Accepted 前 PDA 生产文件与依赖阻断测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pda_production_gate_test_helpers import write_package_json, write_proposed_adr


def test_pda_production_gate_blocks_app_files_before_adr0027_accepted(tmp_path, monkeypatch):
    """ADR-0027 未 Accepted 时，apps/pda-mobile 只能保留 .gitkeep。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)

    pda = tmp_path / "apps/pda-mobile"
    pda.mkdir(parents=True)
    (pda / ".gitkeep").write_text("", encoding="utf-8")

    result = check.collect_result()

    assert result.ok is True
    assert result.pda_files == []

    (pda / "package.json").write_text("{}", encoding="utf-8")
    result = check.collect_result()

    assert result.ok is False
    assert "apps/pda-mobile/package.json" in result.pda_files
    assert "ADR-0027" in " ".join(result.errors)


def test_pda_production_gate_blocks_production_pda_dependencies_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，生产 workspace 不能提前引入 RN / Expo / EAS / Capacitor 依赖。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {"dependencies": {"@capacitor/core": "latest"}},
    )
    write_package_json(
        tmp_path,
        "spikes/spike-005b-webview-capacitor-pda/package.json",
        {"dependencies": {"@capacitor/android": "latest"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.blocked_dependency_files == ["package.json"]
    assert "@capacitor/core" in " ".join(result.errors)
    assert "spikes/spike-005b-webview-capacitor-pda/package.json" not in result.blocked_dependency_files

    write_package_json(tmp_path, "package.json", {"dependencies": {}})
    write_package_json(
        tmp_path,
        "apps/web-admin/package.json",
        {"dependencies": {"react-native": "latest"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.blocked_dependency_files == ["apps/web-admin/package.json"]
    assert "react-native" in " ".join(result.errors)


def test_pda_production_gate_blocks_pda_dev_dependencies_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，生产 workspace 的 devDependencies 也不能提前引入 PDA 工具链。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {"devDependencies": {"@capacitor/cli": "latest"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert result.blocked_dependency_files == ["package.json"]
    assert "package.json:devDependencies:@capacitor/cli" in result.blocked_dependencies
    assert "@capacitor/cli" in " ".join(result.errors)


def test_pda_production_gate_blocks_expo_dependencies_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，生产 workspace 不能提前引入 Expo/RN 打包链路。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "apps/web-admin/package.json",
        {"devDependencies": {"expo": "latest", "expo-router": "latest"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert "apps/web-admin/package.json:devDependencies:expo" in result.blocked_dependencies
    assert "apps/web-admin/package.json:devDependencies:expo-router" in result.blocked_dependencies


def test_pda_production_gate_blocks_eas_cli_dependency_before_adr0027_accepted(
    tmp_path,
    monkeypatch,
):
    """ADR-0027 未 Accepted 时，生产 workspace 不能提前引入 EAS Android 打包链路。"""
    import check_pda_production_gate as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_proposed_adr(tmp_path)
    write_package_json(
        tmp_path,
        "package.json",
        {"devDependencies": {"eas-cli": "latest"}},
    )

    result = check.collect_result()

    assert result.ok is False
    assert "package.json:devDependencies:eas-cli" in result.blocked_dependencies
    assert "RN / Expo / EAS / Capacitor" in " ".join(result.errors)
    assert "eas-cli" in " ".join(result.errors)
