#!/usr/bin/env python3
"""check_handler_test_coverage.py — API handler 测试覆盖起步校验

类别：3. 质量治理
Tier：T2（< 10s）
输入：backend/crates/api/src/lib.rs + backend/crates/api/**/*.rs
输出：人类可读 + --json
退出码：
  0  通过
  1  缺少最小测试覆盖
  2  脚本自身错误

Wave 1 baseline 最小规则：
- 统计 `#[utoipa::path]` 数量
- 若存在 path，则 api crate 测试源必须逐一覆盖每个 OpenAPI path 字面量
  （tests/*.rs 或 `#[cfg(test)]` / `#[test]` 所在文件均算）
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
API_CRATE = REPO_ROOT / "backend" / "crates" / "api"
API_LIB = API_CRATE / "src" / "lib.rs"

UTOIPA_PATH_RE = re.compile(r"#\s*\[\s*utoipa::path\b")
PATH_LITERAL_RE = re.compile(r'path\s*=\s*"([^"]+)"')
TEST_MARKER_RE = re.compile(r"#\s*\[(?:cfg\s*\(\s*test\s*\)|test)\s*\]")


@dataclass
class Issue:
    kind: str
    detail: str


def extract_utoipa_paths(text: str) -> list[str]:
    seen: set[str] = set()
    paths: list[str] = []
    for path in PATH_LITERAL_RE.findall(text):
        if path not in seen:
            seen.add(path)
            paths.append(path)
    return paths


def has_test_markers(text: str) -> bool:
    return bool(TEST_MARKER_RE.search(text))


def extract_test_text(path: Path, text: str) -> str:
    if "tests" in path.parts:
        return text
    match = TEST_MARKER_RE.search(text)
    if not match:
        return ""
    return text[match.start():]


def collect_test_sources(api_crate: Path = API_CRATE) -> list[tuple[Path, str]]:
    if not api_crate.exists():
        return []
    sources: list[tuple[Path, str]] = []
    for rust_file in sorted(api_crate.rglob("*.rs")):
        text = rust_file.read_text(encoding="utf-8")
        if has_test_markers(text) or "tests" in rust_file.parts:
            sources.append((rust_file, text))
    return sources


def display_path(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def check_handler_test_coverage(api_lib: Path = API_LIB, api_crate: Path = API_CRATE) -> tuple[list[Issue], dict[str, object]]:
    if not api_lib.exists():
        return [Issue("missing", f"缺少 {api_lib.relative_to(REPO_ROOT)}")], {
            "path_count": 0,
            "test_source_count": 0,
            "covered_paths": [],
            "missing_paths": [],
            "test_sources": [],
        }

    lib_text = api_lib.read_text(encoding="utf-8")
    path_count = len(UTOIPA_PATH_RE.findall(lib_text))
    declared_paths = extract_utoipa_paths(lib_text)
    test_sources = collect_test_sources(api_crate)

    covered_paths = sorted({
        path
        for path in declared_paths
        for source_path, text in test_sources
        if path in extract_test_text(source_path, text)
    })
    missing_paths = [path for path in declared_paths if path not in covered_paths]

    issues: list[Issue] = []
    if path_count > 0 and not test_sources:
        issues.append(Issue("missing_test", "api crate 存在 #[utoipa::path]，但未找到 tests/*.rs 或 cfg(test) 测试"))
    elif path_count > 0 and not covered_paths:
        issues.append(Issue("missing_path_coverage", "已找到测试源，但未发现任何 OpenAPI path 字面量覆盖"))
    elif path_count > 0 and missing_paths:
        issues.append(Issue(
            "partial_path_coverage",
            "以下 OpenAPI path 缺少测试覆盖: " + ", ".join(missing_paths),
        ))

    stats = {
        "path_count": path_count,
        "test_source_count": len(test_sources),
        "covered_paths": covered_paths,
        "missing_paths": missing_paths,
        "test_sources": [display_path(path) for path, _ in test_sources],
    }
    return issues, stats


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)

    issues, stats = check_handler_test_coverage()
    ok = not issues

    if args.json:
        print(json.dumps({
            "check": "check_handler_test_coverage",
            "tier": "T2",
            "category": "质量治理",
            **stats,
            "issues": [asdict(i) for i in issues],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_handler_test_coverage (T2, 质量治理)")
        print(
            f"  · utoipa paths={stats['path_count']},"
            f" test sources={stats['test_source_count']},"
            f" covered paths={len(stats['covered_paths'])},"
            f" missing paths={len(stats['missing_paths'])}"
        )
        if ok:
            print("  ✓ 已找到最小 OpenAPI path 测试覆盖")
        else:
            print(f"  ✘ 发现 {len(issues)} 个覆盖缺口:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.detail}")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
