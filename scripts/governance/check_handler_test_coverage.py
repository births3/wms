#!/usr/bin/env python3
"""check_handler_test_coverage.py — API handler 测试覆盖起步校验

类别：3. 质量治理
Tier：T2（< 10s）
输入：backend/crates/api/src/**/*.rs + backend/crates/api/**/*.rs
输出：人类可读 + --json
退出码：
  0  通过
  1  缺少最小测试覆盖
  2  脚本自身错误

Wave 1 baseline 最小规则：
- 统计 `#[utoipa::path]` 数量
- 普通模式检查测试源是否逐一提及 OpenAPI path，作为迁移期基线
- `--strict` 只认可与具体请求变量关联的 2xx 状态断言
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
UTOIPA_OPERATION_RE = re.compile(
    r"#\s*\[\s*utoipa::path\s*\(\s*(get|post|put|patch|delete)\b[\s\S]*?\bpath\s*=\s*\"([^\"]+)\"[\s\S]*?\)\s*\]",
    re.IGNORECASE,
)
REQUEST_BUILDER_RE = re.compile(
    r'Request::builder\(\)(?P<body>[\s\S]{0,1200}?)\.uri\(\s*(?:format!\(\s*)?"(?P<uri>[^"]+)"',
)
REQUEST_METHOD_RE = re.compile(
    r'\.method\(\s*(?:(?:Method::)?(GET|POST|PUT|PATCH|DELETE)\b|"(GET|POST|PUT|PATCH|DELETE)")'
)
TEST_PATH_LITERAL_RE = re.compile(
    r'"((?:/api/v1|/openapi\.json|/api-docs|/redoc|/metrics)[^"\n]*)"'
)
SUCCESS_STATUS_RE_TEMPLATE = (
    r"assert_eq!\(\s*{variable}\.status\(\)\s*,\s*"
    r"StatusCode::(?:OK|CREATED|ACCEPTED|NO_CONTENT|PARTIAL_CONTENT)\s*\)"
)
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


def extract_utoipa_operations(text: str) -> list[tuple[str, str]]:
    return list(dict.fromkeys((method.upper(), path) for method, path in UTOIPA_OPERATION_RE.findall(text)))


def extract_http_requests(text: str) -> list[tuple[str, str]]:
    requests: list[tuple[str, str]] = []
    matches = list(REQUEST_BUILDER_RE.finditer(text))
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        requests.append((_request_method(match, text, end), match.group("uri")))
    return requests


def extract_test_path_literals(text: str) -> list[str]:
    """提取测试辅助函数和 format! 中的路由模板。

    不要求测试必须直接调用 `Request::builder()`；项目中的 `request_json`
    等测试 helper 同样是真实 HTTP 路由测试。
    """
    return list(dict.fromkeys(TEST_PATH_LITERAL_RE.findall(text)))


def _request_method(match: re.Match[str], text: str, end: int) -> str:
    tail = text[match.end():end].split(".body(", 1)[0]
    method_matches = list(REQUEST_METHOD_RE.finditer(f"{match.group('body')}{tail}"))
    if not method_matches:
        return "GET"
    return next(group for group in method_matches[-1].groups() if group)


def extract_successful_http_requests(text: str) -> list[tuple[str, str]]:
    requests: list[tuple[str, str]] = []
    matches = list(REQUEST_BUILDER_RE.finditer(text))
    for index, match in enumerate(matches):
        prefix = text[max(0, match.start() - 500):match.start()]
        assignments = re.findall(r"let\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=", prefix)
        if not assignments:
            continue
        variable = assignments[-1]
        end = matches[index + 1].start() if index + 1 < len(matches) else min(len(text), match.end() + 3000)
        assertion_text = text[match.end():end]
        success_re = re.compile(SUCCESS_STATUS_RE_TEMPLATE.format(variable=re.escape(variable)))
        if not success_re.search(assertion_text):
            continue
        requests.append((_request_method(match, text, end), match.group("uri")))
    return requests


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


def _route_matches(declared_path: str, request_uri: str) -> bool:
    request_path = request_uri.split("?", 1)[0]
    declared_parts = declared_path.strip("/").split("/")
    request_parts = request_path.strip("/").split("/")
    return len(declared_parts) == len(request_parts) and all(
        declared.startswith("{") and declared.endswith("}") or declared == actual
        for declared, actual in zip(declared_parts, request_parts)
    )


def check_handler_test_coverage(
    api_lib: Path = API_LIB,
    api_crate: Path = API_CRATE,
    *,
    strict: bool = False,
) -> tuple[list[Issue], dict[str, object]]:
    if not api_lib.exists():
        return [Issue("missing", f"缺少 {api_lib.relative_to(REPO_ROOT)}")], {
            "path_count": 0,
            "test_source_count": 0,
            "covered_paths": [],
            "missing_paths": [],
            "exercised_paths": [],
            "unexercised_paths": [],
            "declared_operations": [],
            "exercised_operations": [],
            "unexercised_operations": [],
            "test_sources": [],
        }

    source_texts = [
        path.read_text(encoding="utf-8")
        for path in sorted(api_lib.parent.rglob("*.rs"))
    ]
    declaration_texts = [text for text in source_texts if UTOIPA_PATH_RE.search(text)]
    path_count = sum(len(UTOIPA_PATH_RE.findall(text)) for text in declaration_texts)
    declared_paths = extract_utoipa_paths("\n".join(declaration_texts))
    declared_operations = extract_utoipa_operations("\n".join(declaration_texts))
    test_sources = collect_test_sources(api_crate)
    test_path_literals = [
        literal
        for source_path, text in test_sources
        for literal in extract_test_path_literals(extract_test_text(source_path, text))
    ]
    covered_paths = sorted({
        path
        for path in declared_paths
        if any(_route_matches(path, literal) for literal in test_path_literals)
    })
    missing_paths = [path for path in declared_paths if path not in covered_paths]
    exercised_operations = sorted({
        (method, path)
        for method, path in declared_operations
        for source_path, text in test_sources
        for request_method, request_uri in extract_successful_http_requests(
            extract_test_text(source_path, text)
        )
        if method == request_method and _route_matches(path, request_uri)
    })
    unexercised_operations = [operation for operation in declared_operations if operation not in exercised_operations]

    issues: list[Issue] = []
    if path_count == 0:
        issues.append(Issue("missing_path_declarations", "api crate 未扫描到任何 #[utoipa::path]，检查路径可能已失效"))
    elif not test_sources:
        issues.append(Issue("missing_test", "api crate 存在 #[utoipa::path]，但未找到 tests/*.rs 或 cfg(test) 测试"))
    elif not covered_paths:
        issues.append(Issue("missing_path_coverage", "已找到测试源，但未发现任何 OpenAPI path 字面量覆盖"))
    elif missing_paths:
        issues.append(Issue(
            "partial_path_coverage",
            "以下 OpenAPI path 缺少测试覆盖: " + ", ".join(missing_paths),
        ))
    if strict and unexercised_operations:
        issues.append(Issue(
            "missing_http_exercise",
            "以下 OpenAPI operation 缺少真实 HTTP 请求与状态断言: "
            + ", ".join(f"{method} {path}" for method, path in unexercised_operations),
        ))

    stats = {
        "path_count": path_count,
        "test_source_count": len(test_sources),
        "covered_paths": covered_paths,
        "missing_paths": missing_paths,
        "exercised_paths": sorted({path for _, path in exercised_operations}),
        "unexercised_paths": sorted({path for _, path in unexercised_operations}),
        "declared_operations": [f"{method} {path}" for method, path in declared_operations],
        "exercised_operations": [f"{method} {path}" for method, path in exercised_operations],
        "unexercised_operations": [f"{method} {path}" for method, path in unexercised_operations],
        "test_sources": [display_path(path) for path, _ in test_sources],
    }
    return issues, stats


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="要求每个 path 有真实 HTTP 请求与状态断言")
    args = parser.parse_args(argv)

    issues, stats = check_handler_test_coverage(strict=args.strict)
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
            f" missing paths={len(stats['missing_paths'])},"
            f" exercised paths={len(stats['exercised_paths'])}"
        )
        if ok:
            mode = "HTTP 执行" if args.strict else "测试源提及"
            print(f"  ✓ 已找到最小 OpenAPI path {mode}覆盖")
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
