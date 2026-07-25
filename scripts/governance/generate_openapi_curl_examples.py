#!/usr/bin/env python3
"""从 OpenAPI 生成每个 operation 的 curl 示例。"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
OPENAPI = ROOT / "shared" / "openapi" / "openapi.json"
OUT = ROOT / "docs" / "api" / "curl-examples.md"


def operation_sort_key(item: tuple[str, str, dict]) -> tuple[str, str]:
    path, method, _operation = item
    return path, method


def path_example(path: str) -> str:
    return re.sub(r"\{([^}]+)\}", r"<\1>", path)


def operation_requires_auth(operation: dict) -> bool:
    return operation.get("security") != []


def operation_security_schemes(operation: dict) -> set[str]:
    return {
        scheme
        for requirement in operation.get("security", [])
        if isinstance(requirement, dict)
        for scheme in requirement
    }


def header_example(name: str) -> str:
    value = {
        "Idempotency-Key": "$IDEMPOTENCY_KEY",
        "X-WMS-API-Key": "$WMS_API_KEY",
        "X-WMS-Warehouse-ID": "$WMS_WAREHOUSE_ID",
    }.get(name, f"<{name}>")
    return f'"{name}: {value}"'


def curl_for(method: str, path: str, operation: dict) -> list[str]:
    url = f'"$WMS_API_BASE{path_example(path)}"'
    parts = ["curl -sS", f"-X {method.upper()}", url]
    added_headers: set[str] = set()
    if "ColdChainApiKeyAuth" in operation_security_schemes(operation):
        parts.extend(["-H", header_example("X-WMS-API-Key")])
        added_headers.add("X-WMS-API-Key")
    elif operation_requires_auth(operation):
        parts.extend(["-H", '"Authorization: Bearer $WMS_TOKEN"'])
        added_headers.add("Authorization")
    for parameter in operation.get("parameters", []):
        if (
            isinstance(parameter, dict)
            and parameter.get("in") == "header"
            and parameter.get("required") is True
            and isinstance(parameter.get("name"), str)
            and parameter["name"] not in added_headers
        ):
            parts.extend(["-H", header_example(parameter["name"])])
            added_headers.add(parameter["name"])
    if method.lower() in {"post", "put", "patch"}:
        parts.extend(["-H", '"Content-Type: application/json"', "-d", "'{}'"])
    return parts


def build_doc() -> str:
    payload = json.loads(OPENAPI.read_text(encoding="utf-8"))
    operations: list[tuple[str, str, dict]] = []
    for path, methods in payload.get("paths", {}).items():
        if not isinstance(methods, dict):
            continue
        for method, operation in methods.items():
            if isinstance(operation, dict):
                operations.append((path, method, operation))

    lines = [
        "# OpenAPI curl 示例",
        "",
        "> 本文件由 `scripts/governance/generate_openapi_curl_examples.py` 根据 `shared/openapi/openapi.json` 生成；不要手工编辑。",
        "",
        "使用前设置：",
        "",
        "```bash",
        "export WMS_API_BASE=http://127.0.0.1:9002",
        "export WMS_TOKEN=<从 /api/v1/auth/login 获取的 access_token>",
        "export WMS_API_KEY=<H1 外部 API Key>",
        "export IDEMPOTENCY_KEY=<本次业务请求的稳定幂等键>",
        "export WMS_WAREHOUSE_ID=<调用主体授权仓库 ID>",
        "```",
        "",
    ]
    for path, method, operation in sorted(operations, key=operation_sort_key):
        lines.extend(
            [
                f"## {method.upper()} {path}",
                "",
                "```bash",
                " \\\n  ".join(curl_for(method, path, operation)),
                "```",
                "",
            ]
        )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = build_doc()
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != expected:
            print(f"{OUT.relative_to(ROOT)} 与 OpenAPI 不同步；运行本脚本刷新", file=sys.stderr)
            return 1
        print("generate_openapi_curl_examples: OK")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(expected, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
