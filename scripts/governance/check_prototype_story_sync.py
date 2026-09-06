#!/usr/bin/env python3
"""check_prototype_story_sync.py — ADR-0043 生产故事同步兼容入口。

原型先行已由 ADR-0043 取代。现在故事与前端实现的同步权威数据位于
`governance/quality-matrix.toml`，本检查验证矩阵 story/deferred story 均能追溯到
`docs/domain/user-stories-*.md`，并拒绝空矩阵。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from _direct_production_frontend import replacement_contract_errors

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MATRIX = REPO_ROOT / "governance" / "quality-matrix.toml"
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"
STORY_RE = re.compile(r"^## (?:~~)?(US-[A-Z0-9]+-\d{3}[a-z]?)(?:~~)?")


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _story_to_file() -> dict[str, str]:
    mapping: dict[str, str] = {}
    for path in DOMAIN_DIR.glob("user-stories-*.md"):
        for line in path.read_text(encoding="utf-8").splitlines():
            match = STORY_RE.match(line)
            if match:
                mapping[match.group(1)] = path.relative_to(REPO_ROOT).as_posix()
    return mapping


def run() -> tuple[list[str], list[str]]:
    errors = replacement_contract_errors()
    warnings: list[str] = []
    if errors:
        return errors, warnings

    payload = _load_toml(MATRIX)
    rows = [
        row
        for section in ("stories", "deferred_stories")
        for row in payload.get(section, [])
        if isinstance(row, dict)
    ]
    if not rows:
        return ["governance/quality-matrix.toml 没有 stories/deferred_stories"], warnings

    story_files = _story_to_file()
    for row in rows:
        story_id = str(row.get("id", "")).strip()
        if story_id and story_id not in story_files:
            errors.append(f"{story_id}: 质量矩阵记录无法追溯到领域故事文件")
    return errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    try:
        errors, warnings = run()
    except Exception as exc:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(exc)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {exc}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors, "warnings": warnings, "ok": not errors}, ensure_ascii=False))
    elif errors:
        print(f"✗ check_prototype_story_sync: {len(errors)} 项违规")
        for error in errors:
            print(f"  - {error}")
    else:
        print("✓ check_prototype_story_sync: ADR-0043 生产质量矩阵与领域故事同步")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
