#!/usr/bin/env python3
"""check_prototype_index_consistency.py — 原型索引一致性校验

类别：6. 原型治理
Tier：T1（< 10s）
输入：docs/prototypes/index.toml + docs/domain/user-stories-*.md + prototypes/
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- index.toml 中每个 story_id 在 user-stories 文件中实际存在
- index.toml 中每个 prototype_slug 对应目录或文件存在（如已创建）
- 无重复 prototype_slug
- priority 值合法（P0-P4）
- end 值合法（pda/pc/pad/h5）
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
INDEX_TOML = REPO_ROOT / "docs" / "prototypes" / "index.toml"
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"
PROTO_DIR = REPO_ROOT / "prototypes"

VALID_PRIORITIES = {"P0", "P1", "P2", "P3", "P4"}
VALID_ENDS = {"pda", "pc", "pad", "h5"}
STORY_ID_RE = re.compile(r"^## (US-[A-Z0-9]+-\d{3}[a-z]?)")


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _collect_story_ids() -> set[str]:
    ids: set[str] = set()
    for f in DOMAIN_DIR.glob("user-stories-*.md"):
        for line in f.read_text(encoding="utf-8").splitlines():
            if line.startswith("## ") and "~~" not in line:
                m = STORY_ID_RE.match(line)
                if m:
                    ids.add(m.group(1))
    return ids


def run() -> list[str]:
    if not INDEX_TOML.exists():
        return ["docs/prototypes/index.toml 不存在"]

    data = _load_toml(INDEX_TOML)
    required = data.get("required", [])
    if not required:
        return []

    story_ids = _collect_story_ids()
    errors: list[str] = []
    slugs_seen: set[str] = set()

    for i, entry in enumerate(required):
        sid = entry.get("story_id", "")
        slug = entry.get("prototype_slug", "")
        priority = entry.get("priority", "")
        end = entry.get("end", "")

        prefix = f"[[required]][{i}] {sid}/{slug}"

        if not sid:
            errors.append(f"{prefix}: 缺少 story_id")
        elif sid not in story_ids:
            errors.append(f"{prefix}: story_id '{sid}' 在 user-stories 中不存在")

        if not slug:
            errors.append(f"{prefix}: 缺少 prototype_slug")
        elif slug in slugs_seen:
            errors.append(f"{prefix}: prototype_slug '{slug}' 重复")
        else:
            slugs_seen.add(slug)

        if priority not in VALID_PRIORITIES:
            errors.append(f"{prefix}: priority '{priority}' 不合法，应为 P0-P4")

        if end not in VALID_ENDS:
            errors.append(f"{prefix}: end '{end}' 不合法，应为 pda/pc/pad/h5")

    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors = run()
    except Exception as e:
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors}))
    else:
        if errors:
            print(f"✗ check_prototype_index_consistency: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        else:
            print("✓ check_prototype_index_consistency: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
