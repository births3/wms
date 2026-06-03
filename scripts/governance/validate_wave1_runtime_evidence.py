#!/usr/bin/env python3
"""Validate Wave 1 runtime evidence JSON files without running the full report."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from report_wave1_completion import (
    REPO_ROOT,
    validate_h2_runtime_payload,
    validate_w1d_runtime_payload,
)


def read_json(path: Path) -> tuple[object | None, str | None]:
    if not path.exists():
        return None, f"missing file: {path}"
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except json.JSONDecodeError as error:
        return None, f"invalid JSON: {path}: {error}"


def validate_one(kind: str, path: Path, *, allow_example_refs: bool) -> tuple[bool, str]:
    payload, error = read_json(path)
    if error:
        return False, error
    if kind == "h2":
        ok, message = validate_h2_runtime_payload(payload, allow_example_refs=allow_example_refs)
    elif kind == "w1d":
        ok, message = validate_w1d_runtime_payload(payload, allow_example_refs=allow_example_refs)
    else:
        raise ValueError(f"unsupported kind: {kind}")
    return ok, f"{path}: {message}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kind", choices=["h2", "w1d", "all"], default="all")
    parser.add_argument(
        "--h2-file",
        default=REPO_ROOT / "docs/retros/wave-1-h2-runtime-evidence.json",
        type=Path,
    )
    parser.add_argument(
        "--w1d-file",
        default=REPO_ROOT / "docs/retros/wave-1-runtime-evidence.json",
        type=Path,
    )
    parser.add_argument(
        "--allow-example-refs",
        action="store_true",
        help="Allow example.* placeholder references; only use for validating .example.json templates.",
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    checks: list[tuple[str, Path]] = []
    if args.kind in {"h2", "all"}:
        checks.append(("h2", args.h2_file))
    if args.kind in {"w1d", "all"}:
        checks.append(("w1d", args.w1d_file))

    results = []
    for kind, path in checks:
        ok, message = validate_one(kind, path, allow_example_refs=args.allow_example_refs)
        results.append({"kind": kind, "path": str(path), "ok": ok, "message": message})

    all_ok = all(result["ok"] for result in results)
    if args.json:
        print(json.dumps({"ok": all_ok, "results": results}, ensure_ascii=False, indent=2))
    else:
        for result in results:
            mark = "✓" if result["ok"] else "✘"
            print(f"{mark} {result['kind']}: {result['message']}")

    return 0 if all_ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
