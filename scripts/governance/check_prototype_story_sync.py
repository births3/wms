#!/usr/bin/env python3
"""check_prototype_story_sync.py — 原型与故事同步校验

类别：6. 原型治理
Tier：T1（< 10s）
输入：docs/prototypes/index.toml + docs/domain/user-stories-*.md
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- index.toml 中 story_id 对应的故事文件存在
- 如果故事文件 mtime > index.toml 中该条目的 last_reviewed（如有），发 warning
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
INDEX_TOML = REPO_ROOT / "docs" / "prototypes" / "index.toml"
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

STORY_ID_RE = re.compile(r"^## (US-[A-Z0-9]+-\d{3}[a-z]?)")


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _story_id_to_file() -> dict[str, Path]:
    """Map story_id -> file path."""
    mapping: dict[str, Path] = {}
    for f in DOMAIN_DIR.glob("user-stories-*.md"):
        for line in f.read_text(encoding="utf-8").splitlines():
            if line.startswith("## ") and "~~" not in line:
                m = STORY_ID_RE.match(line)
                if m:
                    mapping[m.group(1)] = f
    return mapping


def run() -> tuple[list[str], list[str]]:
    """Returns (errors, warnings)."""
    if not INDEX_TOML.exists():
        return (["docs/prototypes/index.toml 不存在"], [])

    data = _load_toml(INDEX_TOML)
    required = data.get("required", [])
    if not required:
        return ([], [])

    sid_to_file = _story_id_to_file()
    errors: list[str] = []
    warnings: list[str] = []

    for entry in required:
        sid = entry.get("story_id", "")
        slug = entry.get("prototype_slug", "")
        if not sid:
            continue

        if sid not in sid_to_file:
            errors.append(f"{sid}/{slug}: 故事文件中找不到该 story_id")
            continue

        # Check freshness: if story file modified after last_reviewed
        last_reviewed = entry.get("last_reviewed_at", "")
        if last_reviewed:
            story_file = sid_to_file[sid]
            file_mtime = os.path.getmtime(story_file)
            # Parse date string YYYY-MM-DD to timestamp (midnight)
            try:
                import datetime
                reviewed_dt = datetime.datetime.strptime(last_reviewed, "%Y-%m-%d")
                reviewed_ts = reviewed_dt.timestamp()
                if file_mtime > reviewed_ts:
                    warnings.append(
                        f"{sid}/{slug}: 故事文件在 last_reviewed_at({last_reviewed}) 之后被修改，建议重新走查"
                    )
            except ValueError:
                errors.append(f"{sid}/{slug}: last_reviewed_at 格式错误 '{last_reviewed}'")

    return (errors, warnings)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors, warnings = run()
    except Exception as e:
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({
            "status": "fail" if errors else "pass",
            "errors": errors,
            "warnings": warnings,
        }))
    else:
        if errors:
            print(f"✗ check_prototype_story_sync: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        elif warnings:
            print(f"⚠ check_prototype_story_sync: 通过（{len(warnings)} 项 warning）")
            for w in warnings:
                print(f"  - {w}")
        else:
            print("✓ check_prototype_story_sync: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
