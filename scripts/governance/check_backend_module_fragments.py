#!/usr/bin/env python3
"""检查后端 Rust 生产 include 是否登记在历史基线。

输入：backend/crates/api/src/**/*.rs、governance/backend-module-fragments-baseline.toml。
输出：人类可读结果或 --json 结果，包含发现、基线、已消失和新增生产 include。
退出码：0 通过；1 发现基线外生产 include；2 检查器自身错误。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 runner
    import tomli as tomllib


ROOT = Path(__file__).resolve().parents[2]
SOURCE_ROOT = ROOT / "backend" / "crates" / "api" / "src"
BASELINE_FILE = ROOT / "governance" / "backend-module-fragments-baseline.toml"
INCLUDE_RE = re.compile(r'include!\s*\(\s*"([^"]+)"\s*\)')


def _relative(path: Path, root: Path = ROOT) -> str:
    return path.relative_to(root).as_posix()


def _fragment_key(parent: str, include: str) -> str:
    return f"{parent}::{include}"


def _is_test_file(path: Path) -> bool:
    """Skip standalone test source files whose include is not production code."""
    return (
        any(part in {"test", "tests"} for part in path.parts)
        or path.stem in {"test", "tests"}
        or path.stem.startswith("test_")
        or path.stem.endswith("_test")
        or path.stem.endswith("_tests")
    )


def _test_module_ranges(text: str) -> list[tuple[int, int]]:
    """Return line ranges enclosed by a #[cfg(test)] module.

    This deliberately uses a small brace scanner: the gate only needs to avoid
    treating test fixtures as production module fragments.
    """
    ranges: list[tuple[int, int]] = []
    lines = text.splitlines()
    pending_cfg = False
    brace_depth = 0
    module_depth: int | None = None
    module_start = 0

    for index, line in enumerate(lines):
        if "#[cfg(test)]" in line:
            pending_cfg = True

        if pending_cfg and re.search(r"\bmod\s+[A-Za-z_]\w*", line) and "{" in line:
            module_depth = brace_depth + line.count("{") - line.count("}")
            module_start = index
            pending_cfg = False

        brace_depth += line.count("{") - line.count("}")

        if module_depth is not None and brace_depth < module_depth:
            ranges.append((module_start, index))
            module_depth = None

    if module_depth is not None:
        ranges.append((module_start, len(lines) - 1))
    return ranges


def _line_is_in_ranges(line_number: int, ranges: list[tuple[int, int]]) -> bool:
    return any(start <= line_number <= end for start, end in ranges)


def discover_fragments(
    source_root: Path = SOURCE_ROOT, root: Path = ROOT
) -> list[dict[str, str]]:
    fragments: list[dict[str, str]] = []
    for parent_path in sorted(source_root.rglob("*.rs")):
        if _is_test_file(parent_path):
            continue
        text = parent_path.read_text(encoding="utf-8", errors="ignore")
        test_ranges = _test_module_ranges(text)
        for line_number, line in enumerate(text.splitlines()):
            if _line_is_in_ranges(line_number, test_ranges):
                continue
            for match in INCLUDE_RE.finditer(line):
                include = match.group(1)
                parent = _relative(parent_path, root)
                fragments.append(
                    {
                        "parent": parent,
                        "include": include,
                        "key": _fragment_key(parent, include),
                    }
                )
    return sorted(fragments, key=lambda item: item["key"])


def load_baseline(path: Path = BASELINE_FILE) -> set[str]:
    payload = tomllib.loads(path.read_text(encoding="utf-8"))
    return {
        _fragment_key(str(item["parent"]), str(item["include"]))
        for item in payload.get("fragments", ())
    }


def check(
    *,
    source_root: Path = SOURCE_ROOT,
    root: Path = ROOT,
    baseline_path: Path = BASELINE_FILE,
) -> dict[str, object]:
    discovered = discover_fragments(source_root, root)
    discovered_keys = {item["key"] for item in discovered}
    baseline = load_baseline(baseline_path)
    new_violations = sorted(discovered_keys - baseline)
    resolved = sorted(baseline - discovered_keys)
    remaining = sorted(discovered_keys & baseline)
    return {
        "check": "check_backend_module_fragments",
        "tier": "T2",
        "category": "后端模块治理",
        "ok": not new_violations,
        "baseline_count": len(baseline),
        "discovered_count": len(discovered_keys),
        "remaining_count": len(remaining),
        "remaining": remaining,
        "resolved": resolved,
        "new_violations": new_violations,
        "message": (
            "发现基线外的生产 include"
            if new_violations
            else "后端生产 include 无新增"
        ),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)
    try:
        result = check()
    except (OSError, tomllib.TOMLDecodeError, KeyError, ValueError) as error:
        result = {
            "check": "check_backend_module_fragments",
            "tier": "T2",
            "category": "后端模块治理",
            "ok": False,
            "error": str(error),
        }

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print("check_backend_module_fragments (T2, 后端模块治理)")
        print(result.get("message", result.get("error", "")))
        print(
            f"discovered={result.get('discovered_count', 0)} "
            f"baseline={result.get('baseline_count', 0)} "
            f"resolved={len(result.get('resolved', []))}"
        )
        for key in result.get("new_violations", []):
            print(f"  NEW {key}")
    return 0 if result.get("ok") else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
