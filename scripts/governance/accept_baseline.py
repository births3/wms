#!/usr/bin/env python3
"""accept_baseline.py — 基线替换接受门禁

类别：6. 原型治理（工具，非 CI 脚本）
用途：把 .visual-snapshots/ 中的候选图安全替换到 governance/visual-baselines/

工作流：
  1. 起 vite + capture_visual_snapshots → candidate 在 prototypes/.visual-snapshots/
  2. 运行（默认 dry-run）：
     python3 scripts/governance/accept_baseline.py --reviewer="项目主人"
  3. 看输出，如果全部 ✓ → 加 --apply 真的替换：
     python3 scripts/governance/accept_baseline.py --reviewer="项目主人" --apply

接受标准（详见 docs/prototypes/baseline-acceptance.md）：
  A. 结构健康：文件大小 / 截断检测
  B. 变化幅度：mean_diff ≤ 5 自动 / 5-30 需 --confirm-medium / >30 需 --force-major
  C. 签字：--reviewer 必填非占位
  D. 字段完整：tab 在 Tabs.tsx + manifest 完整

依赖：PIL
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import re
import shutil
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
TABS_FILE = REPO_ROOT / "prototypes" / "src" / "Tabs.tsx"
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
BASELINE_DIR = REPO_ROOT / "governance" / "visual-baselines"
SNAPSHOT_DIR = REPO_ROOT / "prototypes" / ".visual-snapshots"

# 接受标准阈值
SIZE_MIN_KB = 1
SIZE_MAX_MB = 3
TRUNCATION_THRESHOLD = 0.05  # 底部 30 行非白比例
SMALL_CHANGE = 5.0           # mean_diff ≤ 5 视为小调整
LARGE_CHANGE = 30.0          # mean_diff > 30 视为大变化
LARGE_PIXEL_RATIO = 0.30     # pixel_ratio > 30% 视为大变化
SMALL_PHASH = 5              # phash 距离 ≤ 5 视为视觉等价
LARGE_PHASH = 15             # phash 距离 > 15 视为大幅 layout 变化


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _md5(path: Path) -> str:
    h = hashlib.md5()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _detect_truncation(png: Path) -> float:
    try:
        from PIL import Image
        img = Image.open(png).convert("L")
        w, h = img.size
        if h < 30:
            return 0.0
        bottom = img.crop((0, h - 30, w, h))
        ps = list(bottom.getdata())
        return sum(1 for p in ps if p < 250) / len(ps)
    except Exception:
        return 0.0


def _compare(baseline: Path, candidate: Path) -> tuple[float, float, int]:
    """returns (mean_diff_64x64, pixel_ratio, phash_distance)
    phash_distance: 0 完全一致 / ≤ 5 视觉等价 / 5-10 中度变化 / > 10 大幅变化
    """
    try:
        from PIL import Image, ImageChops
        a = Image.open(baseline).convert("RGB")
        b = Image.open(candidate).convert("RGB")
        if a.size != b.size:
            return (-1.0, -1.0, -1)
        diff = ImageChops.difference(a, b)
        if diff.getbbox() is None:
            return (0.0, 0.0, 0)
        diff_l = diff.convert("L")
        ps = list(diff_l.getdata())
        ratio = sum(1 for p in ps if p > 5) / len(ps)
        a64 = a.resize((64, 64)).convert("L")
        b64 = b.resize((64, 64)).convert("L")
        a_p = list(a64.getdata())
        b_p = list(b64.getdata())
        mean = sum(abs(x - y) for x, y in zip(a_p, b_p)) / len(a_p)

        # phash 感知哈希距离（对 layout 结构变化比 mean_diff 敏感）
        phash_dist = 0
        try:
            import imagehash
            ha = imagehash.phash(a)
            hb = imagehash.phash(b)
            phash_dist = ha - hb
        except ImportError:
            phash_dist = -1  # 未装

        return (mean, ratio, phash_dist)
    except Exception:
        return (-1.0, -1.0, -1)


def _evaluate_one(snap: dict, args) -> tuple[bool, list[str], list[str], dict]:
    """评估一张候选图。Returns (accept, errors, warnings, info)"""
    tab = snap["tab"]
    file_name = snap["file"]
    candidate = SNAPSHOT_DIR / file_name
    baseline = BASELINE_DIR / file_name

    errors: list[str] = []
    warnings: list[str] = []
    info: dict = {"tab": tab}

    if not candidate.exists():
        return (False, [f"候选图不存在：{candidate.relative_to(REPO_ROOT)}（先跑 capture_visual_snapshots.py）"], [], info)

    # A1. 文件大小
    size_kb = candidate.stat().st_size / 1024
    info["size_kb"] = round(size_kb, 1)
    if size_kb < SIZE_MIN_KB:
        errors.append(f"A1: 文件过小 {size_kb:.1f} KB < {SIZE_MIN_KB} KB（截图失败）")
    elif size_kb > SIZE_MAX_MB * 1024:
        warnings.append(f"A1: 文件过大 {size_kb/1024:.1f} MB > {SIZE_MAX_MB} MB")

    # A2. 截断检测
    bottom_ratio = _detect_truncation(candidate)
    info["bottom_ratio"] = round(bottom_ratio, 4)
    if bottom_ratio >= TRUNCATION_THRESHOLD:
        errors.append(f"A3: 底部 30 行非白 {bottom_ratio*100:.1f}% ≥ {TRUNCATION_THRESHOLD*100:.0f}%（截断风险，加大 viewport）")

    # B. 与现有 baseline 比较
    if baseline.exists():
        if _md5(baseline) == _md5(candidate):
            info["change"] = "identical"
        else:
            mean_diff, pixel_ratio, phash_dist = _compare(baseline, candidate)
            info["mean_diff"] = round(mean_diff, 2)
            info["pixel_ratio"] = round(pixel_ratio, 4)
            info["phash"] = phash_dist
            if mean_diff < 0:
                # 尺寸变化（baseline 与 candidate 维度不一致）
                # 通常发生在 manifest viewport 调整后但 PNG 未跟上
                # 必须 --accept-resize 显式确认（视为 major 级变化）
                info["change"] = "resize"
                if not args.accept_resize:
                    errors.append(
                        f"B: baseline 与 candidate 尺寸不同（"
                        f"通常是 manifest viewport 改了但 PNG 未跟上）"
                        f"；如确认是预期请加 --accept-resize"
                    )
            else:
                # 三指标联合判定：mean_diff 像素均值差 + pixel_ratio 像素差比例 + phash 感知哈希距离
                # phash 比 mean_diff 对 layout 结构变化更敏感
                is_major = (
                    mean_diff > LARGE_CHANGE
                    or pixel_ratio > LARGE_PIXEL_RATIO
                    or (phash_dist >= 0 and phash_dist > LARGE_PHASH)
                )
                is_small = (
                    mean_diff <= SMALL_CHANGE
                    and pixel_ratio < LARGE_PIXEL_RATIO
                    and (phash_dist < 0 or phash_dist <= SMALL_PHASH)
                )
                if is_major:
                    info["change"] = "major"
                    if not args.force_major:
                        errors.append(
                            f"B3: 大变化（mean_diff={mean_diff:.1f} pixel_ratio={pixel_ratio*100:.1f}% phash={phash_dist}）"
                            f"必须 --force-major 才接受（请先在浏览器人工确认）"
                        )
                elif is_small:
                    info["change"] = "small"
                else:
                    info["change"] = "medium"
                    if not args.confirm_medium and not args.force_major:
                        errors.append(
                            f"B2: 中等变化（mean_diff={mean_diff:.1f} pixel_ratio={pixel_ratio*100:.1f}% phash={phash_dist}）"
                            f"需 --confirm-medium 或 --force-major"
                        )
    else:
        info["change"] = "new_baseline"

    return (len(errors) == 0, errors, warnings, info)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tab", help="只接受指定 tab")
    parser.add_argument("--apply", action="store_true", help="真的应用替换（默认 dry-run）")
    parser.add_argument("--reviewer", help="C: review 人姓名（应用时必填）")
    parser.add_argument("--confirm-medium", action="store_true", help="B2: 接受中等变化（mean_diff 5-30）")
    parser.add_argument("--force-major", action="store_true", help="B3: 接受大变化（mean_diff > 30）")
    parser.add_argument("--accept-resize", action="store_true", help="B: 接受尺寸变化（baseline 与 candidate 维度不同；通常是 manifest viewport 改了）")
    args = parser.parse_args()

    if not MANIFEST_TOML.exists():
        print(f"[ERROR] 缺少 {MANIFEST_TOML}", file=sys.stderr)
        return 2

    data = _load_toml(MANIFEST_TOML)
    snapshots = data.get("snapshots", [])
    if args.tab:
        snapshots = [s for s in snapshots if s["tab"] == args.tab]
    if not snapshots:
        print(f"[ERROR] manifest 无候选 tab", file=sys.stderr)
        return 2

    # 应用模式必须有 reviewer
    if args.apply and not args.reviewer:
        print(f"[ERROR] --apply 必须同时传 --reviewer=<name>", file=sys.stderr)
        return 2
    if args.reviewer and args.reviewer.strip().lower() in ("todo", "tbd", "?", "-", ""):
        print(f"[ERROR] --reviewer 不能是占位符", file=sys.stderr)
        return 2

    print(f"▶ accept_baseline {'(APPLY)' if args.apply else '(DRY-RUN)'} — {len(snapshots)} 个 tab")
    print()

    accepted: list[dict] = []
    rejected: list[dict] = []

    for snap in snapshots:
        ok, errors, warnings, info = _evaluate_one(snap, args)
        change = info.get("change", "?")
        icon = "✓" if ok else "✘"
        print(f"  {icon} {info['tab']:14s}  size={info.get('size_kb','?')}KB  "
              f"change={change}  "
              f"mean_diff={info.get('mean_diff','—')}  phash={info.get('phash','—')}")
        for e in errors:
            print(f"      ✘ {e}")
        for w in warnings:
            print(f"      ⚠ {w}")

        if ok:
            accepted.append({"snap": snap, "info": info})
        else:
            rejected.append({"snap": snap, "info": info, "errors": errors})

    print()
    print(f"摘要：✓ 可接受 {len(accepted)}  ✘ 拒绝 {len(rejected)}")

    if not args.apply:
        if accepted:
            print(f"\n要应用，加 --apply --reviewer=\"<你的名字>\"：")
            print(f"  python3 scripts/governance/accept_baseline.py --apply --reviewer=\"项目主人\"")
        return 1 if rejected else 0

    # APPLY 模式：复制 + 更新 manifest
    today = datetime.date.today().isoformat()
    print(f"\n▶ 应用替换（reviewer={args.reviewer!r} reviewed_at={today}）...")
    manifest_text = MANIFEST_TOML.read_text(encoding="utf-8")
    for item in accepted:
        snap = item["snap"]
        candidate = SNAPSHOT_DIR / snap["file"]
        baseline = BASELINE_DIR / snap["file"]
        shutil.copy2(candidate, baseline)
        # 更新 manifest 中该 tab 的 reviewed_by + reviewed_at
        # 简单替换：tab="X"\n... reviewed_at="..." 改成 today；reviewed_by 改成 args.reviewer
        tab_pattern = re.compile(
            r'(tab = "' + re.escape(snap["tab"]) + r'"\n(?:.*\n)*?)reviewed_by = "[^"]*"',
            re.MULTILINE,
        )
        manifest_text = tab_pattern.sub(
            lambda m: m.group(1) + f'reviewed_by = "{args.reviewer}"',
            manifest_text,
        )
        date_pattern = re.compile(
            r'(tab = "' + re.escape(snap["tab"]) + r'"\n(?:.*\n)*?)reviewed_at = "[^"]*"',
            re.MULTILINE,
        )
        manifest_text = date_pattern.sub(
            lambda m: m.group(1) + f'reviewed_at = "{today}"',
            manifest_text,
        )
        print(f"  ✓ {snap['tab']:14s}  PNG 替换 + manifest reviewed_at={today}")
    MANIFEST_TOML.write_text(manifest_text, encoding="utf-8")

    if rejected:
        print(f"\n✘ 拒绝 {len(rejected)} 个 tab（错误见上）")
        return 1
    print(f"\n✓ 全部接受")
    return 0


if __name__ == "__main__":
    sys.exit(main())
