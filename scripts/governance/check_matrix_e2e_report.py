#!/usr/bin/env python3
"""check_matrix_e2e_report.py — 校验 Matrix E2E 截图报告。

类别：6. 原型治理
Tier：T4
输入：prototypes/.e2e-artifacts/matrix-e2e-report.json
输出：人类可读 + --json
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
DEFAULT_REPORT = REPO_ROOT / "prototypes" / ".e2e-artifacts" / "matrix-e2e-report.json"


def validate_report_payload(payload: dict[str, Any], *, require_full: bool) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []

    if payload.get("schema_version") != 1:
        errors.append("schema_version 必须为 1")
    if require_full and payload.get("mode") != "full":
        errors.append(f"报告 mode={payload.get('mode')}，但当前要求 full")
    if require_full and int(payload.get("expected_count", 0)) < 200:
        errors.append(f"full matrix expected_count 必须 >= 200，got {payload.get('expected_count')}")

    for key in ("expected_count", "actual_count", "passed_count", "failed_count", "missing_count", "screenshot_missing_count"):
        if not isinstance(payload.get(key), int):
            errors.append(f"{key} 必须是整数")

    if payload.get("playwright_exit") != 0:
        errors.append(f"playwright_exit={payload.get('playwright_exit')}")
    if payload.get("missing_count", 0) != 0:
        errors.append(f"missing_count={payload.get('missing_count')}: {payload.get('missing', [])[:10]}")
    if payload.get("failed_count", 0) != 0:
        errors.append(f"failed_count={payload.get('failed_count')}: {payload.get('failed', [])[:10]}")
    if payload.get("screenshot_missing_count", 0) != 0:
        errors.append(
            f"screenshot_missing_count={payload.get('screenshot_missing_count')}: "
            f"{payload.get('screenshot_missing', [])[:10]}"
        )
    if payload.get("actual_count") != payload.get("expected_count"):
        errors.append(f"actual_count {payload.get('actual_count')} != expected_count {payload.get('expected_count')}")
    if payload.get("passed_count") != payload.get("expected_count"):
        errors.append(f"passed_count {payload.get('passed_count')} != expected_count {payload.get('expected_count')}")

    for result in payload.get("results", []):
        tab = result.get("tab", "<unknown>")
        if result.get("status") != "passed":
            errors.append(f"{tab}: status={result.get('status')} issues={result.get('issues', [])[:5]}")
        screenshots = result.get("screenshots", [])
        if len(screenshots) < 2:
            errors.append(f"{tab}: 必须至少有 initial + after-interaction 两张截图")
        for screenshot in screenshots:
            path = Path(screenshot)
            if not path.exists():
                errors.append(f"{tab}: screenshot 不存在 {screenshot}")
            elif path.stat().st_size < 1024:
                errors.append(f"{tab}: screenshot 异常小 {screenshot}")
        if not result.get("keywordHits"):
            warnings.append(f"{tab}: keywordHits 为空")

    return errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--report", default=str(DEFAULT_REPORT))
    parser.add_argument("--allow-partial", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report_path = Path(args.report)
    errors: list[str] = []
    warnings: list[str] = []
    if not report_path.exists():
        errors.append(f"缺少报告：{report_path}")
        payload: dict[str, Any] = {}
    else:
        try:
            payload = json.loads(report_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            payload = {}
            errors.append(f"报告 JSON 无效：{exc}")

    if payload:
        more_errors, warnings = validate_report_payload(payload, require_full=not args.allow_partial)
        errors.extend(more_errors)

    if args.json:
        print(json.dumps({
            "ok": not errors,
            "report": str(report_path),
            "errors": errors,
            "warnings": warnings,
        }, ensure_ascii=False))
    else:
        print("check_matrix_e2e_report")
        if payload:
            print(
                f"  · mode={payload.get('mode')} "
                f"passed={payload.get('passed_count')}/{payload.get('expected_count')} "
                f"duration={payload.get('duration_seconds')}s"
            )
        for warning in warnings:
            print(f"  ⚠ {warning}")
        if errors:
            print(f"  ✘ {len(errors)} violation(s):")
            for error in errors:
                print(f"    - {error}")
        else:
            print("  ✓ Matrix E2E report 通过")

    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
