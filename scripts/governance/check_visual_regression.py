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
VISUAL_SOURCE_DIRS = [
    REPO_ROOT / "prototypes" / "src",
    REPO_ROOT / "prototypes" / "public",
    REPO_ROOT / "packages" / "ui" / "src",
]
VISUAL_SOURCE_FILES = [
    MANIFEST_TOML,
    REPO_ROOT / "prototypes" / "package.json",
    REPO_ROOT / "prototypes" / "index.html",
    REPO_ROOT / "prototypes" / "postcss.config.js",
    REPO_ROOT / "prototypes" / "tailwind.config.js",
    REPO_ROOT / "prototypes" / "vite.config.ts",
    REPO_ROOT / "packages" / "ui" / "package.json",
    REPO_ROOT / "packages" / "ui" / "tailwind-preset.cjs",
    REPO_ROOT / "pnpm-lock.yaml",
]
SNAPSHOT_SOURCE_DIGEST = SNAPSHOT_DIR / ".source.sha256"

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


def _source_digest(source_files: list[Path], *, root: Path = REPO_ROOT) -> str:
    """按相对路径和内容计算确定性摘要，不受 mtime 影响。"""
    digest = hashlib.sha256()
    for path in sorted((path for path in source_files if path.is_file()), key=lambda item: str(item)):
        try:
            display = path.relative_to(root)
        except ValueError:
            display = path
        digest.update(str(display).encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def visual_source_files() -> list[Path]:
    return [
        *VISUAL_SOURCE_FILES,
        *[
            path
            for source_dir in VISUAL_SOURCE_DIRS
            for path in source_dir.rglob("*")
            if path.is_file()
        ],
    ]


def visual_source_digest() -> str:
    return _source_digest(visual_source_files())


def _exit_error(message: str, *, json_mode: bool, exit_code: int) -> None:
    if json_mode:
        print(json.dumps({"status": "error", "errors": [message], "warnings": [], "truncations": [], "results": []}))
    else:
        print(f"[ERROR] {message}", file=sys.stderr)
    sys.exit(exit_code)


def _detect_truncation(snapshot: Path) -> tuple[bool, float]:
    """检测截图底部是否被截断
    思路：取底部 30 行像素，统计非白色像素比例。
    - 比例 < 5%：底部基本是白色（页面完整结束）
    - 比例 >= 5%：底部有内容（页面可能被截断）
    返回 (is_truncated, ratio)
    """
    try:
        from PIL import Image
    except ImportError:
        return (False, 0.0)
    try:
        img = Image.open(snapshot).convert("L")  # 灰度
    except Exception:
        return (False, 0.0)
    w, h = img.size
    if h < 30:
        return (False, 0.0)
    # 取底部 30 行
    bottom = img.crop((0, h - 30, w, h))
    pixels = list(bottom.getdata())
    # 非白色像素：灰度 < 250
    nonwhite = sum(1 for p in pixels if p < 250)
    ratio = nonwhite / len(pixels) if pixels else 0.0
    # 阈值 5%：如果底部 30 行有 ≥5% 非白像素，认为可能截断
    return (ratio >= 0.05, ratio)


def _compare_pixels(baseline: Path, snapshot: Path, diff_out: Path | None = None) -> tuple[float, float, str, int]:
    """Returns (mean_diff_64x64, pixel_diff_ratio, error_msg, phash_distance)
    phash_distance: -1 未计算，0 完全一致，越大结构越不同
    """
    try:
        from PIL import Image, ImageChops
    except ImportError:
        return (0.0, 0.0, "PIL 未安装", -1)

    try:
        img_a = Image.open(baseline).convert("RGB")
        img_b = Image.open(snapshot).convert("RGB")
    except Exception as e:
        return (0.0, 0.0, f"加载失败: {e}", -1)

    if img_a.size != img_b.size:
        return (-1.0, -1.0, f"尺寸不同 baseline={img_a.size} snapshot={img_b.size}", -1)

    diff = ImageChops.difference(img_a, img_b)
    bbox = diff.getbbox()
    if bbox is None:
        return (0.0, 0.0, "", 0)
    diff_l = diff.convert("L")
    pixels = list(diff_l.getdata())
    total = len(pixels)
    nonzero = sum(1 for p in pixels if p > 5)
    pixel_ratio = nonzero / total if total else 0.0

    a64 = img_a.resize((64, 64)).convert("L")
    b64 = img_b.resize((64, 64)).convert("L")
    a_pix = list(a64.getdata())
    b_pix = list(b64.getdata())
    mean_diff = sum(abs(x - y) for x, y in zip(a_pix, b_pix)) / len(a_pix)

    phash_dist = -1
    try:
        import imagehash
        phash_dist = imagehash.phash(img_a) - imagehash.phash(img_b)
    except ImportError:
        pass

    if diff_out is not None and bbox:
        diff_out.parent.mkdir(parents=True, exist_ok=True)
        red = Image.new("RGB", img_a.size, (255, 0, 0))
        mask = diff_l.point(lambda p: 255 if p > 5 else 0)
        composite = Image.composite(red, img_a, mask)
        composite.save(diff_out)

    return (mean_diff, pixel_ratio, "", phash_dist)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-diff-image", action="store_true", help="不生成差异图")
    args = parser.parse_args()

    if not MANIFEST_TOML.exists():
        _exit_error(f"缺少 {MANIFEST_TOML}", json_mode=args.json, exit_code=2)
    if not SNAPSHOT_DIR.exists() or not any(SNAPSHOT_DIR.iterdir()):
        _exit_error("无 snapshot：先跑 capture_visual_snapshots.py", json_mode=args.json, exit_code=2)

    data = _load_toml(MANIFEST_TOML)
    snapshots = data.get("snapshots", [])
    recorded_digest = SNAPSHOT_SOURCE_DIGEST.read_text(encoding="utf-8").strip() if SNAPSHOT_SOURCE_DIGEST.exists() else ""
    current_digest = visual_source_digest()
    if recorded_digest != current_digest:
        message = "snapshot 源码摘要缺失或已变化；先运行 capture_visual_snapshots.py 生成本次构建截图"
        if args.json:
            print(json.dumps({"status": "fail", "errors": [message], "warnings": [], "truncations": [], "results": []}))
        else:
            print(f"[ERROR] {message}", file=sys.stderr)
        sys.exit(1)

    results: list[dict] = []
    errors: list[str] = []
    warnings: list[str] = []
    truncations: list[str] = []

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

        # 0) 截断检测（snapshot 底部是否非空白）
        is_trunc, trunc_ratio = _detect_truncation(snapshot)
        if is_trunc:
            truncations.append(f"{tab}: 底部 30 行非白像素 {trunc_ratio*100:.1f}%（可能被截断，建议加大 viewport 高度）")

        # 1) MD5 短路
        md5_a = _md5(baseline)
        md5_b = _md5(snapshot)
        if md5_a == md5_b:
            results.append({"tab": tab, "status": "identical", "mean_diff": 0.0, "pixel_ratio": 0.0})
            continue

        # 2) 像素差异
        diff_out = None if args.no_diff_image else DIFF_DIR / snap["file"].replace(".png", ".diff.png")
        mean_diff, pixel_ratio, err, phash_dist = _compare_pixels(baseline, snapshot, diff_out)
        if err:
            errors.append(f"{tab}: 像素对比失败 - {err}")
            continue

        # 三指标联合分级（mean_diff / pixel_ratio / phash）
        is_phash_large = phash_dist >= 0 and phash_dist > 15
        if mean_diff > MEAN_DIFF_ERR or pixel_ratio > PIXEL_RATIO_ERR or is_phash_large:
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
            "truncations": truncations,
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
        if truncations:
            print(f"\n⚠ {len(truncations)} 项底部截断：")
            for t in truncations:
                print(f"  - {t}")
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
