#!/usr/bin/env python3
"""check_layer_dependency.py — 前后端分层依赖校验

类别：2. 代码治理
Tier：T2（< 10s）
输入：backend/crates/domain/src + backend/crates/api/src + apps/web-admin/src + apps/customer-portal/src
      + packages/*/src
输出：人类可读 + --json
退出码：
  0  通过
  1  发现违规依赖
  2  脚本自身错误

规则（Wave 1 最小版）：
- domain 层不得引用 api / infra / axum / sqlx
- api service/repository 层不得反向引用 runtime `auth::AuthContext`
- 前端 feature/lib 不得反向引用 page/app-shell；page 只能以类型方式引用 app-shell/API client
- `packages/api-client` 与 `packages/ui` 不得依赖应用层或彼此的传输层
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
WEB_ADMIN_SRC = REPO_ROOT / "apps" / "web-admin" / "src"
CUSTOMER_PORTAL_SRC = REPO_ROOT / "apps" / "customer-portal" / "src"
API_CLIENT_SRC = REPO_ROOT / "packages" / "api-client" / "src"
UI_SRC = REPO_ROOT / "packages" / "ui" / "src"
FRONTEND_SOURCE_ROOTS = (WEB_ADMIN_SRC, CUSTOMER_PORTAL_SRC, API_CLIENT_SRC, UI_SRC)

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


_FRONTEND_STATIC_IMPORT = re.compile(
    r"^\s*(?P<keyword>import|export)\b(?P<body>.*?\bfrom\s*)?['\"](?P<source>[^'\"]+)['\"]"
)
_FRONTEND_DYNAMIC_IMPORT = re.compile(
    r"\bimport\s*\(\s*['\"](?P<source>[^'\"]+)['\"]\s*\)"
)


def _repo_relative_path(path: Path | str) -> str:
    candidate = Path(path)
    if not candidate.is_absolute():
        candidate = REPO_ROOT / candidate
    try:
        return candidate.resolve().relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return candidate.as_posix()


def frontend_layer_for_path(path: Path | str) -> str | None:
    """返回前端文件所在层；未知路径不参与方向判定。"""
    normalized = _repo_relative_path(path)
    if normalized == "apps/web-admin/src/App.tsx" or normalized == "apps/web-admin/src/main.tsx":
        return "app-shell"
    if normalized.startswith("apps/web-admin/src/app-shell/"):
        return "app-shell"
    if normalized.startswith("apps/web-admin/src/pages/"):
        return "page"
    if normalized.startswith("apps/web-admin/src/features/"):
        return "feature"
    if normalized.startswith("apps/web-admin/src/lib/"):
        return "lib"
    if normalized in {
        "apps/customer-portal/src/App.tsx",
        "apps/customer-portal/src/main.tsx",
        "apps/customer-portal/src/App",
        "apps/customer-portal/src/main",
    }:
        return "app-shell"
    if normalized in {
        "apps/customer-portal/src/api.ts",
        "apps/customer-portal/src/api",
        "apps/customer-portal/src/schema.ts",
        "apps/customer-portal/src/schema",
        "apps/customer-portal/src/types.ts",
        "apps/customer-portal/src/types",
    }:
        return "api-client"
    if normalized.startswith("apps/customer-portal/src/"):
        return "page"
    if normalized == "packages/api-client/src" or normalized.startswith("packages/api-client/src/"):
        return "api-client"
    if normalized == "packages/ui/src" or normalized.startswith("packages/ui/src/"):
        return "ui"
    return None


def _frontend_import_target(path: Path | str, source: str) -> Path | None:
    if source.startswith("@/"):
        return WEB_ADMIN_SRC / source[2:]
    if source == "@wms/api-client" or source.startswith("@wms/api-client/"):
        return API_CLIENT_SRC / source.removeprefix("@wms/api-client").lstrip("/")
    if source == "@wms/ui" or source.startswith("@wms/ui/"):
        return UI_SRC / source.removeprefix("@wms/ui").lstrip("/")
    if source.startswith("."):
        current = Path(path)
        if not current.is_absolute():
            current = REPO_ROOT / current
        return current.parent / source
    return None


def _frontend_imports(text: str) -> list[tuple[int, str, bool]]:
    imports: list[tuple[int, str, bool]] = []
    for lineno, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.split("//", 1)[0]
        match = _FRONTEND_STATIC_IMPORT.match(line)
        if match:
            body = match.group("body") or ""
            imports.append((lineno, match.group("source"), body.strip().startswith("type ")))
            continue
        match = _FRONTEND_DYNAMIC_IMPORT.search(line)
        if match:
            imports.append((lineno, match.group("source"), False))
    return imports


def find_frontend_dependency_issues(text: str, *, path: str) -> list[Issue]:
    """检查前端层只能向下依赖；类型导入例外保持显式且最小。"""
    current_layer = frontend_layer_for_path(path)
    if current_layer is None:
        return []

    forbidden: dict[str, set[str]] = {
        "feature": {"page", "app-shell"},
        "lib": {"page", "app-shell", "feature"},
        "page": {"app-shell", "api-client"},
        "api-client": {"app-shell", "page", "feature", "lib", "ui"},
        "ui": {"app-shell", "page", "feature", "api-client"},
    }
    issues: list[Issue] = []
    customer_portal_page = _repo_relative_path(path).startswith("apps/customer-portal/src/")
    for lineno, source, type_only in _frontend_imports(text):
        target = _frontend_import_target(path, source)
        target_layer = frontend_layer_for_path(target) if target else None
        if target_layer not in forbidden.get(current_layer, set()):
            continue
        if type_only and current_layer == "page" and target_layer in {"app-shell", "api-client"}:
            continue
        if customer_portal_page and current_layer == "page" and target_layer == "api-client":
            continue
        issues.append(Issue(
            path=path,
            line=lineno,
            kind="frontend_dependency_direction",
            detail=(
                f"前端 {current_layer} 层不得依赖 {target_layer} 层: "
                f"{source}{'（type-only 例外未命中）' if type_only else ''}"
            ),
        ))
    return issues


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


def scan_layer_dependencies(
    domain_src: Path = DOMAIN_SRC,
    api_src: Path = API_SRC,
    frontend_roots: tuple[Path, ...] = FRONTEND_SOURCE_ROOTS,
) -> tuple[list[Issue], dict[str, int]]:
    issues: list[Issue] = []
    stats = {
        "domain_files": 0,
        "api_files": 0,
        "frontend_files": 0,
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

    for frontend_root in frontend_roots:
        for source_file in sorted(frontend_root.rglob("*.ts")) + sorted(frontend_root.rglob("*.tsx")):
            stats["frontend_files"] += 1
            issues.extend(find_frontend_dependency_issues(
                source_file.read_text(encoding="utf-8"),
                path=_repo_relative_path(source_file),
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
            f" api={stats['api_files']} file(s),"
            f" frontend={stats['frontend_files']} file(s)"
        )
        if ok:
            print("  ✓ domain 层未发现 api / infra / axum / sqlx 引用")
            print("  ✓ api service/repository 层未发现 runtime auth 上下文反向依赖")
            print("  ✓ 前端 app shell → page → feature → api-client 方向未发现反向依赖")
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
