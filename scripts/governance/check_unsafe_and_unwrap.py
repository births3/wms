#!/usr/bin/env python3
"""check_unsafe_and_unwrap.py — Rust unsafe / unwrap 禁用校验

类别：2. 代码治理
Tier：T2（< 10s）
输入：backend/crates/**/*.rs
输出：人类可读 + --json
退出码：
  0  通过
  1  发现 unsafe 或 unwrap
  2  脚本自身错误

说明：
- 仅检查 `unsafe` 与 `.unwrap()`
- `expect(...)` 目前允许
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


def find_unsafe_unwrap_issues(text: str, *, path: str) -> list[Issue]:
    sanitized = strip_rust_comments_and_strings(text)
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
        if UNWRAP_RE.search(line):
            issues.append(Issue(
                path=path,
                line=lineno,
                kind="unwrap",
                detail="禁止使用 .unwrap()",
            ))
    return issues


def scan_backend_crates(root: Path = BACKEND_CRATES) -> tuple[list[Issue], int]:
    issues: list[Issue] = []
    files = sorted(root.rglob("*.rs")) if root.exists() else []
    for rust_file in files:
        issues.extend(find_unsafe_unwrap_issues(
            rust_file.read_text(encoding="utf-8"),
            path=str(rust_file.relative_to(REPO_ROOT)),
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
            print("  ✓ 未发现 unsafe 或 .unwrap()")
        else:
            print(f"  ✘ 发现 {len(issues)} 处违规:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.path}:{issue.line} {issue.detail}")
            print("  · 说明：本脚本仅检查 unsafe 与 .unwrap()，不限制 expect(...)")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
