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
MATRIX_MD = REPO_ROOT / "docs" / "prototypes" / "prototype-matrix-r3.md"
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"
PROTO_DIR = REPO_ROOT / "prototypes"
TABS_FILE = PROTO_DIR / "src" / "Tabs.tsx"
FULL_MATRIX_SPECS_FILE = PROTO_DIR / "src" / "prototype-kit" / "full-matrix-specs.ts"

VALID_PRIORITIES = {"P0", "P1", "P2", "P3", "P4"}
VALID_ENDS = {"pda", "pc", "pad", "h5"}
STORY_ID_RE = re.compile(r"^## (?:~~)?(US-[A-Z0-9]+-\d{3}[a-z]?)(?:~~)?")
TAB_VALUE_RE = re.compile(r'\{\s*value:\s*"([a-z0-9-]+)"', re.MULTILINE)
SPEC_SLUG_RE = re.compile(r'^\s*slug:\s*"([a-z0-9-]+)"', re.MULTILINE)
MATRIX_ROW_RE = re.compile(r"^\|\s*(\d+)\s*\|\s*(US-[A-Z0-9]+-\d{3}[a-z]?)\s*\|")


END_MAP = {
    "PC": "pc",
    "PDA": "pda",
    "PAD": "pad",
    "H5": "h5",
}


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
            if line.startswith("## "):
                m = STORY_ID_RE.match(line)
                if m:
                    ids.add(m.group(1))
    return ids


def _collect_tab_values() -> set[str]:
    values: set[str] = set()
    if not TABS_FILE.exists():
        return values
    values.update(TAB_VALUE_RE.findall(TABS_FILE.read_text(encoding="utf-8")))
    if FULL_MATRIX_SPECS_FILE.exists():
        values.update(SPEC_SLUG_RE.findall(FULL_MATRIX_SPECS_FILE.read_text(encoding="utf-8")))
    return values


def _slug_for(story_id: str, end: str) -> str:
    return f"{end}-{story_id[3:].lower()}"


def _collect_matrix_required() -> set[tuple[str, str, str]]:
    """Return required (story_id, end, slug) from the full prototype matrix."""
    if not MATRIX_MD.exists():
        return set()

    required: set[tuple[str, str, str]] = set()
    for line in MATRIX_MD.read_text(encoding="utf-8").splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 7:
            continue
        if not cells[0].isdigit() or not cells[1].startswith("US-"):
            continue
        story_id = cells[1]
        exempt_col = cells[4]
        if "豁免" in exempt_col:
            continue
        for raw_end in cells[3].split("+"):
            raw_end = raw_end.strip()
            if raw_end == "后端":
                continue
            end = END_MAP.get(raw_end)
            if end:
                required.add((story_id, end, _slug_for(story_id, end)))
    return required


def run() -> list[str]:
    if not INDEX_TOML.exists():
        return ["docs/prototypes/index.toml 不存在"]

    data = _load_toml(INDEX_TOML)
    required = data.get("required", [])
    if not required:
        return []

    story_ids = _collect_story_ids()
    tab_values = _collect_tab_values()
    matrix_required = _collect_matrix_required()
    errors: list[str] = []
    slugs_seen: set[str] = set()
    index_required: set[tuple[str, str, str]] = set()

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

        if slug and slug not in tab_values:
            errors.append(f"{prefix}: prototype_slug '{slug}' 未在 prototypes/src/Tabs.tsx 中注册")

        if priority not in VALID_PRIORITIES:
            errors.append(f"{prefix}: priority '{priority}' 不合法，应为 P0-P4")

        if end not in VALID_ENDS:
            errors.append(f"{prefix}: end '{end}' 不合法，应为 pda/pc/pad/h5")
        elif sid and slug:
            index_required.add((sid, end, slug))

    if matrix_required:
        missing = matrix_required - index_required
        extra = index_required - matrix_required
        for sid, end, slug in sorted(missing):
            errors.append(f"prototype-matrix-r3.md: {sid}/{end}/{slug} 未进入 docs/prototypes/index.toml")
        for sid, end, slug in sorted(extra):
            errors.append(f"docs/prototypes/index.toml: {sid}/{end}/{slug} 不在 prototype-matrix-r3.md 非豁免清单中")

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
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors, "ok": not errors}))
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
