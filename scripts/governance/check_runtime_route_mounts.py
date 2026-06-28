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
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
OPENAPI_JSON = REPO_ROOT / "shared" / "openapi" / "openapi.json"
WMS_API_RS = REPO_ROOT / "backend" / "crates" / "api" / "src" / "bin" / "wms_api.rs"
API_LIB_RS = REPO_ROOT / "backend" / "crates" / "api" / "src" / "lib.rs"


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


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def load_openapi_paths() -> set[str]:
    payload = json.loads(OPENAPI_JSON.read_text(encoding="utf-8"))
    paths = payload.get("paths", {})
    if not isinstance(paths, dict):
        raise ValueError("shared/openapi/openapi.json 缺少 paths 对象")
    return set(paths)


def scan() -> list[Issue]:
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

    for spec in ROUTE_MOUNT_SPECS:
        if spec.openapi_path not in openapi_paths:
            continue
        for token in spec.tokens:
            if token not in combined:
                issues.append(Issue(rel(WMS_API_RS), f"{spec.name} OpenAPI 已声明但运行时缺少挂载标记: {token}"))
        for path in spec.files:
            if not path.exists():
                issues.append(Issue(rel(path), f"{spec.name} OpenAPI 已声明但缺少运行时 handler 文件"))

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues = scan()
    payload = {
        "check": "check_runtime_route_mounts",
        "tier": "T1",
        "category": "接口契约治理",
        "openapi": rel(OPENAPI_JSON),
        "runtime_entry": rel(WMS_API_RS),
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
