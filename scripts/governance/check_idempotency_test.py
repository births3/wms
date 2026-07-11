#!/usr/bin/env python3
"""check_idempotency_test.py — Wave 3 T3：Idempotency-Key 写路径测试证据

类别：后端治理
Tier：T3
规则（真实契约 + 测试证据，禁止假绿）：
1. 从 OpenAPI 收集声明 Idempotency-Key 的写操作（x-idempotency-exempt-reason 豁免）
2. backend/crates/api/tests 中须有可识别该操作的测试函数，
   且同函数含幂等记录、冲突或重放断言
3. 支持 HTTP 路径与 repository 直测命名（如 receive_receiving_order）
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

CHECK_NAME = "check_idempotency_test"
OPENAPI_JSON = REPO_ROOT / "shared" / "openapi" / "openapi.json"
DEFAULT_TEST_ROOT = REPO_ROOT / "backend" / "crates" / "api" / "tests"
WRITE_METHODS = {"post", "put", "patch", "delete"}
IDEMPOTENCY_HEADER = "Idempotency-Key"
IDEMPOTENCY_EXEMPT_REASON = "x-idempotency-exempt-reason"
TEST_FN_RE = re.compile(r"(?:#\[[^\]]+\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
ACTION_SEGMENTS = {
    "ack",
    "cancel",
    "complete",
    "confirm",
    "disable",
    "inspect",
    "nack",
    "publish",
    "putaway",
    "receive",
    "reject",
    "resend",
    "review",
    "rollback",
    "ship",
    "sign",
    "test",
}
ACTION_ALIASES = {"receive": ("receive", "receiving")}


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
    action = _norm_token(segs[-1])
    resource = _norm_token(segs[-2]) if len(segs) >= 2 else ""
    if action and resource and _token_in_text(action, lower) and _token_in_text(resource, lower):
        return True
    if action and f"{action}_" in lower and any(_token_in_text(s, lower) for s in segs[:-1]):
        return True
    need = segs[-3:] if len(segs) >= 3 else segs
    return all(_token_in_text(s, lower) for s in need)


def _has_header_parameter(operation: dict, header_name: str) -> bool:
    for param in operation.get("parameters") or []:
        if not isinstance(param, dict):
            continue
        if param.get("in") == "header" and str(param.get("name", "")).lower() == header_name.lower():
            return True
    return header_name.lower() in json.dumps(operation).lower()


def _function_bodies(text: str) -> list[str]:
    matches = list(TEST_FN_RE.finditer(text))
    return [
        text[match.start() : matches[index + 1].start() if index + 1 < len(matches) else len(text)]
        for index, match in enumerate(matches)
    ]


def _has_idempotency_evidence(body: str, operation: dict) -> bool:
    lower = body.lower()
    action = operation["path"].rstrip("/").rsplit("/", 1)[-1].lower()
    function_name = TEST_FN_RE.search(body)
    action_names = ACTION_ALIASES.get(action, (action,))
    if action in ACTION_SEGMENTS and (
        function_name is None
        or not any(name in function_name.group(1).lower() for name in action_names)
    ):
        return False
    if "idempotency_request" in lower or "idempotencyconflict" in lower:
        return True
    return "replay" in lower and bool(re.search(r"assert_(?:eq|ne|matches)!|\.expect\(", body))


def collect_idempotency_required_ops(openapi_path: Path = OPENAPI_JSON) -> list[dict]:
    if not openapi_path.is_file():
        return []
    spec = json.loads(openapi_path.read_text(encoding="utf-8"))
    required: list[dict] = []
    for path, item in (spec.get("paths") or {}).items():
        if not isinstance(item, dict):
            continue
        for method, operation in item.items():
            method_l = method.lower()
            if method_l not in WRITE_METHODS or not isinstance(operation, dict):
                continue
            if operation.get(IDEMPOTENCY_EXEMPT_REASON):
                continue
            if not _has_header_parameter(operation, IDEMPOTENCY_HEADER):
                continue
            required.append(
                {
                    "id": f"{method.upper()} {path}",
                    "method": method.upper(),
                    "path": path,
                    "operation_id": str(operation.get("operationId") or ""),
                    "path_re": path_to_regex(path),
                }
            )
    return required


def find_missing_idempotency_tests(required_ops: list[dict], test_root: Path) -> list[dict]:
    if not test_root.is_dir():
        return [
            {
                "id": op["id"],
                "method": op["method"],
                "path": op["path"],
                "operation_id": op["operation_id"],
                "kind": "missing_idempotency_test_evidence",
            }
            for op in required_ops
        ]
    files = [
        _function_bodies(p.read_text(encoding="utf-8", errors="ignore"))
        for p in sorted(test_root.rglob("*.rs"))
    ]
    missing: list[dict] = []
    for op in required_ops:
        evidence_hit = False
        for bodies in files:
            if any(
                file_covers_operation(body, op["path"], op["path_re"], op["operation_id"])
                and _has_idempotency_evidence(body, op)
                for body in bodies
            ):
                evidence_hit = True
                break
        if not evidence_hit:
            missing.append(
                {
                    "id": op["id"],
                    "method": op["method"],
                    "path": op["path"],
                    "operation_id": op["operation_id"],
                    "kind": "missing_idempotency_test_evidence",
                }
            )
    return missing


def run_check(
    *,
    openapi_path: Path = OPENAPI_JSON,
    test_root: Path = DEFAULT_TEST_ROOT,
    auto_shrink: bool = True,
) -> dict:
    required = collect_idempotency_required_ops(openapi_path)
    missing = find_missing_idempotency_tests(required, test_root)
    violation_ids = [item["id"] for item in missing]
    evaluation = evaluate(CHECK_NAME, violation_ids, auto_shrink=auto_shrink)
    ok = not evaluation.has_failure
    return {
        "check": CHECK_NAME,
        "tier": "T3",
        "category": "后端治理",
        "ok": ok,
        "required_ops": len(required),
        "missing_tests": len(missing),
        "new_violations": evaluation.new_violations,
        "still_present": evaluation.still_present,
        "resolved": evaluation.resolved,
        "expired": evaluation.expired,
        "samples_missing": missing[:30],
        "message": (
            "Idempotency-Key 写路径测试证据覆盖通过（无新增缺口）"
            if ok
            else f"发现 {len(evaluation.new_violations)} 个新增幂等测试缺口"
        ),
        "baseline_report": format_report(CHECK_NAME, evaluation),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Wave 3 T3：幂等写路径测试证据门禁")
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
            print(f"check_idempotency_test 脚本错误: {exc}")
        return 2

    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print("check_idempotency_test (T3, 后端治理)")
        print(f"  · 需幂等写操作: {result['required_ops']}")
        print(f"  · 缺测试证据: {result['missing_tests']}")
        print(f"  · 新增违规: {len(result['new_violations'])}")
        print(f"  · baseline 仍抑制: {len(result['still_present'])}")
        print(f"  {'✓' if result['ok'] else '✘'} {result['message']}")
        for item in result.get("samples_missing", [])[:15]:
            mark = "+" if item["id"] in result["new_violations"] else "·"
            print(f"    {mark} {item['id']}")
        print(result.get("baseline_report") or "")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
