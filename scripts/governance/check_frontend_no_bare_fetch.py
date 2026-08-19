#!/usr/bin/env python3
"""禁止生产 apps/**/src 绕过共享 API 客户端直接调用 fetch。"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SOURCE_SUFFIXES = {".js", ".jsx", ".ts", ".tsx"}
FETCH_RE = re.compile(r"\bfetch\s*\(")
SHARED_CLIENT_ALLOWLIST = {REPO_ROOT / "packages" / "api-client" / "src" / "client.ts"}


def source_files(root: Path) -> list[Path]:
    return sorted(
        path
        for app_src in root.glob("apps/*/src")
        for path in app_src.rglob("*")
        if path.is_file() and path.suffix in SOURCE_SUFFIXES
    )


def scan(paths: list[Path], *, allowlist: set[Path] | None = None) -> list[dict[str, object]]:
    allowed = allowlist or set()
    violations: list[dict[str, object]] = []
    for path in paths:
        if path in allowed:
            continue
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if FETCH_RE.search(line):
                violations.append({"path": str(path), "line": line_number, "text": line.strip()})
    return violations


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)

    paths = source_files(REPO_ROOT)
    violations = scan(paths, allowlist=SHARED_CLIENT_ALLOWLIST)
    ok = not violations
    payload = {
        "check": "check_frontend_no_bare_fetch",
        "tier": "T1",
        "category": "前端治理",
        "ok": ok,
        "scanned_files": len(paths),
        "allowlist": [str(path.relative_to(REPO_ROOT)) for path in sorted(SHARED_CLIENT_ALLOWLIST)],
        "violations": violations,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_frontend_no_bare_fetch (T1, 前端治理) — scanned {len(paths)} files")
        if ok:
            print("  ✓ apps/**/src 未发现裸 fetch")
        else:
            for violation in violations:
                print(f"  ✘ {violation['path']}:{violation['line']} {violation['text']}", file=sys.stderr)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
