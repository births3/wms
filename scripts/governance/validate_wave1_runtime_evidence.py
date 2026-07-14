#!/usr/bin/env python3
"""Validate Wave 1 runtime evidence JSON files without running the full report."""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from _wave_evidence_validator import evidence_execution_status
from report_wave1_completion import (
    REPO_ROOT,
    validate_h2_runtime_payload,
    validate_w1d_runtime_payload,
)


H2_STAGING_BOUNDARY_RE = re.compile(r"(^|[^0-9a-z])staging([^0-9a-z]|$)", re.IGNORECASE)


def read_json(path: Path) -> tuple[object | None, str | None]:
    if not path.exists():
        return None, f"missing file: {path}"
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except json.JSONDecodeError as error:
        return None, f"invalid JSON: {path}: {error}"


def h2_staging_ref_fields(payload: object) -> list[str]:
    if not isinstance(payload, dict):
        return []
    performance = payload.get("performance")
    seal_cron = payload.get("seal_cron")
    fields: list[str] = []
    if isinstance(performance, dict) and H2_STAGING_BOUNDARY_RE.search(
        str(performance.get("benchmark_log_ref", ""))
    ):
        fields.append("performance.benchmark_log_ref")
    if isinstance(seal_cron, dict) and H2_STAGING_BOUNDARY_RE.search(
        str(seal_cron.get("cron_log_ref", ""))
    ):
        fields.append("seal_cron.cron_log_ref")
    return fields


def validate_one(kind: str, path: Path, *, allow_example_refs: bool) -> tuple[bool, str]:
    payload, error = read_json(path)
    if error:
        return False, error
    if kind == "h2":
        ok, message = validate_h2_runtime_payload(payload, allow_example_refs=allow_example_refs)
        staging_fields = h2_staging_ref_fields(payload)
        if ok and staging_fields:
            ok = False
            message = (
                "H2 runtime evidence 必须是 dev 边界，不能包含 staging 引用: "
                + ", ".join(staging_fields)
            )
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
        help=(
            "Allow refs containing example domain tokens when validating .example.json templates; "
            "template placeholders are still rejected."
        ),
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
        results.append({
            "kind": kind,
            "path": str(path),
            "ok": ok,
            "status": evidence_execution_status(ok, message),
            "message": message,
        })

    all_ok = all(result["ok"] for result in results)
    statuses = {result["status"] for result in results}
    status = "passed" if all_ok else ("failed" if "failed" in statuses else "blocked")
    if args.json:
        print(json.dumps({
            "ok": all_ok,
            "status": status,
            "results": results,
        }, ensure_ascii=False, indent=2))
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
