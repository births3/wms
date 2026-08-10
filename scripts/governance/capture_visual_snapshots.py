#!/usr/bin/env python3
"""capture_visual_snapshots.py — Playwright 复用 Chrome 自动截图

用法：
  python3 scripts/governance/capture_visual_snapshots.py [--port 5173] [--start-server]
  → 输出到 prototypes/.visual-snapshots/<tab>.png

机制：
  1. 用 manifest.toml 配置的 viewport 宽高
  2. 由一个 Playwright 进程复用同一个 Chrome 浏览器
  3. 每个页面使用独立浏览器上下文，避免 localStorage 等状态串页

依赖：google-chrome 或 chromium-browser
"""
from __future__ import annotations

import argparse
import json
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
PLAYWRIGHT_CAPTURE = REPO_ROOT / "scripts" / "governance" / "capture_visual_snapshots.mjs"

# 高度上限（极少触发；超大页面如 gallery）
MAX_HEIGHT = 4000
CHROME_HEADLESS_FRAME_HEIGHT = 87


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
    """返回 manifest 配置的截图高度。"""
    _ = chrome, url, width
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


def _capture_batch(
    chrome: str,
    port: int,
    jobs: list[dict[str, object]],
) -> dict[str, tuple[bool, str]]:
    """用一个 Playwright/Chrome 进程捕获全部页面，避免逐页冷启动浏览器。"""
    payload = {
        "chrome": chrome,
        "base_url": f"http://127.0.0.1:{port}/",
        "jobs": jobs,
    }
    command = [
        "pnpm",
        "--dir",
        str(REPO_ROOT / "prototypes"),
        "exec",
        "node",
        str(PLAYWRIGHT_CAPTURE),
    ]
    try:
        result = subprocess.run(
            command,
            input=json.dumps(payload),
            capture_output=True,
            text=True,
            timeout=max(60, len(jobs) * 15),
        )
    except subprocess.TimeoutExpired:
        return {str(job["tab"]): (False, "Playwright 批量截图超时") for job in jobs}

    if result.returncode != 0:
        error = result.stderr.strip()[:400] or "Playwright 批量截图失败"
        return {str(job["tab"]): (False, error) for job in jobs}
    try:
        capture_results = json.loads(result.stdout).get("results", [])
    except (AttributeError, json.JSONDecodeError):
        return {str(job["tab"]): (False, "Playwright 返回了无效结果") for job in jobs}
    return {
        str(item["tab"]): (bool(item.get("ok")), str(item.get("error", "")))
        for item in capture_results
        if isinstance(item, dict) and "tab" in item
    }


def _snapshots_are_fresh(
    snapshots: list[dict[str, object]],
    output_dir: Path,
    recorded_digest: str,
    current_digest: str,
) -> bool:
    """源码摘要一致且 manifest 中每个快照都存在时才允许复用。"""
    if not recorded_digest or recorded_digest != current_digest:
        return False
    return all(
        isinstance(snapshot.get("file"), str)
        and (output_dir / str(snapshot["file"])).is_file()
        and (output_dir / str(snapshot["file"])).stat().st_size > 0
        for snapshot in snapshots
    )


def _pad_screenshot_to_window(out_file: Path, width: int, height: int) -> tuple[bool, str]:
    """复现 Chrome CLI：内容区按窗口框架缩小，输出 PNG 底部补白到配置高度。"""
    try:
        from PIL import Image

        with Image.open(out_file) as source:
            image = source.convert("RGB")
        if image.width != width or image.height > height:
            return (False, f"截图尺寸异常: {image.size}，期望宽 {width}、高不超过 {height}")
        if image.height == height:
            return (True, "")
        padded = Image.new("RGB", (width, height), "white")
        padded.paste(image, (0, 0))
        padded.save(out_file)
        return (True, "")
    except Exception as error:  # noqa: BLE001
        return (False, f"截图补白失败: {error}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=5173)
    parser.add_argument("--start-server", action="store_true", help="自动启动并在结束时关闭原型 Vite")
    parser.add_argument("--if-stale", action="store_true", help="源码摘要和快照完整时跳过重复捕获")
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
    if args.if_stale:
        from check_visual_regression import SNAPSHOT_SOURCE_DIGEST, visual_source_digest

        recorded_digest = (
            SNAPSHOT_SOURCE_DIGEST.read_text(encoding="utf-8").strip()
            if SNAPSHOT_SOURCE_DIGEST.exists()
            else ""
        )
        if _snapshots_are_fresh(snapshots, OUTPUT_DIR, recorded_digest, visual_source_digest()):
            print(f"✓ {len(snapshots)} 个视觉快照与当前源码摘要一致，跳过重复捕获")
            return

    vite_process = None
    try:
        vite_process = _start_vite(args.port) if args.start_server else None
        if not _check_vite(args.port):
            print(f"[ERROR] vite dev server 未运行（端口 {args.port}）", file=sys.stderr)
            print("  请先在 prototypes/ 跑 pnpm dev，或使用 --start-server", file=sys.stderr)
            sys.exit(2)
        print(f"▶ 截 {len(snapshots)} 个 tab（chrome: {chrome}）")
        failed = 0
        jobs: list[dict[str, object]] = []
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
            out_file.unlink(missing_ok=True)
            jobs.append(
                {
                    "tab": tab,
                    "url_hash": url_hash,
                    "width": w,
                    "height": h,
                    "viewport_height": max(1, h - CHROME_HEADLESS_FRAME_HEIGHT),
                    "out_file": str(out_file),
                }
            )

        capture_results = _capture_batch(chrome, args.port, jobs)
        for job in jobs:
            tab = str(job["tab"])
            w = int(job["width"])
            h = int(job["height"])
            out_file = Path(str(job["out_file"]))
            ok, err = capture_results.get(tab, (False, "Playwright 未返回该页面结果"))
            if ok:
                ok, err = _pad_screenshot_to_window(out_file, w, h)
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
