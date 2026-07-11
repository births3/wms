#!/usr/bin/env python3
"""check_audit_trail_coverage.py — Wave 3 T3：API 写路径审计覆盖最小门禁

类别：后端治理
Tier：T3
输出：人类可读 + --json
退出码：0 通过 / 1 失败 / 2 脚本错误
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
API_SRC = ROOT / "backend" / "crates" / "api" / "src"


def run_check() -> dict:
    if not API_SRC.is_dir():
        return {
            "check": "check_audit_trail_coverage",
            "tier": "T3",
            "category": "后端治理",
            "ok": False,
            "scanned": 0,
            "audit_hits": 0,
            "message": "backend/crates/api/src 不存在",
        }

    handlers = list(API_SRC.rglob("*handlers*.rs")) + list(API_SRC.rglob("*handler*.rs"))
    audit_hits = 0
    for path in handlers:
        text = path.read_text(encoding="utf-8", errors="ignore")
        lower = text.lower()
        if "audit" in lower or "append_only" in text or "append-only" in text:
            audit_hits += 1

    ok = len(handlers) > 0 and audit_hits > 0
    return {
        "check": "check_audit_trail_coverage",
        "tier": "T3",
        "category": "后端治理",
        "ok": ok,
        "scanned": len(handlers),
        "audit_hits": audit_hits,
        "message": "审计关键字覆盖门禁通过（最小实现）" if ok else "未在 handler 中发现 audit 引用",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Wave 3 T3：API 写路径审计覆盖最小门禁")
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args()
    try:
        result = run_check()
    except Exception as exc:  # noqa: BLE001 — 脚本边界
        if args.json:
            print(json.dumps({"check": "check_audit_trail_coverage", "tier": "T3", "category": "后端治理", "ok": False, "error": str(exc)}, ensure_ascii=False))
        else:
            print(f"check_audit_trail_coverage 脚本错误: {exc}")
        return 2

    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print("check_audit_trail_coverage (T3, 后端治理)")
        print(f"  · 扫描 handler 文件: {result['scanned']}")
        print(f"  · 含 audit 关键字: {result['audit_hits']}")
        print(f"  {'✓' if result['ok'] else '✘'} {result['message']}")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
