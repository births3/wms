#!/usr/bin/env python3
"""run_matrix_e2e_screenshots.py — 运行全量矩阵 E2E 截图门禁。

类别：6. 原型治理
Tier：T4

默认覆盖 governance/visual-baselines/manifest.toml 中的全部 tab。
本脚本会生成 prototypes/.e2e-artifacts/matrix-input.json，调用 Playwright，
并聚合每个 tab 的 initial / after-interaction 截图与 DOM 健康检查结果。
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

_THIS = Path(__file__).resolve()
SCRIPTS_DIR = _THIS.parent
REPO_ROOT = SCRIPTS_DIR.parent.parent
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
SCENARIOS_TOML = REPO_ROOT / "governance" / "visual-baselines" / "e2e-scenarios.toml"
DEFAULT_ARTIFACTS_DIR = REPO_ROOT / "prototypes" / ".e2e-artifacts"

sys.path.insert(0, str(SCRIPTS_DIR))
from check_e2e_matrix_completeness import infer_device, load_toml, resolve_policy  # noqa: E402


def parse_viewport(value: str) -> dict[str, int]:
    match = re.match(r"^(\d+)x(\d+)(?:~(\d+))?$", value)
    if not match:
        raise ValueError(f"viewport 格式错误：{value}")
    return {"width": int(match.group(1)), "height": int(match.group(2))}


def find_chrome() -> str | None:
    for name in ("google-chrome", "chromium-browser", "chromium", "chrome"):
        found = shutil.which(name)
        if found:
            return found
    return None


def build_scenarios(
    manifest: dict[str, Any],
    scenarios_config: dict[str, Any],
    *,
    only_tabs: set[str] | None,
    limit: int | None,
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for snap in manifest.get("snapshots", []):
        tab = snap["tab"]
        if only_tabs and tab not in only_tabs:
            continue
        policy = resolve_policy(scenarios_config, tab)
        out.append({
            "tab": tab,
            "urlHash": snap["url_hash"],
            "viewport": parse_viewport(snap["viewport"]),
            "file": snap["file"],
            "device": infer_device(tab),
            "relatedStory": snap.get("related_story", ""),
            "expectedKeywords": snap.get("expected_keywords", []),
            "minKeywordHitRatio": float(policy["min_keyword_hit_ratio"]),
            "requiredSelectors": policy["required_selectors"],
            "maxHorizontalOverflowPx": int(policy["max_horizontal_overflow_px"]),
            "forbidConsoleErrors": bool(policy["forbid_console_errors"]),
            "detectTextOverflow": bool(policy["detect_text_overflow"]),
            "detectControlOverlap": bool(policy["detect_control_overlap"]),
            "detectVerticalCjkTable": bool(policy["detect_vertical_cjk_table"]),
            "clickStrategy": policy["click_strategy"],
        })
        if limit is not None and len(out) >= limit:
            break
    return out


def load_result_files(results_dir: Path) -> dict[str, Any]:
    results: dict[str, Any] = {}
    if not results_dir.exists():
        return results
    for path in results_dir.glob("*.json"):
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        tab = payload.get("tab")
        if tab:
            results[tab] = payload
    return results


def write_report(
    *,
    artifacts_dir: Path,
    scenarios: list[dict[str, Any]],
    playwright_exit: int,
    duration_seconds: float,
) -> dict[str, Any]:
    results = load_result_files(artifacts_dir / "results")
    expected_tabs = [scenario["tab"] for scenario in scenarios]
    missing = [tab for tab in expected_tabs if tab not in results]
    failed = [
        tab for tab in expected_tabs
        if tab in results and results[tab].get("status") != "passed"
    ]
    screenshot_missing: list[str] = []
    for tab in expected_tabs:
        result = results.get(tab)
        if not result:
            continue
        for screenshot in result.get("screenshots", []):
            path = Path(screenshot)
            if not path.exists() or path.stat().st_size < 1024:
                screenshot_missing.append(f"{tab}: {screenshot}")

    report = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "mode": "full" if len(expected_tabs) >= 200 else "partial",
        "expected_count": len(expected_tabs),
        "actual_count": len(results),
        "passed_count": len([r for r in results.values() if r.get("status") == "passed"]),
        "failed_count": len(failed),
        "missing_count": len(missing),
        "screenshot_missing_count": len(screenshot_missing),
        "playwright_exit": playwright_exit,
        "duration_seconds": round(duration_seconds, 2),
        "ok": playwright_exit == 0 and not missing and not failed and not screenshot_missing,
        "missing": missing,
        "failed": failed,
        "screenshot_missing": screenshot_missing,
        "results": [results[tab] for tab in expected_tabs if tab in results],
    }
    (artifacts_dir / "matrix-e2e-report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tab", action="append", help="只运行指定 tab，可重复")
    parser.add_argument("--limit", type=int, help="只运行前 N 个场景（调试用，会生成 partial 报告）")
    parser.add_argument("--base-url", default="http://127.0.0.1:5173")
    parser.add_argument("--artifacts-dir", default=str(DEFAULT_ARTIFACTS_DIR))
    parser.add_argument("--headed", action="store_true")
    parser.add_argument("--keep-artifacts", action="store_true")
    args = parser.parse_args()

    if not MANIFEST_TOML.exists() or not SCENARIOS_TOML.exists():
        print("缺少 manifest.toml 或 e2e-scenarios.toml；先跑 check_e2e_matrix_completeness.py", file=sys.stderr)
        return 2

    artifacts_dir = Path(args.artifacts_dir).resolve()
    if not args.keep_artifacts and artifacts_dir.exists():
        shutil.rmtree(artifacts_dir)
    (artifacts_dir / "results").mkdir(parents=True, exist_ok=True)
    (artifacts_dir / "screenshots").mkdir(parents=True, exist_ok=True)

    manifest = load_toml(MANIFEST_TOML)
    scenarios_config = load_toml(SCENARIOS_TOML)
    scenarios = build_scenarios(
        manifest,
        scenarios_config,
        only_tabs=set(args.tab) if args.tab else None,
        limit=args.limit,
    )
    if not scenarios:
        print("没有匹配的 E2E scenario", file=sys.stderr)
        return 2

    input_payload = {
        "baseUrl": args.base_url.rstrip("/"),
        "artifactsDir": str(artifacts_dir),
        "scenarios": scenarios,
    }
    input_path = artifacts_dir / "matrix-input.json"
    input_path.write_text(json.dumps(input_payload, ensure_ascii=False, indent=2), encoding="utf-8")

    chrome = find_chrome()
    env = os.environ.copy()
    env["MATRIX_E2E_INPUT"] = str(input_path)
    env["MATRIX_E2E_ARTIFACTS"] = str(artifacts_dir)
    env["MATRIX_E2E_BASE_URL"] = args.base_url.rstrip("/")
    if chrome:
        env["PLAYWRIGHT_CHROMIUM_EXECUTABLE"] = chrome

    command = [
        "pnpm",
        "--dir",
        str(REPO_ROOT / "prototypes"),
        "exec",
        "playwright",
        "test",
        "--config=playwright-matrix-config.ts",
    ]
    if args.headed:
        command.append("--headed")

    import time
    start = time.perf_counter()
    print(f"▶ Matrix E2E screenshots: {len(scenarios)} tab")
    if chrome:
        print(f"  · chromium executable: {chrome}")
    else:
        print("  · chromium executable: Playwright default browser")
    proc = subprocess.run(command, cwd=REPO_ROOT / "prototypes", env=env, check=False)
    duration = time.perf_counter() - start

    report = write_report(
        artifacts_dir=artifacts_dir,
        scenarios=scenarios,
        playwright_exit=proc.returncode,
        duration_seconds=duration,
    )
    print(
        "▶ Matrix E2E summary: "
        f"{report['passed_count']}/{report['expected_count']} passed, "
        f"failed={report['failed_count']}, missing={report['missing_count']}, "
        f"screenshot_missing={report['screenshot_missing_count']}"
    )
    print(f"  · report: {(artifacts_dir / 'matrix-e2e-report.json').relative_to(REPO_ROOT)}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
