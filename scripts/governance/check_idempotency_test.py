#!/usr/bin/env python3
"""check_idempotency_test.py — Wave 3 T3：幂等测试存在性最小门禁

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
BACKEND = ROOT / "backend"
PATTERNS = ("idempotency", "Idempotency-Key", "idempotency_key")


def run_check() -> dict:
    if not BACKEND.is_dir():
        return {
            "check": "check_idempotency_test",
            "tier": "T3",
            "category": "后端治理",
            "ok": False,
            "source_hits": 0,
            "test_hits": 0,
            "samples": [],
            "message": "backend/ 不存在",
        }

    hits: list[str] = []
    for path in BACKEND.rglob("*.rs"):
        if "target" in path.parts:
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        if any(token in text for token in PATTERNS):
            hits.append(str(path.relative_to(ROOT)))

    test_hits = [
        h
        for h in hits
        if "/tests/" in h or h.endswith("_tests.rs") or "test" in Path(h).name.lower()
    ]
    ok = len(test_hits) > 0
    return {
        "check": "check_idempotency_test",
        "tier": "T3",
        "category": "后端治理",
        "ok": ok,
        "source_hits": len(hits),
        "test_hits": len(test_hits),
        "samples": test_hits[:5],
        "message": "幂等测试存在性门禁通过（最小实现）" if ok else "未找到幂等相关测试",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Wave 3 T3：幂等测试存在性最小门禁")
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args()
    try:
        result = run_check()
    except Exception as exc:  # noqa: BLE001
        if args.json:
            print(json.dumps({"check": "check_idempotency_test", "tier": "T3", "category": "后端治理", "ok": False, "error": str(exc)}, ensure_ascii=False))
        else:
            print(f"check_idempotency_test 脚本错误: {exc}")
        return 2

    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print("check_idempotency_test (T3, 后端治理)")
        print(f"  · 含幂等关键字源文件: {result['source_hits']}")
        print(f"  · 其中测试相关: {result['test_hits']}")
        print(f"  {'✓' if result['ok'] else '✘'} {result['message']}")
        for sample in result.get("samples", []):
            print(f"    · {sample}")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
