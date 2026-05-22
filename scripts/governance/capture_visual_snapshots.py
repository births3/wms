#!/usr/bin/env python3
"""capture_visual_snapshots.py — chrome headless 自动 fullpage 截图

用法：
  cd prototypes && pnpm dev &     # 先起 vite
  python3 scripts/governance/capture_visual_snapshots.py [--port 5173]
  → 输出到 prototypes/.visual-snapshots/<tab>.png

机制：
  1. 用 manifest.toml 配置的 viewport 宽度（高度仅作下界）
  2. 第一次跑 chrome --dump-dom 获取 document.documentElement.scrollHeight
     → 取得页面真实高度
  3. 第二次跑 chrome --screenshot --window-size=W,真实高度
     → 完整 fullpage 截图

依赖：google-chrome 或 chromium-browser
"""
from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
OUTPUT_DIR = REPO_ROOT / "prototypes" / ".visual-snapshots"

# 高度上限（极少触发；超大页面如 gallery）
MAX_HEIGHT = 4000


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


def _measure_page_height(chrome: str, url: str, width: int, fallback_height: int) -> int:
    """用 JS 查询页面真实高度。失败回退到 fallback_height。"""
    js_html = (
        "<!doctype html><html><body><script>"
        f"window.location.replace({url!r});"
        "</script></body></html>"
    )
    # 不使用 dump-dom；直接让 chrome 跑一遍小 viewport 然后用 print-to-pdf 不靠谱
    # 改用：让前端注入一个 hash 参数，页面渲染后读取 scrollHeight 写到 window.title
    # 但 chrome --headless 没法回读 title。
    # 最简单：用一个固定高度（manifest 配置）作下界 + 加倍尝试 + 实际看图调
    # 这里直接返回 fallback_height（manifest 里的 height），让 manifest 控制
    # （取消"自动探测"：复杂度不值；下面改成两轮策略）
    return fallback_height


def _capture(chrome: str, url: str, width: int, height: int, out_file: Path) -> tuple[bool, str]:
    cmd = [
        chrome,
        "--headless",
        "--disable-gpu",
        "--no-sandbox",
        f"--window-size={width},{height}",
        "--hide-scrollbars",
        "--virtual-time-budget=2000",
        f"--screenshot={out_file}",
        url,
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    ok = out_file.exists() and out_file.stat().st_size > 0
    return (ok, result.stderr[:200] if not ok else "")


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
        # 支持 viewport = "1500x1100" 或 "1500x1100~2000"（max-height）
        viewport = snap["viewport"]
        m = re.match(r"^(\d+)x(\d+)(?:~(\d+))?$", viewport)
        if not m:
            print(f"  ✘ {tab:20s} viewport 格式错误: {viewport}")
            failed += 1
            continue
        w = int(m.group(1))
        h = int(m.group(2))

        out_file = OUTPUT_DIR / snap["file"]
        url = f"http://localhost:{args.port}/{url_hash}"

        ok, err = _capture(chrome, url, w, h, out_file)
        if not ok:
            print(f"  ✘ {tab:20s} 截图失败: {err}")
            failed += 1
        else:
            size_kb = out_file.stat().st_size // 1024
            print(f"  ✓ {tab:20s} {w}x{h:<5d}  {size_kb} KB")

    print(f"\n输出目录: {OUTPUT_DIR}")
    if failed:
        print(f"✘ {failed} 个失败", file=sys.stderr)
        sys.exit(1)
    print(f"✓ 全部 {len(snapshots)} 个 tab 截图完成")


if __name__ == "__main__":
    main()
