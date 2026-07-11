#!/usr/bin/env python3
"""check_audit_trail_coverage.py — Wave 3 T3：业务写路径的 audit 测试证据

类别：后端治理
Tier：T3
规则（真实写路径 + 测试证据，禁止假绿关键字扫盘）：
1. 从 OpenAPI 收集写操作（POST/PUT/PATCH/DELETE）
2. 每个写操作在 backend/crates/api/tests 中须有“路径/动作可识别”的测试函数，
   且同函数查询 audit_event 持久化证据
3. 另：HTTP 写成功测试（Request::builder + 2xx）的函数体必须含审计证据
4. 违规 id 走 baseline：禁止新增缺口

退出码：0 通过 / 1 有新增违规 / 2 脚本错误
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
SCRIPTS_DIR = _THIS.parent
REPO_ROOT = SCRIPTS_DIR.parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))

from _baseline import evaluate, format_report  # noqa: E402

CHECK_NAME = "check_audit_trail_coverage"
OPENAPI_JSON = REPO_ROOT / "shared" / "openapi" / "openapi.json"
DEFAULT_TEST_ROOT = REPO_ROOT / "backend" / "crates" / "api" / "tests"
WRITE_METHODS = {"post", "put", "patch", "delete"}
SKIP_PATH_PREFIXES = (
    "/api/v1/healthz",
    "/api/v1/auth/login",
    "/api/v1/auth/refresh",
)

REQUEST_BUILDER_RE = re.compile(
    r"Request::builder\(\)(?P<body>[\s\S]{0,1600}?)\.uri\(\s*(?:format!\(\s*)?\"(?P<uri>[^\"]+)\"",
)
REQUEST_METHOD_RE = re.compile(
    r"\.method\(\s*(?:(?:Method::)?(GET|POST|PUT|PATCH|DELETE)\b|\"(GET|POST|PUT|PATCH|DELETE)\")",
)
SUCCESS_STATUS_RE_TEMPLATE = (
    r"assert_eq!\(\s*{variable}\.status\(\)\s*,\s*"
    r"StatusCode::(?:OK|CREATED|ACCEPTED|NO_CONTENT|PARTIAL_CONTENT)\s*\)"
)
TEST_FN_RE = re.compile(r"(?:#\[[^\]]+\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
AUDIT_EVIDENCE_RE = re.compile(r"audit_event", re.IGNORECASE)


def path_to_regex(path: str) -> re.Pattern[str]:
    parts: list[str] = []
    for segment in path.split("/"):
        if not segment:
            continue
        if segment.startswith("{") and segment.endswith("}"):
            parts.append(r"[^/]+")
        else:
            parts.append(re.escape(segment))
    return re.compile(r"/" + r"/".join(parts) + r"(?:[\"'?\s]|$)")


def significant_segments(path: str) -> list[str]:
    skip = {"api", "v1"}
    out: list[str] = []
    for segment in path.split("/"):
        if not segment or segment.startswith("{") or segment in skip:
            continue
        out.append(segment.lower())
    return out


def _norm_token(value: str) -> str:
    return value.lower().replace("-", "_")


def _token_in_text(token: str, lower: str) -> bool:
    """允许复数资源名与 repository 方法名互认（receiving_orders ↔ receiving_order）。"""
    t = _norm_token(token)
    if t in lower:
        return True
    if t.endswith("s") and t[:-1] in lower:
        return True
    if f"{t}s" in lower:
        return True
    return False


def file_covers_operation(text: str, path: str, path_re: re.Pattern[str], operation_id: str = "") -> bool:
    if path in text or path_re.search(text):
        return True
    if operation_id and operation_id in text:
        return True
    lower = text.lower()
    segs = significant_segments(path)
    if not segs:
        return False
    # 动作 + 资源片段（兼容 repository 直测：receive_receiving_order）
    action = _norm_token(segs[-1])
    resource = _norm_token(segs[-2]) if len(segs) >= 2 else ""
    if action and resource and _token_in_text(action, lower) and _token_in_text(resource, lower):
        return True
    if action and f"{action}_" in lower and any(_token_in_text(s, lower) for s in segs[:-1]):
        return True
    # 至少命中末尾 3 个显著段
    need = segs[-3:] if len(segs) >= 3 else segs
    return all(_token_in_text(s, lower) for s in need)


def collect_openapi_write_ops(openapi_path: Path = OPENAPI_JSON) -> list[dict]:
    if not openapi_path.is_file():
        return []
    spec = json.loads(openapi_path.read_text(encoding="utf-8"))
    ops: list[dict] = []
    for path, item in (spec.get("paths") or {}).items():
        if not isinstance(item, dict):
            continue
        if any(path.startswith(prefix) for prefix in SKIP_PATH_PREFIXES):
            continue
        for method, operation in item.items():
            method_l = method.lower()
            if method_l not in WRITE_METHODS or not isinstance(operation, dict):
                continue
            ops.append(
                {
                    "id": f"{method.upper()} {path}",
                    "method": method.upper(),
                    "path": path,
                    "operation_id": str(operation.get("operationId") or ""),
                    "path_re": path_to_regex(path),
                }
            )
    return ops


def _request_method(match: re.Match[str], text: str, end: int) -> str:
    tail = text[match.end() : end].split(".body(", 1)[0]
    method_matches = list(REQUEST_METHOD_RE.finditer(f"{match.group('body')}{tail}"))
    if not method_matches:
        return "GET"
    return next(group for group in method_matches[-1].groups() if group)


def _enclosing_test_name(text: str, pos: int) -> str | None:
    last = None
    for match in TEST_FN_RE.finditer(text):
        if match.start() > pos:
            break
        last = match.group(1)
    return last


def _test_function_span(text: str, fn_name: str) -> tuple[int, int] | None:
    matches = list(TEST_FN_RE.finditer(text))
    for index, match in enumerate(matches):
        if match.group(1) == fn_name:
            end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
            return match.start(), end
    return None


def _function_bodies(text: str) -> list[str]:
    matches = list(TEST_FN_RE.finditer(text))
    return [
        text[match.start() : matches[index + 1].start() if index + 1 < len(matches) else len(text)]
        for index, match in enumerate(matches)
    ]


def collect_write_success_tests(test_root: Path) -> list[dict]:
    if not test_root.is_dir():
        return []
    samples: list[dict] = []
    for path in sorted(test_root.rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="ignore")
        matches = list(REQUEST_BUILDER_RE.finditer(text))
        for index, match in enumerate(matches):
            end = matches[index + 1].start() if index + 1 < len(matches) else min(len(text), match.end() + 3500)
            method = _request_method(match, text, end)
            if method not in {m.upper() for m in WRITE_METHODS}:
                continue
            prefix = text[max(0, match.start() - 500) : match.start()]
            assignments = re.findall(r"let\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=", prefix)
            if not assignments:
                continue
            variable = assignments[-1]
            success_re = re.compile(SUCCESS_STATUS_RE_TEMPLATE.format(variable=re.escape(variable)))
            if not success_re.search(text[match.end() : end]):
                continue
            test_name = _enclosing_test_name(text, match.start()) or path.stem
            span = _test_function_span(text, test_name)
            body = text[span[0] : span[1]] if span else text
            uri = match.group("uri")
            samples.append(
                {
                    "method": method,
                    "path": uri,
                    "test": test_name,
                    "file": str(path.relative_to(REPO_ROOT)) if path.is_relative_to(REPO_ROOT) else str(path),
                    "has_audit_evidence": bool(AUDIT_EVIDENCE_RE.search(body)),
                    "id": f"{method} {uri} :: {test_name}",
                }
            )
    return samples


def find_missing_audit_assertions(samples: list[dict]) -> list[dict]:
    return [
        {
            "id": sample["id"],
            "method": sample["method"],
            "path": sample["path"],
            "test": sample["test"],
            "file": sample["file"],
            "kind": "missing_audit_event_evidence",
        }
        for sample in samples
        if not sample["has_audit_evidence"]
    ]


def find_missing_openapi_audit_tests(ops: list[dict], test_root: Path) -> list[dict]:
    if not test_root.is_dir():
        return [
            {
                "id": op["id"],
                "method": op["method"],
                "path": op["path"],
                "kind": "missing_openapi_audit_test_evidence",
            }
            for op in ops
        ]
    files = [
        _function_bodies(p.read_text(encoding="utf-8", errors="ignore"))
        for p in sorted(test_root.rglob("*.rs"))
    ]
    missing: list[dict] = []
    for op in ops:
        covered = False
        for bodies in files:
            if any(
                file_covers_operation(body, op["path"], op["path_re"], op["operation_id"])
                and AUDIT_EVIDENCE_RE.search(body)
                for body in bodies
            ):
                covered = True
                break
        if not covered:
            missing.append(
                {
                    "id": op["id"],
                    "method": op["method"],
                    "path": op["path"],
                    "kind": "missing_openapi_audit_test_evidence",
                }
            )
    return missing


def run_check(
    *,
    openapi_path: Path = OPENAPI_JSON,
    test_root: Path = DEFAULT_TEST_ROOT,
    auto_shrink: bool = True,
) -> dict:
    samples = collect_write_success_tests(test_root)
    missing_http = find_missing_audit_assertions(samples)
    ops = collect_openapi_write_ops(openapi_path)
    missing_ops = find_missing_openapi_audit_tests(ops, test_root)
    # 合并违规：HTTP 级用完整 id，OpenAPI 级用 METHOD path
    missing = missing_http + missing_ops
    violation_ids = [item["id"] for item in missing]
    evaluation = evaluate(CHECK_NAME, violation_ids, auto_shrink=auto_shrink)
    ok = not evaluation.has_failure
    return {
        "check": CHECK_NAME,
        "tier": "T3",
        "category": "后端治理",
        "ok": ok,
        "write_success_tests": len(samples),
        "openapi_write_ops": len(ops),
        "missing_audit_assertions": len(missing),
        "new_violations": evaluation.new_violations,
        "still_present": evaluation.still_present,
        "resolved": evaluation.resolved,
        "expired": evaluation.expired,
        "samples_missing": missing[:30],
        "message": (
            "写路径 audit 测试证据覆盖通过（无新增缺口）"
            if ok
            else f"发现 {len(evaluation.new_violations)} 个新增审计证据缺口"
        ),
        "baseline_report": format_report(CHECK_NAME, evaluation),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Wave 3 T3：写路径 audit 测试证据门禁")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-shrink", action="store_true")
    parser.add_argument("--openapi", type=Path, default=OPENAPI_JSON)
    parser.add_argument("--test-root", type=Path, default=DEFAULT_TEST_ROOT)
    args = parser.parse_args()
    try:
        result = run_check(
            openapi_path=args.openapi,
            test_root=args.test_root,
            auto_shrink=not args.no_shrink,
        )
    except Exception as exc:  # noqa: BLE001
        if args.json:
            print(
                json.dumps(
                    {
                        "check": CHECK_NAME,
                        "tier": "T3",
                        "category": "后端治理",
                        "ok": False,
                        "error": str(exc),
                    },
                    ensure_ascii=False,
                )
            )
        else:
            print(f"check_audit_trail_coverage 脚本错误: {exc}")
        return 2

    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print("check_audit_trail_coverage (T3, 后端治理)")
        print(f"  · OpenAPI 写操作: {result['openapi_write_ops']}")
        print(f"  · HTTP 写成功测试: {result['write_success_tests']}")
        print(f"  · 缺 audit 证据: {result['missing_audit_assertions']}")
        print(f"  · 新增违规: {len(result['new_violations'])}")
        print(f"  · baseline 仍抑制: {len(result['still_present'])}")
        print(f"  {'✓' if result['ok'] else '✘'} {result['message']}")
        for item in result.get("samples_missing", [])[:12]:
            mark = "+" if item["id"] in result["new_violations"] else "·"
            print(f"    {mark} {item['id']}")
        print(result.get("baseline_report") or "")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
