#!/usr/bin/env python3
"""capture_visual_snapshots.py — 调用 chrome headless 生成 11 个 tab 的 PNG

用法：
  cd prototypes && pnpm dev &     # 先起 vite
  python3 scripts/governance/capture_visual_snapshots.py [--port 5173]
  → 输出到 prototypes/.visual-snapshots/<tab>.png

依赖：google-chrome 或 chromium-browser

脚本读取 governance/visual-baselines/manifest.toml，按 viewport 截每个 tab
"""
from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
OUTPUT_DIR = REPO_ROOT / "prototypes" / ".visual-snapshots"


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _find_chrome() -> str:
    for c in ["google-chrome", "chromium-browser", "chromium", "chrome"]:
        path = shutil.which(c)
        if path:
            return path
    raise RuntimeError("未找到 chrome/chromium，无法截图")


def _check_vite(port: int) -> bool:
    try:
        import urllib.request
        urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=2).read()
        return True
    except Exception:
        return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=5173)
    args = parser.parse_args()

    if not MANIFEST_TOML.exists():
        print(f"[ERROR] 缺少 {MANIFEST_TOML}", file=sys.stderr)
        sys.exit(2)

    if not _check_vite(args.port):
        print(f"[ERROR] vite dev server 未运行（端口 {args.port}）", file=sys.stderr)
        print(f"  请先在 prototypes/ 跑：pnpm dev", file=sys.stderr)
        sys.exit(2)

    chrome = _find_chrome()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    data = _load_toml(MANIFEST_TOML)
    snapshots = data.get("snapshots", [])
    if not snapshots:
        print("[ERROR] manifest.toml 无 [[snapshots]] 条目", file=sys.stderr)
        sys.exit(2)

    print(f"▶ 截 {len(snapshots)} 个 tab（chrome: {chrome}）")
    failed = 0
    for snap in snapshots:
        tab = snap["tab"]
        url_hash = snap["url_hash"]
        viewport = snap["viewport"]  # "1500x950"
        out_file = OUTPUT_DIR / snap["file"]

        url = f"http://localhost:{args.port}/{url_hash}"
        cmd = [
            chrome,
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            f"--window-size={viewport.replace('x', ',')}",
            "--hide-scrollbars",
            f"--screenshot={out_file}",
            url,
        ]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if not out_file.exists() or out_file.stat().st_size == 0:
            print(f"  ✘ {tab:20s} 截图失败")
            print(f"    stderr: {result.stderr[:200]}")
            failed += 1
        else:
            size_kb = out_file.stat().st_size // 1024
            print(f"  ✓ {tab:20s} {viewport:>10s}  {size_kb} KB")

    print(f"\n输出目录: {OUTPUT_DIR}")
    if failed:
        print(f"✘ {failed} 个失败", file=sys.stderr)
        sys.exit(1)
    print(f"✓ 全部 {len(snapshots)} 个 tab 截图完成")


if __name__ == "__main__":
    main()
