#!/usr/bin/env python3
"""check_runtime_route_mounts.py — OpenAPI 到运行时路由挂载检查

类别：5. 接口契约治理
Tier：T1（< 10s，纯静态扫描）
输入：shared/openapi/openapi.json + backend/crates/api/src/bin/wms_api.rs
输出：人类可读 + --json
退出码：
  0  已声明的核心运行时路由族均挂载
  1  OpenAPI 已声明但运行时未挂载
  2  脚本自身错误
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
OPENAPI_JSON = REPO_ROOT / "shared" / "openapi" / "openapi.json"
WMS_API_RS = REPO_ROOT / "backend" / "crates" / "api" / "src" / "bin" / "wms_api.rs"
API_LIB_RS = REPO_ROOT / "backend" / "crates" / "api" / "src" / "lib.rs"
API_SRC = REPO_ROOT / "backend" / "crates" / "api" / "src"
HTTP_METHODS = {"get", "post", "put", "patch", "delete"}


@dataclass(frozen=True)
class RouteMountSpec:
    name: str
    openapi_path: str
    tokens: tuple[str, ...]
    files: tuple[Path, ...]


@dataclass
class Issue:
    file: str
    message: str


ROUTE_MOUNT_SPECS = (
    RouteMountSpec(
        name="M1 基础档案",
        openapi_path="/api/v1/master-data/products",
        tokens=("master_data_router(", "MasterDataAppState", "master_data_handlers"),
        files=(REPO_ROOT / "backend" / "crates" / "api" / "src" / "master_data_handlers.rs",),
    ),
    RouteMountSpec(
        name="M1 系统字典",
        openapi_path="/api/v1/system-dictionaries/{dict_code}/items",
        tokens=("system_dictionary_router(", "SystemDictionaryAppState", "system_dictionary_handlers"),
        files=(REPO_ROOT / "backend" / "crates" / "api" / "src" / "system_dictionary_handlers.rs",),
    ),
)

STRICT_ROUTE_MOUNT_SPECS = (
    RouteMountSpec(
        name="M6 报表",
        openapi_path="/api/v1/reports/query",
        tokens=("reports_router(", "ReportsAppState", "reports_handlers"),
        files=(REPO_ROOT / "backend" / "crates" / "api" / "src" / "reports_handlers.rs",),
    ),
    RouteMountSpec(
        name="M-PM 参数对照",
        openapi_path="/api/v1/parameter-mapping/execute",
        tokens=("parameter_mapping_router(", "ParameterMappingAppState", "parameter_mapping_handlers"),
        files=(REPO_ROOT / "backend" / "crates" / "api" / "src" / "parameter_mapping_handlers.rs",),
    ),
)


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def load_openapi_paths() -> set[str]:
    payload = json.loads(OPENAPI_JSON.read_text(encoding="utf-8"))
    paths = payload.get("paths", {})
    if not isinstance(paths, dict):
        raise ValueError("shared/openapi/openapi.json 缺少 paths 对象")
    return set(paths)


def load_openapi_operations() -> set[str]:
    payload = json.loads(OPENAPI_JSON.read_text(encoding="utf-8"))
    paths = payload.get("paths", {})
    if not isinstance(paths, dict):
        raise ValueError("shared/openapi/openapi.json 缺少 paths 对象")
    return {
        f"{method.upper()} {path}"
        for path, item in paths.items()
        if isinstance(item, dict)
        for method in item
        if method in HTTP_METHODS
    }


def _route_calls(source: str) -> list[str]:
    calls: list[str] = []
    cursor = 0
    while (start := source.find(".route(", cursor)) >= 0:
        index = start + len(".route(")
        depth = 1
        quoted = False
        escaped = False
        while index < len(source) and depth:
            char = source[index]
            if quoted:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    quoted = False
            elif char == '"':
                quoted = True
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
            index += 1
        if depth == 0:
            calls.append(source[start + len(".route("):index - 1])
        cursor = max(index, start + len(".route("))
    return calls


def operations_from_sources(sources: list[str]) -> set[str]:
    operations: set[str] = set()
    for source in sources:
        for call in _route_calls(source):
            path_match = re.match(r'\s*"([^"]+)"\s*,', call)
            if not path_match:
                continue
            path = re.sub(r":([A-Za-z_][A-Za-z0-9_]*)", r"{\1}", path_match.group(1))
            handler_expression = call[path_match.end():]
            for method in HTTP_METHODS:
                if re.search(rf"\b{method}\s*\(", handler_expression):
                    operations.add(f"{method.upper()} {path}")
    return operations


def invalid_axum_route_paths(source: str) -> set[str]:
    invalid: set[str] = set()
    for call in _route_calls(source):
        path_match = re.match(r'\s*"([^"]+)"\s*,', call)
        if path_match and re.search(r"\{[A-Za-z_][A-Za-z0-9_]*\}", path_match.group(1)):
            invalid.add(path_match.group(1))
    return invalid


def mounted_runtime_sources() -> list[tuple[Path, str]]:
    entry = WMS_API_RS.read_text(encoding="utf-8").split("#[cfg(test)]", 1)[0]
    mounted_routers = set(re.findall(r"\.merge\(\s*([a-zA-Z0-9_]+_router)\s*\(", entry))
    sources = [(WMS_API_RS, entry)]
    for path in API_SRC.glob("*.rs"):
        if path == WMS_API_RS:
            continue
        source = path.read_text(encoding="utf-8").split("#[cfg(test)]", 1)[0]
        if any(re.search(rf"pub\s+fn\s+{re.escape(router)}\b", source) for router in mounted_routers):
            sources.append((path, source))
    return sources


def runtime_operations() -> set[str]:
    return operations_from_sources([source for _, source in mounted_runtime_sources()])


def scan(*, strict: bool = False) -> list[Issue]:
    issues: list[Issue] = []
    for required in (OPENAPI_JSON, WMS_API_RS, API_LIB_RS):
        if not required.exists():
            issues.append(Issue(rel(required), "必需文件不存在"))
    if issues:
        return issues

    openapi_paths = load_openapi_paths()
    runtime_text = WMS_API_RS.read_text(encoding="utf-8")
    lib_text = API_LIB_RS.read_text(encoding="utf-8")
    combined = f"{runtime_text}\n{lib_text}"

    specs = (*ROUTE_MOUNT_SPECS, *STRICT_ROUTE_MOUNT_SPECS) if strict else ROUTE_MOUNT_SPECS
    for spec in specs:
        if spec.openapi_path not in openapi_paths:
            continue
        for token in spec.tokens:
            if token not in combined:
                issues.append(Issue(rel(WMS_API_RS), f"{spec.name} OpenAPI 已声明但运行时缺少挂载标记: {token}"))
        for path in spec.files:
            if not path.exists():
                issues.append(Issue(rel(path), f"{spec.name} OpenAPI 已声明但缺少运行时 handler 文件"))

    for path, source in mounted_runtime_sources():
        issues.extend(
            Issue(
                rel(path),
                f"Axum 0.7 动态路由必须使用 :param，不能使用 OpenAPI {{param}} 语法: {route}",
            )
            for route in sorted(invalid_axum_route_paths(source))
        )

    if strict:
        missing_operations = sorted(load_openapi_operations() - runtime_operations())
        issues.extend(
            Issue(rel(WMS_API_RS), f"OpenAPI operation 未挂载到已合并运行时 Router: {operation}")
            for operation in missing_operations
        )

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--strict", action="store_true", help="逐 operation 检查全部正式 OpenAPI 路由")
    args = parser.parse_args(argv)

    issues = scan(strict=args.strict)
    payload = {
        "check": "check_runtime_route_mounts",
        "tier": "T1",
        "category": "接口契约治理",
        "openapi": rel(OPENAPI_JSON),
        "runtime_entry": rel(WMS_API_RS),
        "strict": args.strict,
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }

    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_runtime_route_mounts (T1, 接口契约治理)")
        print(f"  · OpenAPI: {payload['openapi']}")
        print(f"  · Runtime: {payload['runtime_entry']}")
        if issues:
            print(f"  ✘ {len(issues)} 处路由挂载缺口:")
            for issue in issues:
                print(f"    - {issue.file}: {issue.message}")
        else:
            print("  ✓ 核心 OpenAPI 路由族已挂载到运行时")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
