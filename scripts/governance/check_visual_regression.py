#!/usr/bin/env python3
"""check_visual_regression.py — 视觉回归对比

类别：6. 原型治理
Tier：T3（重，依赖 vite + chrome 提前生成 snapshot）
输入：governance/visual-baselines/*.png + prototypes/.visual-snapshots/*.png
输出：人类可读 + --json
退出码：0 通过 / 1 大差异（PR 阻断）/ 2 脚本错误

校验流程：
1. 对每个 manifest.toml 中的 snapshot：
   a. 计算 baseline 和 snapshot 的 MD5 → 一致 → ✓ 完美
   b. MD5 不同 → 用 PIL 加载 → 计算 mean_diff (64×64 灰度均值差) + pixel_diff_ratio (像素级不同比例)
2. 阈值（参 governance/visual-baselines/README.md）：
   - mean_diff ≤ 2 + pixel_diff_ratio ≤ 0.5%        → ✓ 完美 / 等价
   - mean_diff 2-10 或 pixel_diff_ratio 0.5-3%      → ⚠ 警告
   - mean_diff > 10 或 pixel_diff_ratio > 3%        → ✘ 错误
3. 写差异图到 prototypes/.visual-diffs/<tab>.diff.png（红色叠加差异区域）

依赖：PIL (Pillow)
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
BASELINE_DIR = REPO_ROOT / "governance" / "visual-baselines"
SNAPSHOT_DIR = REPO_ROOT / "prototypes" / ".visual-snapshots"
DIFF_DIR = REPO_ROOT / "prototypes" / ".visual-diffs"

# 阈值
MEAN_DIFF_WARN = 2.0
MEAN_DIFF_ERR = 10.0
PIXEL_RATIO_WARN = 0.005  # 0.5%
PIXEL_RATIO_ERR = 0.03    # 3%


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


def _compare_pixels(baseline: Path, snapshot: Path, diff_out: Path | None = None) -> tuple[float, float, str]:
    """Returns (mean_diff_64x64, pixel_diff_ratio, error_msg)"""
    try:
        from PIL import Image, ImageChops
    except ImportError:
        return (0.0, 0.0, "PIL 未安装")

    try:
        img_a = Image.open(baseline).convert("RGB")
        img_b = Image.open(snapshot).convert("RGB")
    except Exception as e:
        return (0.0, 0.0, f"加载失败: {e}")

    if img_a.size != img_b.size:
        return (-1.0, -1.0, f"尺寸不同 baseline={img_a.size} snapshot={img_b.size}")

    # 像素级差异比例
    diff = ImageChops.difference(img_a, img_b)
    bbox = diff.getbbox()
    if bbox is None:
        return (0.0, 0.0, "")  # 完全一致
    # 用 getdata 计算非零像素数
    diff_l = diff.convert("L")
    pixels = list(diff_l.getdata())
    total = len(pixels)
    nonzero = sum(1 for p in pixels if p > 5)  # 阈值 5 (256 灰度)，过滤抗锯齿微差
    pixel_ratio = nonzero / total if total else 0.0

    # 64×64 灰度均值差（感知差异）
    a64 = img_a.resize((64, 64)).convert("L")
    b64 = img_b.resize((64, 64)).convert("L")
    a_pix = list(a64.getdata())
    b_pix = list(b64.getdata())
    mean_diff = sum(abs(x - y) for x, y in zip(a_pix, b_pix)) / len(a_pix)

    # 写差异图
    if diff_out is not None and bbox:
        diff_out.parent.mkdir(parents=True, exist_ok=True)
        # 红色叠加
        red = Image.new("RGB", img_a.size, (255, 0, 0))
        mask = diff_l.point(lambda p: 255 if p > 5 else 0)
        composite = Image.composite(red, img_a, mask)
        composite.save(diff_out)

    return (mean_diff, pixel_ratio, "")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-diff-image", action="store_true", help="不生成差异图")
    args = parser.parse_args()

    if not MANIFEST_TOML.exists():
        print(f"[ERROR] 缺少 {MANIFEST_TOML}", file=sys.stderr)
        sys.exit(2)
    if not SNAPSHOT_DIR.exists() or not any(SNAPSHOT_DIR.iterdir()):
        print(f"[ERROR] 无 snapshot：先跑 capture_visual_snapshots.py", file=sys.stderr)
        sys.exit(2)

    data = _load_toml(MANIFEST_TOML)
    snapshots = data.get("snapshots", [])

    results: list[dict] = []
    errors: list[str] = []
    warnings: list[str] = []

    for snap in snapshots:
        tab = snap["tab"]
        baseline = BASELINE_DIR / snap["file"]
        snapshot = SNAPSHOT_DIR / snap["file"]

        if not baseline.exists():
            errors.append(f"{tab}: baseline 缺失 {snap['file']}")
            continue
        if not snapshot.exists():
            errors.append(f"{tab}: snapshot 缺失（先跑 capture）")
            continue

        # 1) MD5 短路
        md5_a = _md5(baseline)
        md5_b = _md5(snapshot)
        if md5_a == md5_b:
            results.append({"tab": tab, "status": "identical", "mean_diff": 0.0, "pixel_ratio": 0.0})
            continue

        # 2) 像素差异
        diff_out = None if args.no_diff_image else DIFF_DIR / snap["file"].replace(".png", ".diff.png")
        mean_diff, pixel_ratio, err = _compare_pixels(baseline, snapshot, diff_out)
        if err:
            errors.append(f"{tab}: 像素对比失败 - {err}")
            continue

        # 分级
        if mean_diff > MEAN_DIFF_ERR or pixel_ratio > PIXEL_RATIO_ERR:
            level = "error"
            msg = f"{tab}: 视觉回归（mean_diff={mean_diff:.2f}, pixel_ratio={pixel_ratio*100:.2f}%）"
            errors.append(msg)
            if diff_out:
                errors.append(f"  ↳ 差异图: {diff_out.relative_to(REPO_ROOT)}")
        elif mean_diff > MEAN_DIFF_WARN or pixel_ratio > PIXEL_RATIO_WARN:
            level = "warning"
            msg = f"{tab}: 轻微差异（mean_diff={mean_diff:.2f}, pixel_ratio={pixel_ratio*100:.2f}%）"
            warnings.append(msg)
        else:
            level = "passed"
        results.append({
            "tab": tab, "status": level,
            "mean_diff": round(mean_diff, 2),
            "pixel_ratio": round(pixel_ratio, 4),
        })

    if args.json:
        print(json.dumps({
            "status": "fail" if errors else "pass",
            "errors": errors,
            "warnings": warnings,
            "results": results,
            "thresholds": {
                "mean_diff_warn": MEAN_DIFF_WARN,
                "mean_diff_err": MEAN_DIFF_ERR,
                "pixel_ratio_warn": PIXEL_RATIO_WARN,
                "pixel_ratio_err": PIXEL_RATIO_ERR,
            },
        }))
    else:
        print(f"▶ 视觉回归（{len(snapshots)} 个 tab）")
        for r in results:
            icon = {"identical": "✓", "passed": "✓", "warning": "⚠", "error": "✘"}[r["status"]]
            tag = "完美" if r["status"] == "identical" else r["status"]
            print(f"  {icon} {r['tab']:20s} {tag:8s} mean_diff={r['mean_diff']:.2f}  pixel_ratio={r['pixel_ratio']*100:.2f}%")
        if warnings:
            print(f"\n⚠ {len(warnings)} 项警告：")
            for w in warnings:
                print(f"  - {w}")
        if errors:
            print(f"\n✘ {len(errors)} 项错误（PR 阻断）：")
            for e in errors:
                print(f"  - {e}")
            print(f"\n  → 看差异图：prototypes/.visual-diffs/")
            print(f"  → 如视觉变化是预期：cp prototypes/.visual-snapshots/*.png governance/visual-baselines/")
        elif not warnings:
            print(f"\n✓ check_visual_regression: 全部通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
