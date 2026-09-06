#!/usr/bin/env python3
"""check_prototype_index_consistency.py — 前端故事索引一致性兼容入口。

ADR-0043 已将新页面的权威索引从原型 index.toml 迁移到生产前端质量矩阵。
本检查保留原入口名以维持 Tier 编号稳定，但实际验证：
- ADR-0043 的替代契约完整；
- quality-matrix.toml 中 stories/deferred_stories 的 story id 唯一；
- 每个矩阵 story id 都能在领域 user-stories 文档中找到。
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


def _domain_story_ids() -> set[str]:
    ids: set[str] = set()
    for path in DOMAIN_DIR.glob("user-stories-*.md"):
        for line in path.read_text(encoding="utf-8").splitlines():
            match = STORY_RE.match(line)
            if match:
                ids.add(match.group(1))
    return ids


def run() -> list[str]:
    errors = replacement_contract_errors()
    if errors:
        return errors

    payload = _load_toml(MATRIX)
    rows = [
        row
        for section in ("stories", "deferred_stories")
        for row in payload.get(section, [])
        if isinstance(row, dict)
    ]
    ids = [str(row.get("id", "")).strip() for row in rows]
    domain_ids = _domain_story_ids()

    for index, story_id in enumerate(ids):
        if not story_id:
            errors.append(f"quality-matrix row {index} 缺少 id")
        elif story_id not in domain_ids:
            errors.append(f"quality-matrix story '{story_id}' 在 user-stories 中不存在")

    seen: set[str] = set()
    for story_id in ids:
        if story_id and story_id in seen:
            errors.append(f"quality-matrix story id 重复: {story_id}")
        seen.add(story_id)
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    try:
        errors = run()
    except Exception as exc:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(exc)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {exc}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors, "ok": not errors}, ensure_ascii=False))
    elif errors:
        print(f"✗ check_prototype_index_consistency: {len(errors)} 项违规")
        for error in errors:
            print(f"  - {error}")
    else:
        print("✓ check_prototype_index_consistency: ADR-0043 生产质量矩阵索引一致")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
