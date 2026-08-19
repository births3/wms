#!/usr/bin/env python3
"""check_e2e_matrix_completeness.py — Matrix E2E 策略完整性检查。

类别：6. 原型治理
Tier：T1
输入：
  - governance/visual-baselines/manifest.toml
  - governance/visual-baselines/e2e-scenarios.toml
输出：人类可读 + --json

本脚本只做静态覆盖检查，不启动浏览器。真正的 Playwright 全量矩阵截图
由 run_matrix_e2e_screenshots.py 执行，报告由 check_matrix_e2e_report.py 校验。
"""
from __future__ import annotations

import argparse
import fnmatch
import json
import sys
from pathlib import Path
from typing import Any

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
SCENARIOS_TOML = REPO_ROOT / "governance" / "visual-baselines" / "e2e-scenarios.toml"

REQUIRED_DEFAULT_KEYS = {
    "required_selectors",
    "min_keyword_hit_ratio",
    "max_horizontal_overflow_px",
    "forbid_console_errors",
    "detect_text_overflow",
    "detect_control_overlap",
    "detect_vertical_cjk_table",
    "click_strategy",
    "capture_states",
}


def load_toml(path: Path) -> dict[str, Any]:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def infer_device(tab: str) -> str:
    for device in ("pc", "pda", "pad", "h5"):
        if tab.startswith(f"{device}-"):
            return device
    if "pda" in tab:
        return "pda"
    if "h5" in tab:
        return "h5"
    return "pc"


def override_matches(override: dict[str, Any], tab: str) -> bool:
    return any(fnmatch.fnmatchcase(tab, pattern) for pattern in override.get("match_globs", []))


def resolve_policy(scenarios: dict[str, Any], tab: str) -> dict[str, Any]:
    policy = dict(scenarios.get("defaults", {}))
    device = infer_device(tab)
    device_policy = scenarios.get("devices", {}).get(device, {})
    policy.update(device_policy)
    for override in scenarios.get("overrides", []):
        if override_matches(override, tab):
            policy.update({k: v for k, v in override.items() if k not in {"name", "match_globs"}})
    return policy


def validate_matrix_config(
    manifest: dict[str, Any],
    scenarios: dict[str, Any],
) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []

    snapshots = manifest.get("snapshots", [])
    if not snapshots:
        errors.append("manifest.toml 缺少 [[snapshots]]")
        return errors, warnings

    defaults = scenarios.get("defaults")
    if not isinstance(defaults, dict):
        errors.append("e2e-scenarios.toml 缺少 [defaults]")
        return errors, warnings

    missing_defaults = sorted(REQUIRED_DEFAULT_KEYS - set(defaults))
    for key in missing_defaults:
        errors.append(f"[defaults] 缺少 {key}")

    capture_states = defaults.get("capture_states", [])
    if "initial" not in capture_states or "after-interaction" not in capture_states:
        errors.append("[defaults].capture_states 必须包含 initial 与 after-interaction")

    seen_tabs: set[str] = set()
    for snap in snapshots:
        tab = snap.get("tab")
        if not tab:
            errors.append("manifest snapshot 缺 tab")
            continue
        if tab in seen_tabs:
            errors.append(f"manifest tab 重复：{tab}")
        seen_tabs.add(tab)

        for key in ("url_hash", "viewport", "file", "expected_keywords", "related_story"):
            if key not in snap:
                errors.append(f"{tab}: manifest 缺 {key}")

        policy = resolve_policy(scenarios, tab)
        missing_policy = sorted(REQUIRED_DEFAULT_KEYS - set(policy))
        for key in missing_policy:
            errors.append(f"{tab}: E2E policy 缺 {key}")

        selectors = policy.get("required_selectors", [])
        if "main" not in selectors:
            errors.append(f"{tab}: required_selectors 必须包含 main")

        ratio = policy.get("min_keyword_hit_ratio")
        if not isinstance(ratio, (int, float)) or ratio < 0.5:
            errors.append(f"{tab}: min_keyword_hit_ratio 必须 >= 0.5")

    overrides = scenarios.get("overrides", [])
    for override in overrides:
        name = override.get("name", "<unnamed>")
        matches = [snap["tab"] for snap in snapshots if override_matches(override, snap["tab"])]
        if not matches:
            errors.append(f"override {name}: match_globs 没有命中任何 manifest tab")

    manifest_tabs = {snap.get("tab") for snap in snapshots}
    if "m4-manifest" in manifest_tabs:
        m4_policy = resolve_policy(scenarios, "m4-manifest")
        if not m4_policy.get("detect_vertical_cjk_table"):
            errors.append("m4-manifest 必须启用 detect_vertical_cjk_table，防止随货同行单中文竖排回归")

    if len(snapshots) < 200:
        warnings.append(f"manifest 只有 {len(snapshots)} 个 tab；预期当前矩阵约 204 个")

    return errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    errors: list[str] = []
    warnings: list[str] = []
    if not MANIFEST_TOML.exists():
        errors.append(f"缺少 {MANIFEST_TOML.relative_to(REPO_ROOT)}")
    if not SCENARIOS_TOML.exists():
        errors.append(f"缺少 {SCENARIOS_TOML.relative_to(REPO_ROOT)}")
    if errors:
        if args.json:
            print(json.dumps({"ok": False, "errors": errors, "warnings": warnings}, ensure_ascii=False))
        else:
            print("check_e2e_matrix_completeness")
            for error in errors:
                print(f"  ✘ {error}")
        return 1

    manifest = load_toml(MANIFEST_TOML)
    scenarios = load_toml(SCENARIOS_TOML)
    errors, warnings = validate_matrix_config(manifest, scenarios)
    snapshots = manifest.get("snapshots", [])

    if args.json:
        print(json.dumps({
            "ok": not errors,
            "snapshots": len(snapshots),
            "errors": errors,
            "warnings": warnings,
        }, ensure_ascii=False))
    else:
        print(f"check_e2e_matrix_completeness — {len(snapshots)} tab")
        for warning in warnings:
            print(f"  ⚠ {warning}")
        if errors:
            print(f"  ✘ {len(errors)} violation(s):")
            for error in errors:
                print(f"    - {error}")
        else:
            print("  ✓ Matrix E2E 策略覆盖全部 manifest tab")

    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
