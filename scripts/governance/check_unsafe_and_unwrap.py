#!/usr/bin/env python3
"""check_unsafe_and_unwrap.py — Rust unsafe / panic shortcut 禁用校验

类别：2. 代码治理
Tier：T2（< 10s）
输入：backend/crates/**/*.rs
输出：人类可读 + --json
退出码：
  0  通过
  1  发现 unsafe / unwrap / expect / panic
  2  脚本自身错误

说明：
- 生产路径禁止 `unsafe` / `.unwrap()` / `.expect()` / `panic!()`
- 测试代码允许 `.unwrap()` / `.expect()` / `panic!()`
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
BACKEND_CRATES = REPO_ROOT / "backend" / "crates"

UNSAFE_RE = re.compile(r"\bunsafe\b")
UNWRAP_RE = re.compile(r"\.unwrap\s*\(")
EXPECT_RE = re.compile(r"\.expect\s*\(")
PANIC_RE = re.compile(r"\bpanic!\s*\(")
TEST_ATTR_RE = re.compile(r"#\s*\[\s*(?:test|tokio::test|sqlx::test)\b")
CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
INCLUDE_RE = re.compile(r'include!\s*\(\s*"([^"]+)"\s*\)')


@dataclass
class Issue:
    path: str
    line: int
    kind: str
    detail: str


def strip_rust_comments_and_strings(text: str) -> str:
    out: list[str] = []
    i = 0
    n = len(text)
    state = "code"
    block_depth = 0

    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""

        if state == "code":
            if ch == "/" and nxt == "/":
                state = "line_comment"
                out.extend("  ")
                i += 2
                continue
            if ch == "/" and nxt == "*":
                state = "block_comment"
                block_depth = 1
                out.extend("  ")
                i += 2
                continue
            if ch == '"':
                state = "string"
                out.append(" ")
                i += 1
                continue
            if ch == "'":
                state = "char"
                out.append(" ")
                i += 1
                continue
            out.append(ch)
            i += 1
            continue

        if state == "line_comment":
            if ch == "\n":
                state = "code"
                out.append("\n")
            else:
                out.append(" ")
            i += 1
            continue

        if state == "block_comment":
            if ch == "/" and nxt == "*":
                block_depth += 1
                out.extend("  ")
                i += 2
                continue
            if ch == "*" and nxt == "/":
                block_depth -= 1
                out.extend("  ")
                i += 2
                if block_depth == 0:
                    state = "code"
                continue
            out.append("\n" if ch == "\n" else " ")
            i += 1
            continue

        if state == "string":
            if ch == "\\" and i + 1 < n:
                out.extend("  ")
                i += 2
                continue
            if ch == '"':
                state = "code"
            out.append("\n" if ch == "\n" else " ")
            i += 1
            continue

        if state == "char":
            if ch == "\\" and i + 1 < n:
                out.extend("  ")
                i += 2
                continue
            if ch == "'":
                state = "code"
            out.append("\n" if ch == "\n" else " ")
            i += 1
            continue

    return "".join(out)


def _is_test_file(path: str) -> bool:
    normalized = path.replace("\\", "/")
    return (
        "/tests/" in normalized
        or normalized.startswith("tests/")
        or normalized.endswith("/tests.rs")
    )


def _test_only_include_files(root: Path) -> set[Path]:
    """识别只被 `#[cfg(test)] mod` 引入的拆分测试文件。"""
    test_only: set[Path] = set()
    test_module_re = re.compile(
        r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*"
        r"mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{(?P<body>[\s\S]*?)\n\s*\}",
    )
    for source in sorted(root.rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for module in test_module_re.finditer(text):
            for include_path in INCLUDE_RE.findall(module.group("body")):
                candidate = (source.parent / include_path).resolve()
                if candidate.is_file():
                    test_only.add(candidate)
    return test_only


def test_code_lines(text: str, *, path: str, test_only: bool = False) -> set[int]:
    lines = text.splitlines()
    if test_only or _is_test_file(path):
        return set(range(1, len(lines) + 1))

    test_lines: set[int] = set()
    pending_test_attr = False
    test_block_depth: int | None = None

    for lineno, line in enumerate(lines, start=1):
        if test_block_depth is not None:
            test_lines.add(lineno)
            test_block_depth += line.count("{") - line.count("}")
            if test_block_depth <= 0:
                test_block_depth = None
            continue

        if CFG_TEST_RE.search(line) or TEST_ATTR_RE.search(line):
            test_lines.add(lineno)
            pending_test_attr = True
            continue

        if pending_test_attr:
            test_lines.add(lineno)
            if "{" in line:
                test_block_depth = line.count("{") - line.count("}")
                if test_block_depth <= 0:
                    test_block_depth = None
                pending_test_attr = False
            continue

    return test_lines


def find_unsafe_unwrap_issues(
    text: str, *, path: str, test_only: bool = False
) -> list[Issue]:
    sanitized = strip_rust_comments_and_strings(text)
    test_lines = test_code_lines(text, path=path, test_only=test_only)
    issues: list[Issue] = []
    for lineno, line in enumerate(sanitized.splitlines(), start=1):
        if not line.strip():
            continue
        if UNSAFE_RE.search(line):
            issues.append(Issue(
                path=path,
                line=lineno,
                kind="unsafe",
                detail="禁止使用 unsafe",
            ))
        if lineno in test_lines:
            continue
        if UNWRAP_RE.search(line):
            issues.append(Issue(
                path=path,
                line=lineno,
                kind="unwrap",
                detail="禁止使用 .unwrap()",
            ))
        if EXPECT_RE.search(line):
            issues.append(Issue(
                path=path,
                line=lineno,
                kind="expect",
                detail="生产路径禁止使用 .expect()",
            ))
        if PANIC_RE.search(line):
            issues.append(Issue(
                path=path,
                line=lineno,
                kind="panic",
                detail="生产路径禁止使用 panic!()",
            ))
    return issues


def scan_backend_crates(root: Path = BACKEND_CRATES) -> tuple[list[Issue], int]:
    issues: list[Issue] = []
    test_only_files = _test_only_include_files(root)
    files = sorted(root.rglob("*.rs")) if root.exists() else []
    for rust_file in files:
        issues.extend(find_unsafe_unwrap_issues(
            rust_file.read_text(encoding="utf-8"),
            path=str(rust_file.relative_to(REPO_ROOT)),
            test_only=rust_file.resolve() in test_only_files,
        ))
    return issues, len(files)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)

    issues, scanned_files = scan_backend_crates()
    ok = not issues

    if args.json:
        print(json.dumps({
            "check": "check_unsafe_and_unwrap",
            "tier": "T2",
            "category": "代码治理",
            "scanned_files": scanned_files,
            "issues": [asdict(i) for i in issues],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_unsafe_and_unwrap (T2, 代码治理)")
        print(f"  · scanned: {scanned_files} file(s)")
        if ok:
            print("  ✓ 未发现 unsafe / 生产路径 .unwrap() / .expect() / panic!()")
        else:
            print(f"  ✘ 发现 {len(issues)} 处违规:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.path}:{issue.line} {issue.detail}")
            print("  · 说明：测试代码允许 .unwrap() / .expect() / panic!()")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
