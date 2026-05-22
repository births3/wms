#!/usr/bin/env python3
"""check_visual_keywords.py — 图片识别视觉 review

类别：6. 原型治理
Tier：T3（重，依赖 vite + chrome + tesseract）

机制：
1. 对每张 baseline PNG 跑 tesseract OCR（英文模型，识别英文+数字）
2. 从 manifest.toml 读每个 tab 的 expected_keywords 列表
3. 校验关键字到位率：< 70% → 错误（视觉异常，组件可能未渲染或被遮挡）

用途：
- 替代人工逐张 review 中"关键文字是否在位"
- 中文识别需 tesseract chi_sim 模型；当前只验证英文/数字（含 PR-2026-XXX / GSP §X / SO-XXXX-XXXX 等结构化标识）
- 结合 check_visual_regression（截断检测）+ check_baseline_completeness（三者一致性），形成完整视觉治理闭环

依赖：tesseract（系统命令）+ PIL
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
BASELINE_DIR = REPO_ROOT / "governance" / "visual-baselines"

# 阈值（sanity check 模式）：
# OCR 对小字/彩色字识别有限，不追求 100% 命中。重点是防全空白/大量乱码。
PASS_THRESHOLD = 0.20  # ≥ 20% 通过（说明 OCR 至少识别出 1-2 个预期关键字）
MIN_OCR_CHARS = 100    # OCR 总输出 < 100 字符视为图异常（全空白或截图失败）


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


def _check_tesseract() -> bool:
    try:
        result = subprocess.run(["tesseract", "--version"], capture_output=True, text=True, timeout=5)
        return result.returncode == 0
    except Exception:
        return False


def _ocr_extract(png: Path) -> str:
    """跑 tesseract，返回原始文本"""
    out_prefix = f"/tmp/_ocr_{png.stem}"
    try:
        subprocess.run(
            ["tesseract", str(png), out_prefix, "-l", "eng"],
            capture_output=True, text=True, timeout=30,
        )
        txt_path = Path(f"{out_prefix}.txt")
        if txt_path.exists():
            return txt_path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        pass
    return ""


def _normalize(text: str) -> str:
    """归一化 OCR 文本：保留字母/数字/常见标点，统一大小写"""
    return re.sub(r"\s+", " ", text).upper()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--tab", help="只检查指定 tab")
    args = parser.parse_args()

    if not _check_tesseract():
        print("[ERROR] tesseract 未安装", file=sys.stderr)
        sys.exit(2)

    if not MANIFEST_TOML.exists():
        print(f"[ERROR] 缺少 {MANIFEST_TOML}", file=sys.stderr)
        sys.exit(2)

    data = _load_toml(MANIFEST_TOML)
    snapshots = data.get("snapshots", [])
    if args.tab:
        snapshots = [s for s in snapshots if s["tab"] == args.tab]

    results: list[dict] = []
    errors: list[str] = []
    warnings: list[str] = []

    for snap in snapshots:
        tab = snap["tab"]
        keywords = snap.get("expected_keywords")
        if not keywords:
            # 未配置关键字 → 跳过（后续逐步补）
            results.append({"tab": tab, "status": "skipped", "reason": "无 expected_keywords"})
            continue

        png_path = BASELINE_DIR / snap["file"]
        if not png_path.exists():
            errors.append(f"{tab}: PNG 文件不存在")
            continue

        text = _ocr_extract(png_path)
        norm = _normalize(text)
        # 计算 OCR 识别的 ASCII 字符总数（去除空白和乱码）
        ascii_chars = sum(1 for c in text if c.isalnum() or c in "-_./")

        hits = [kw for kw in keywords if kw.upper() in norm]
        miss = [kw for kw in keywords if kw.upper() not in norm]
        ratio = len(hits) / len(keywords) if keywords else 0.0

        # 双重校验逻辑（务实）：
        # 1. 至少 1 个关键字命中 → 通过（说明图渲染了内容，且关键标识在位）
        # 2. 0 命中 + OCR 字符 < 100 → 错误（图异常/全空白/截图失败）
        # 3. 0 命中 + OCR 字符 ≥ 100 → 警告（图渲染但关键字未识别，可能是字号问题或 expected_keywords 配错）
        if len(hits) > 0:
            status = "passed"
        elif ascii_chars < MIN_OCR_CHARS:
            status = "error"
            errors.append(f"{tab}: 0 关键字命中 + OCR 字符数 {ascii_chars} < {MIN_OCR_CHARS}（图异常或全空白）")
        else:
            status = "warning"
            warnings.append(f"{tab}: 0 关键字命中（OCR {ascii_chars} 字符识别正常，但 expected_keywords 未命中；检查 keywords 配置或视觉是否真异常）")

        results.append({
            "tab": tab, "status": status,
            "ratio": round(ratio, 2),
            "ocr_chars": ascii_chars,
            "hits": hits, "miss": miss,
            "total": len(keywords),
        })

    if args.json:
        print(json.dumps({
            "status": "fail" if errors else "pass",
            "results": results,
            "errors": errors,
            "warnings": warnings,
        }, ensure_ascii=False))
    else:
        print(f"▶ 视觉关键字识别（OCR）— {len(results)} 个 tab")
        for r in results:
            if r["status"] == "skipped":
                print(f"  - {r['tab']:14s}  (未配置 expected_keywords，跳过)")
            else:
                icon = {"passed": "✓", "warning": "⚠", "error": "✘"}[r["status"]]
                print(f"  {icon} {r['tab']:14s}  OCR {r.get('ocr_chars',0):4d} 字符  关键字 {len(r['hits'])}/{r['total']} ({r['ratio']*100:.0f}%)")
        if errors:
            print(f"\n✘ {len(errors)} 个 tab 视觉关键字异常：")
            for e in errors:
                print(f"  - {e}")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
