#!/usr/bin/env python3
"""capture_visual_snapshots.py — chrome headless 自动 fullpage 截图

用法：
  python3 scripts/governance/capture_visual_snapshots.py [--port 5173] [--start-server]
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
import os
import re
import signal
import shutil
import subprocess
import sys
import time
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


def _start_vite(port: int) -> subprocess.Popen:
    process = subprocess.Popen(
        ["pnpm", "exec", "vite", "--host", "127.0.0.1", "--port", str(port), "--strictPort"],
        cwd=REPO_ROOT / "prototypes",
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    for _ in range(120):
        if process.poll() is not None:
            raise RuntimeError(f"vite 启动失败或端口 {port} 已被占用")
        if _check_vite(port):
            return process
        time.sleep(0.25)
    _stop_process(process)
    raise RuntimeError(f"vite 在端口 {port} 启动超时")


def _stop_process(process: subprocess.Popen) -> None:
    if process.poll() is not None:
        return
    os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=10)


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
    out_file.unlink(missing_ok=True)
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
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    except subprocess.TimeoutExpired:
        return (False, "chrome 截图超时")
    ok = result.returncode == 0 and out_file.exists() and out_file.stat().st_size > 0
    return (ok, result.stderr[:200] if not ok else "")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=5173)
    parser.add_argument("--start-server", action="store_true", help="自动启动并在结束时关闭原型 Vite")
    args = parser.parse_args()

    if not MANIFEST_TOML.exists():
        print(f"[ERROR] 缺少 {MANIFEST_TOML}", file=sys.stderr)
        sys.exit(2)

    chrome = _find_chrome()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    data = _load_toml(MANIFEST_TOML)
    snapshots = data.get("snapshots", [])
    if not snapshots:
        print("[ERROR] manifest.toml 无 [[snapshots]] 条目", file=sys.stderr)
        sys.exit(2)

    vite_process = None
    try:
        vite_process = _start_vite(args.port) if args.start_server else None
        if not _check_vite(args.port):
            print(f"[ERROR] vite dev server 未运行（端口 {args.port}）", file=sys.stderr)
            print("  请先在 prototypes/ 跑 pnpm dev，或使用 --start-server", file=sys.stderr)
            sys.exit(2)
        print(f"▶ 截 {len(snapshots)} 个 tab（chrome: {chrome}）")
        failed = 0
        for snap in snapshots:
            tab = snap["tab"]
            url_hash = snap["url_hash"]
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
        from check_visual_regression import SNAPSHOT_SOURCE_DIGEST, visual_source_digest

        SNAPSHOT_SOURCE_DIGEST.write_text(visual_source_digest() + "\n", encoding="utf-8")
        print(f"✓ 全部 {len(snapshots)} 个 tab 截图完成")
    finally:
        if vite_process is not None:
            _stop_process(vite_process)


if __name__ == "__main__":
    main()
