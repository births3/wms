#!/usr/bin/env python3
"""validate_openapi_artifacts.py — OpenAPI 产物完整性校验

类别：1. 文档治理
Tier：T2（< 10s）
输入：shared/openapi/openapi.json
输出：人类可读 + --json
退出码：
  0  通过
  1  产物缺失或结构不合法
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


@dataclass
class Issue:
    kind: str
    detail: str


def validate_openapi_document(data: object) -> list[Issue]:
    issues: list[Issue] = []
    if not isinstance(data, dict):
        return [Issue("invalid_type", "openapi.json 顶层必须是 JSON object")]

    if data.get("openapi") != "3.0.3":
        issues.append(Issue("openapi_version", "openapi 字段必须为 3.0.3"))

    paths = data.get("paths")
    if not isinstance(paths, dict) or not paths:
        issues.append(Issue("paths", "paths 必须存在且非空"))

    components = data.get("components")
    schemas = components.get("schemas") if isinstance(components, dict) else None
    if not isinstance(schemas, dict) or not schemas:
        issues.append(Issue("schemas", "components.schemas 必须存在且非空"))

    return issues


def load_and_validate_openapi(path: Path = OPENAPI_JSON) -> tuple[list[Issue], dict[str, object]]:
    if not path.exists():
        return [Issue("missing", f"缺少 {path.relative_to(REPO_ROOT)}")], {"path": str(path.relative_to(REPO_ROOT))}

    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return [Issue("invalid_json", f"JSON 解析失败: {e}")], {"path": str(path.relative_to(REPO_ROOT))}

    issues = validate_openapi_document(data)
    stats = {
        "path": str(path.relative_to(REPO_ROOT)),
        "openapi": data.get("openapi"),
        "path_count": len(data.get("paths", {})) if isinstance(data.get("paths"), dict) else 0,
        "schema_count": len(data.get("components", {}).get("schemas", {}))
        if isinstance(data.get("components"), dict) and isinstance(data.get("components", {}).get("schemas"), dict)
        else 0,
    }
    return issues, stats


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)

    issues, stats = load_and_validate_openapi()
    ok = not issues

    if args.json:
        print(json.dumps({
            "check": "validate_openapi_artifacts",
            "tier": "T2",
            "category": "文档治理",
            **stats,
            "issues": [asdict(i) for i in issues],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("validate_openapi_artifacts (T2, 文档治理)")
        print(f"  · path: {stats.get('path', 'shared/openapi/openapi.json')}")
        if ok:
            print(
                "  ✓ openapi.json 结构有效"
                f" (openapi={stats['openapi']}, paths={stats['path_count']}, schemas={stats['schema_count']})"
            )
        else:
            print(f"  ✘ 发现 {len(issues)} 个问题:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.detail}")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
