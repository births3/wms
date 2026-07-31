#!/usr/bin/env python3
"""check_layer_dependency.py — Rust 分层依赖校验

类别：2. 代码治理
Tier：T2（< 10s）
输入：backend/crates/domain/src + backend/crates/api/src
输出：人类可读 + --json
退出码：
  0  通过
  1  发现违规依赖
  2  脚本自身错误

规则（Wave 1 最小版）：
- domain 层不得引用 api / infra / axum / sqlx
- api service/repository 层不得反向引用 runtime `auth::AuthContext`
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
DOMAIN_SRC = REPO_ROOT / "backend" / "crates" / "domain" / "src"
API_SRC = REPO_ROOT / "backend" / "crates" / "api" / "src"

FORBIDDEN_DOMAIN_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("api", re.compile(r"\bwms_api\b")),
    ("api", re.compile(r"\b(?:crate|self|super)::api\b")),
    ("infra", re.compile(r"\b(?:crate|self|super)::infra\b")),
    ("infra", re.compile(r"\binfra::")),
    ("axum", re.compile(r"\baxum::")),
    ("sqlx", re.compile(r"\bsqlx::")),
)

SERVICE_REPOSITORY_FILE_MARKERS = (
    "_service.rs",
    "_repository.rs",
    "_repository_",
    "/repository/",
    "persistence.rs",
    "idempotency.rs",
    "db.rs",
    "models.rs",
    "workflow.rs",
    "report.rs",
    "report_audit.rs",
    "report_helpers.rs",
    "stamp.rs",
    "upstream_delivery.rs",
    "delivery.rs",
    "print_data.rs",
    "support.rs",
    "sites.rs",
    "leases.rs",
    "printers.rs",
    "trace.rs",
    "inventory_count.rs",
)

FORBIDDEN_SERVICE_REPOSITORY_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    (
        "runtime_auth_context",
        re.compile(
            r"(?:crate|self|super)::auth::AuthContext"
            r"|(?:crate|self|super)::auth::\{[^}]*\bAuthContext\b"
            r"|\bauth::AuthContext"
            r"|\bauth::\{[^}]*\bAuthContext\b"
        ),
    ),
)


@dataclass
class Issue:
    path: str
    line: int
    kind: str
    detail: str


def iter_rust_files(root: Path) -> list[Path]:
    if not root.exists():
        return []
    return sorted(root.rglob("*.rs"))


def find_domain_dependency_issues(text: str, *, path: str) -> list[Issue]:
    issues: list[Issue] = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        code = line.split("//", 1)[0]
        if not code.strip():
            continue
        for kind, pattern in FORBIDDEN_DOMAIN_PATTERNS:
            if pattern.search(code):
                issues.append(Issue(
                    path=path,
                    line=lineno,
                    kind=kind,
                    detail=f"domain 层不得引用 {kind}: {code.strip()}",
                ))
                break
    return issues


def is_service_repository_file(path: str) -> bool:
    normalized = path.replace("\\", "/")
    if "/bin/" in normalized or "handlers" in Path(normalized).name:
        return False
    return any(marker in normalized for marker in SERVICE_REPOSITORY_FILE_MARKERS)


def find_service_repository_dependency_issues(text: str, *, path: str) -> list[Issue]:
    """检查 service/repository 是否直接依赖 runtime auth 上下文。"""
    if not is_service_repository_file(path):
        return []

    issues: list[Issue] = []
    code = "\n".join(line.split("//", 1)[0] for line in text.splitlines())
    for kind, pattern in FORBIDDEN_SERVICE_REPOSITORY_PATTERNS:
        for match in pattern.finditer(code):
            lineno = code.count("\n", 0, match.start()) + 1
            line = code.splitlines()[lineno - 1].strip()
            issues.append(Issue(
                path=path,
                line=lineno,
                kind=kind,
                detail=f"service/repository 层不得引用 runtime auth 上下文: {line}",
            ))
    return issues


def scan_layer_dependencies(domain_src: Path = DOMAIN_SRC, api_src: Path = API_SRC) -> tuple[list[Issue], dict[str, int]]:
    issues: list[Issue] = []
    stats = {
        "domain_files": 0,
        "api_files": 0,
    }

    for rust_file in iter_rust_files(domain_src):
        stats["domain_files"] += 1
        issues.extend(find_domain_dependency_issues(
            rust_file.read_text(encoding="utf-8"),
            path=str(rust_file.relative_to(REPO_ROOT)),
        ))

    for rust_file in iter_rust_files(api_src):
        stats["api_files"] += 1
        text = rust_file.read_text(encoding="utf-8")
        issues.extend(find_service_repository_dependency_issues(
            text,
            path=str(rust_file.relative_to(REPO_ROOT)),
        ))

    return issues, stats


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)

    issues, stats = scan_layer_dependencies()
    ok = not issues

    if args.json:
        print(json.dumps({
            "check": "check_layer_dependency",
            "tier": "T2",
            "category": "代码治理",
            "scanned": stats,
            "issues": [asdict(i) for i in issues],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_layer_dependency (T2, 代码治理)")
        print(
            "  · scanned:"
            f" domain={stats['domain_files']} file(s),"
            f" api={stats['api_files']} file(s)"
        )
        if ok:
            print("  ✓ domain 层未发现 api / infra / axum / sqlx 引用")
            print("  ✓ api service/repository 层未发现 runtime auth 上下文反向依赖")
        else:
            print(f"  ✘ 发现 {len(issues)} 处分层违规:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.path}:{issue.line} {issue.detail}")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
