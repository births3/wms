#!/usr/bin/env python3
"""check_baseline_completeness.py — ADR-0043 真实页面证据完整性兼容入口。

默认模式保持 T1 的纯静态契约检查：生产页面必须登记真实 Playwright E2E、截图路径和
quality-matrix evidence_refs。使用 ``--require-files`` 时进入运行时证据模式，除静态契约外还
会逐个验证 quality matrix 中声明的真实截图文件确实存在、不是空文件，并且是可解析出正数
尺寸的 PNG。CI 的 PR deep-validation 在真实 E2E 完成后使用该模式，避免“只登记路径就变绿”。
"""
from __future__ import annotations

import argparse
import json
import struct
import subprocess
import sys
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    import tomli as tomllib

from _direct_production_frontend import replacement_contract_errors

_THIS = Path(__file__).resolve()
SCRIPTS_DIR = _THIS.parent
REPO_ROOT = _THIS.parent.parent.parent
REPLACEMENT = SCRIPTS_DIR / "check_scope_gap_discovery.py"
QUALITY_MATRIX = REPO_ROOT / "governance" / "quality-matrix.toml"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
MIN_SCREENSHOT_BYTES = 1024


def _static_contract_errors() -> list[str]:
    errors = replacement_contract_errors()
    if errors:
        return errors
    if not REPLACEMENT.is_file():
        return ["真实页面证据检查器 check_scope_gap_discovery.py 不存在"]

    result = subprocess.run(
        [sys.executable, str(REPLACEMENT), "--json"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0:
        return []
    detail = result.stdout.strip() or result.stderr.strip() or f"exit={result.returncode}"
    return [f"生产页面真实 E2E/截图契约未闭环: {detail[:3000]}"]


def _matrix_records() -> list[tuple[str, str, str, str]]:
    if not QUALITY_MATRIX.is_file():
        return []
    data = tomllib.loads(QUALITY_MATRIX.read_text(encoding="utf-8"))
    records: list[tuple[str, str, str, str]] = []
    for section in ("stories", "deferred_stories"):
        values = data.get(section, [])
        if not isinstance(values, list):
            continue
        for story in values:
            if not isinstance(story, dict):
                continue
            story_id = str(story.get("id", "-"))
            screenshots = story.get("e2e_screenshots", [])
            if not isinstance(screenshots, list):
                continue
            for record in screenshots:
                if not isinstance(record, dict):
                    continue
                page = record.get("page")
                spec = record.get("spec")
                screenshot = record.get("screenshot")
                if all(isinstance(value, str) and value for value in (page, spec, screenshot)):
                    records.append((story_id, page, spec, screenshot))
    return records


def _png_dimensions(path: Path) -> tuple[int, int] | None:
    try:
        with path.open("rb") as handle:
            header = handle.read(24)
    except OSError:
        return None
    if len(header) < 24 or header[:8] != PNG_SIGNATURE or header[12:16] != b"IHDR":
        return None
    width, height = struct.unpack(">II", header[16:24])
    if width <= 0 or height <= 0:
        return None
    return width, height


def _runtime_file_errors() -> list[str]:
    if not QUALITY_MATRIX.is_file():
        return ["缺少 governance/quality-matrix.toml，无法验证真实截图文件"]

    errors: list[str] = []
    records = _matrix_records()
    if not records:
        return ["quality matrix 未声明任何 e2e_screenshots，运行时截图门禁没有可验证对象"]

    seen: set[str] = set()
    for story_id, page, spec, screenshot in records:
        if screenshot in seen:
            continue
        seen.add(screenshot)

        spec_path = REPO_ROOT / spec
        screenshot_path = REPO_ROOT / screenshot
        label = f"{story_id}/{page}"

        if not screenshot.startswith("artifacts/screenshot-portal/real-web/"):
            errors.append(f"{label}: 截图证据不在 real-web 目录：{screenshot}")
            continue
        if not spec.startswith("prototypes/e2e/") or not spec.endswith("-real.spec.ts"):
            errors.append(f"{label}: 截图证据未绑定真实 Playwright spec：{spec}")
            continue
        if not spec_path.is_file():
            errors.append(f"{label}: Playwright spec 不存在：{spec}")
            continue
        if Path(screenshot).name not in spec_path.read_text(encoding="utf-8"):
            errors.append(f"{label}: spec 未引用声明的截图文件名：{screenshot}")
            continue
        if not screenshot_path.is_file():
            errors.append(f"{label}: 真实截图文件不存在：{screenshot}")
            continue

        size = screenshot_path.stat().st_size
        if size < MIN_SCREENSHOT_BYTES:
            errors.append(
                f"{label}: 截图文件异常小：{screenshot} ({size} bytes < {MIN_SCREENSHOT_BYTES})"
            )
            continue
        dimensions = _png_dimensions(screenshot_path)
        if dimensions is None:
            errors.append(f"{label}: 截图不是有效 PNG/IHDR：{screenshot}")
            continue

    return errors


def run(*, require_files: bool = False) -> list[str]:
    errors = _static_contract_errors()
    if require_files:
        errors.extend(_runtime_file_errors())
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--base", default=None, help="保留历史 CLI 兼容；ADR-0043 模式由质量矩阵决定证据范围")
    parser.add_argument(
        "--require-files",
        action="store_true",
        help="真实 E2E 完成后验证 quality matrix 声明的 PNG 文件确实存在且有效",
    )
    args = parser.parse_args()

    try:
        errors = run(require_files=args.require_files)
    except Exception as exc:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(exc)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {exc}", file=sys.stderr)
        return 2

    payload: dict[str, Any] = {
        "status": "fail" if errors else "pass",
        "errors": errors,
        "ok": not errors,
        "require_files": args.require_files,
    }
    if args.require_files:
        payload["declared_screenshot_count"] = len({record[3] for record in _matrix_records()})

    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    elif errors:
        mode = "运行时真实截图" if args.require_files else "静态截图契约"
        print(f"✗ check_baseline_completeness: {mode}发现 {len(errors)} 项缺口")
        for error in errors:
            print(f"  - {error}")
    elif args.require_files:
        count = len({record[3] for record in _matrix_records()})
        print(f"✓ check_baseline_completeness: {count} 个真实页面 PNG 文件存在且有效")
    else:
        print("✓ check_baseline_completeness: ADR-0043 真实页面 E2E/截图静态契约完整")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
