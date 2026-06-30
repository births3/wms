#!/usr/bin/env python3
"""check_baseline_completeness.py — 原型 baseline 完整性治理

类别：6. 原型治理
Tier：T1（强制；diff 触发：prototypes/src/Tabs.tsx 或 manifest.toml 改动）

校验三者一致性：
  1. Tabs.tsx 中 TABS 数组的 value （来源）
  2. governance/visual-baselines/manifest.toml 的 [[snapshots]].tab
  3. governance/visual-baselines/<file>.png 实际文件

任意一项缺失即报错（PR 阻断）：
  - 加了 tab 但忘记 manifest 条目 → "tab '{value}' 在 Tabs.tsx 但 manifest.toml 无条目"
  - 加了 tab 但忘记入 baseline → "tab '{value}' 在 manifest.toml 但 baseline PNG 文件不存在"
  - manifest 有条目但 Tabs.tsx 已删 → "tab '{value}' 在 manifest 但 Tabs.tsx 无对应"

依赖：仅 stdlib（无需 PIL）
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
TABS_FILE = REPO_ROOT / "prototypes" / "src" / "Tabs.tsx"
FULL_MATRIX_SPECS_FILE = REPO_ROOT / "prototypes" / "src" / "prototype-kit" / "full-matrix-specs.ts"
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
BASELINE_DIR = REPO_ROOT / "governance" / "visual-baselines"


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _extract_tab_values_from_tabs_tsx(content: str) -> list[str]:
    """从 Tabs.tsx 中抽取所有 tab 的 value 字符串。
    匹配形如：{ value: "h2-audit", ... }
    """
    # 匹配 value: "xxx" 形式（不在注释里）
    pattern = re.compile(r'\{\s*value:\s*"([a-z0-9-]+)"', re.MULTILINE)
    return pattern.findall(content)


def _extract_tab_values_from_full_matrix_specs(content: str) -> list[str]:
    """抽取数据驱动全量矩阵 tab 的 slug 字符串。"""
    pattern = re.compile(r'^\s*slug:\s*"([a-z0-9-]+)"', re.MULTILINE)
    return pattern.findall(content)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    errors: list[str] = []

    # 1. 读 Tabs.tsx
    if not TABS_FILE.exists():
        errors.append(f"缺少 {TABS_FILE.relative_to(REPO_ROOT)}")
        if args.json:
            print(json.dumps({"status": "fail", "errors": errors}))
        else:
            print(f"check_baseline_completeness — 致命错误")
            for e in errors:
                print(f"  - {e}")
        return 1

    tabs_tsx_values = _extract_tab_values_from_tabs_tsx(TABS_FILE.read_text(encoding="utf-8"))
    if FULL_MATRIX_SPECS_FILE.exists():
        tabs_tsx_values.extend(
            _extract_tab_values_from_full_matrix_specs(
                FULL_MATRIX_SPECS_FILE.read_text(encoding="utf-8")
            )
        )
    if not tabs_tsx_values:
        errors.append(f"Tabs.tsx 解析失败：未找到任何 tab value")

    # 2. 读 manifest.toml
    if not MANIFEST_TOML.exists():
        errors.append(f"缺少 {MANIFEST_TOML.relative_to(REPO_ROOT)}")
    manifest_data = _load_toml(MANIFEST_TOML) if MANIFEST_TOML.exists() else {"snapshots": []}
    manifest_snaps = manifest_data.get("snapshots", [])
    manifest_tabs = [s["tab"] for s in manifest_snaps]
    manifest_files = {s["tab"]: s.get("file") for s in manifest_snaps}

    # 3. 三者一致性校验
    set_tabs = set(tabs_tsx_values)
    set_manifest = set(manifest_tabs)

    # a) Tabs.tsx 有但 manifest 无
    missing_in_manifest = set_tabs - set_manifest
    for tab in sorted(missing_in_manifest):
        errors.append(f"tab '{tab}' 在 Tabs.tsx 但 manifest.toml 无 [[snapshots]] 条目（请加 manifest + 截图）")

    # b) manifest 有但 Tabs.tsx 已删
    orphan_in_manifest = set_manifest - set_tabs
    for tab in sorted(orphan_in_manifest):
        errors.append(f"tab '{tab}' 在 manifest.toml 但 Tabs.tsx 已无对应（请删 manifest 条目）")

    # c) baseline PNG 文件存在 + reviewed 字段必填 + reviewed_at >= PNG mtime
    import datetime
    for tab in set_tabs & set_manifest:
        snap = next((s for s in manifest_snaps if s["tab"] == tab), None)
        if snap is None:
            continue
        png_name = snap.get("file")
        if not png_name:
            errors.append(f"tab '{tab}' 在 manifest 缺 file 字段")
            continue
        png_path = BASELINE_DIR / png_name
        if not png_path.exists():
            errors.append(f"tab '{tab}' 缺 baseline PNG 文件：{png_path.relative_to(REPO_ROOT)}")
            continue
        if png_path.stat().st_size < 1024:  # < 1KB 视为空文件
            errors.append(f"tab '{tab}' baseline PNG 异常小（{png_path.stat().st_size} bytes，疑似截图失败）")
            continue

        # reviewed_by 必填（不能为空字符串或 'TODO'）
        reviewed_by = snap.get("reviewed_by", "").strip()
        if not reviewed_by or reviewed_by.lower() in ("todo", "tbd", "?", "-"):
            errors.append(f"tab '{tab}' manifest.reviewed_by 缺失或占位（'{reviewed_by}'），必须填实际 review 人")

        # reviewed_at 必填 + 格式 YYYY-MM-DD
        reviewed_at = snap.get("reviewed_at", "").strip()
        if not reviewed_at:
            errors.append(f"tab '{tab}' manifest.reviewed_at 缺失")
            continue
        try:
            datetime.date.fromisoformat(reviewed_at)
        except ValueError:
            errors.append(f"tab '{tab}' manifest.reviewed_at 格式错误（'{reviewed_at}'，需 YYYY-MM-DD）")
            continue

        # 注：PNG mtime 与 reviewed_at 的比较已移除，原因：
        # - mtime 在 cp / git checkout 时会被刷新，不可靠
        # - 'PNG 改了视觉是否健康' 由视觉回归和人工走查替代治理
        # - 真要追溯 review 历史，看 git log 即可

    # d) 反向：baseline 目录里有 .png 但 manifest 无引用
    referenced = {f for f in manifest_files.values() if f}
    for png in BASELINE_DIR.glob("*.png"):
        if png.name not in referenced:
            errors.append(f"baseline PNG 孤儿（manifest 无引用）：{png.name}")

    if args.json:
        print(json.dumps({
            "status": "fail" if errors else "pass",
            "errors": errors,
            "ok": not errors,
            "tabs_in_code": sorted(set_tabs),
            "tabs_in_manifest": sorted(set_manifest),
        }))
    else:
        total = len(set_tabs)
        print(f"check_baseline_completeness — Tabs.tsx({len(set_tabs)}) ↔ manifest({len(set_manifest)}) ↔ PNG({len(referenced)})")
        if errors:
            print(f"  ✘ {len(errors)} violation(s):")
            for e in errors:
                print(f"    - {e}")
        else:
            print(f"  ✓ 全部 {total} 个 tab 三者一致")
            print(f"  规范：加新 page → 同步加 Tabs.tsx + manifest.toml + 跑 capture 入 baseline")

    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
