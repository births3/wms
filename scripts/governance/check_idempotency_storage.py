#!/usr/bin/env python3
"""检查幂等表直写基线只能收缩，禁止新增重复实现。"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 runner
    import tomli as tomllib

ROOT = Path(__file__).resolve().parents[2]
SOURCE_ROOT = ROOT / "backend" / "crates" / "api" / "src"
BASELINE_FILE = ROOT / "governance" / "idempotency-direct-access-baseline.toml"
SHARED_MODULE = "backend/crates/api/src/idempotency.rs"
TOKEN = "idempotency_request"


def _relative(path: Path, root: Path = ROOT) -> str:
    return path.relative_to(root).as_posix()


def discover_direct_access(source_root: Path = SOURCE_ROOT, root: Path = ROOT) -> list[str]:
    return sorted(
        _relative(path, root)
        for path in source_root.rglob("*.rs")
        if TOKEN in path.read_text(encoding="utf-8", errors="ignore")
    )


def load_baseline(path: Path = BASELINE_FILE) -> set[str]:
    payload = tomllib.loads(path.read_text(encoding="utf-8"))
    return {str(item["path"]) for item in payload.get("direct_access", [])}


def check(
    *, source_root: Path = SOURCE_ROOT, root: Path = ROOT, baseline_path: Path = BASELINE_FILE
) -> dict[str, object]:
    direct = set(discover_direct_access(source_root, root))
    baseline = load_baseline(baseline_path)
    tracked = baseline | {SHARED_MODULE}
    new_violations = sorted(direct - tracked)
    resolved = sorted(baseline - direct)
    remaining = sorted(direct & baseline)
    return {
        "check": "check_idempotency_storage",
        "tier": "T1",
        "category": "后端治理",
        "ok": not new_violations,
        "shared_module": SHARED_MODULE,
        "baseline_count": len(baseline),
        "remaining_count": len(remaining),
        "remaining": remaining,
        "resolved": resolved,
        "new_violations": new_violations,
        "message": (
            "幂等表直接访问没有新增，基线可继续收缩"
            if not new_violations
            else "发现未登记的幂等表直接访问"
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    try:
        result = check()
    except (OSError, tomllib.TOMLDecodeError, KeyError) as error:
        result = {
            "check": "check_idempotency_storage",
            "tier": "T1",
            "category": "后端治理",
            "ok": False,
            "error": str(error),
        }
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(f"{result['check']} ({result['tier']}, {result['category']})")
        print(result.get("message", result.get("error", "")))
        print(f"remaining={result.get('remaining_count', 0)} resolved={len(result.get('resolved', []))}")
        for path in result.get("new_violations", []):
            print(f"  NEW {path}")
    return 0 if result.get("ok") else 1


if __name__ == "__main__":
    sys.exit(main())
