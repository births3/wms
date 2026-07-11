#!/usr/bin/env python3
"""以版本化 OpenAPI 兼容面阻止删除接口、字段和新增必填输入。"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
OPENAPI = REPO_ROOT / "shared/openapi/openapi.json"
BASELINE = REPO_ROOT / "governance/baselines/openapi-compat-v1.json"
HTTP_METHODS = {"get", "post", "put", "patch", "delete"}
SCHEMA_KEYS = (
    "$ref",
    "type",
    "format",
    "nullable",
    "enum",
    "items",
    "oneOf",
    "anyOf",
    "allOf",
    "additionalProperties",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minLength",
    "maxLength",
    "pattern",
    "minItems",
    "maxItems",
    "uniqueItems",
    "minProperties",
    "maxProperties",
    "required",
    "properties",
)


@dataclass(frozen=True)
class Issue:
    rule: str
    location: str
    message: str


def load_json(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path} 必须是 JSON object")
    return payload


def schema_signature(schema: object) -> object:
    if not isinstance(schema, dict):
        return schema
    result: dict[str, object] = {}
    for key in SCHEMA_KEYS:
        if key not in schema:
            continue
        value = schema[key]
        if key == "properties" and isinstance(value, dict):
            result[key] = {
                name: schema_signature(property_schema)
                for name, property_schema in sorted(value.items())
            }
        elif isinstance(value, list):
            result[key] = [schema_signature(item) for item in value]
        elif isinstance(value, dict):
            result[key] = schema_signature(value)
        else:
            result[key] = value
    return result


def content_schema(payload: object) -> object:
    if not isinstance(payload, dict):
        return {}
    content = payload.get("content", {})
    if not isinstance(content, dict):
        return {}
    media = content.get("application/json") or next(iter(content.values()), {})
    return schema_signature(media.get("schema", {})) if isinstance(media, dict) else {}


def schema_is_compatible(old: object, new: object, *, direction: str = "both") -> bool:
    if not isinstance(old, dict) or not isinstance(new, dict):
        return old == new
    for key, old_value in old.items():
        if key == "properties":
            continue
        if key == "required" and isinstance(old_value, list) and (
            key not in new or isinstance(new.get(key), list)
        ):
            continue
        if key not in new or not schema_is_compatible(old_value, new[key], direction=direction):
            return False
    if any(
        key not in old
        for key, value in new.items()
        if key != "properties" and not (key == "required" and isinstance(value, list))
    ):
        return False
    old_required = old.get("required", [])
    new_required = new.get("required", [])
    if isinstance(old_required, list) or isinstance(new_required, list):
        if not isinstance(old_required, list) or not isinstance(new_required, list):
            return False
        old_required_set = set(old_required)
        new_required_set = set(new_required)
        required_is_compatible = (
            new_required_set <= old_required_set
            if direction == "request"
            else old_required_set <= new_required_set
            if direction == "response"
            else old_required_set == new_required_set
        )
        if not required_is_compatible:
            return False
    old_properties = old.get("properties", {})
    new_properties = new.get("properties", {})
    if isinstance(old_properties, dict) and isinstance(new_properties, dict):
        for name, old_property in old_properties.items():
            if name not in new_properties or not schema_is_compatible(
                old_property, new_properties[name], direction=direction
            ):
                return False
    return True


def referenced_schemas(value: object) -> set[str]:
    if isinstance(value, list):
        return set().union(*(referenced_schemas(item) for item in value)) if value else set()
    if not isinstance(value, dict):
        return set()
    refs = {
        ref.rsplit("/", 1)[-1]
        for ref in [value.get("$ref")]
        if isinstance(ref, str) and ref.startswith("#/components/schemas/")
    }
    for child in value.values():
        refs.update(referenced_schemas(child))
    return refs


def parameter_surface(parameters: object) -> dict[str, object]:
    result: dict[str, object] = {}
    if not isinstance(parameters, list):
        return result
    for parameter in parameters:
        if not isinstance(parameter, dict):
            continue
        if "$ref" in parameter:
            result[f"ref:{parameter['$ref']}"] = {"schema": {"$ref": parameter["$ref"]}}
            continue
        key = f"{parameter.get('in', '')}:{parameter.get('name', '')}"
        result[key] = {
            "required": bool(parameter.get("required", False)),
            "schema": schema_signature(parameter.get("schema", {})),
        }
    return result


def parameter_is_compatible(old: object, new: object) -> bool:
    if not isinstance(old, dict) or not isinstance(new, dict):
        return old == new
    if not bool(old.get("required", False)) and bool(new.get("required", False)):
        return False
    return schema_is_compatible(old.get("schema", {}), new.get("schema", {}), direction="request")


def build_surface(openapi: dict[str, Any]) -> dict[str, object]:
    operations: dict[str, object] = {}
    paths = openapi.get("paths", {})
    if not isinstance(paths, dict):
        raise ValueError("OpenAPI 缺少 paths object")
    for path, item in sorted(paths.items()):
        if not isinstance(item, dict):
            continue
        inherited_parameters = item.get("parameters", [])
        for method, operation in sorted(item.items()):
            if method not in HTTP_METHODS or not isinstance(operation, dict):
                continue
            parameters = [
                *(inherited_parameters if isinstance(inherited_parameters, list) else []),
                *(operation.get("parameters", []) if isinstance(operation.get("parameters"), list) else []),
            ]
            request_body = operation.get("requestBody", {})
            responses = operation.get("responses", {})
            operations[f"{method.upper()} {path}"] = {
                "parameters": parameter_surface(parameters),
                "request_required": bool(
                    isinstance(request_body, dict) and request_body.get("required", False)
                ),
                "request_schema": content_schema(request_body),
                "responses": {
                    str(code): content_schema(response)
                    for code, response in sorted(responses.items())
                } if isinstance(responses, dict) else {},
            }

    schemas: dict[str, object] = {}
    components = openapi.get("components", {})
    raw_schemas = components.get("schemas", {}) if isinstance(components, dict) else {}
    if isinstance(raw_schemas, dict):
        for name, schema in sorted(raw_schemas.items()):
            if not isinstance(schema, dict):
                continue
            properties = schema.get("properties", {})
            schemas[name] = {
                "required": sorted(schema.get("required", [])),
                "properties": {
                    key: schema_signature(value)
                    for key, value in sorted(properties.items())
                } if isinstance(properties, dict) else {},
                "schema": schema_signature(schema),
            }
    schema_usage: dict[str, set[str]] = {name: set() for name in schemas}
    for operation in operations.values():
        for parameter in operation.get("parameters", {}).values():
            for name in referenced_schemas(parameter):
                schema_usage.setdefault(name, set()).add("request")
        for name in referenced_schemas(operation.get("request_schema", {})):
            schema_usage.setdefault(name, set()).add("request")
        for response in operation.get("responses", {}).values():
            for name in referenced_schemas(response):
                schema_usage.setdefault(name, set()).add("response")
    changed = True
    while changed:
        changed = False
        for name, schema in schemas.items():
            for referenced in referenced_schemas(schema):
                before = len(schema_usage.setdefault(referenced, set()))
                schema_usage[referenced].update(schema_usage.get(name, set()))
                changed |= len(schema_usage[referenced]) != before
    return {
        "version": 1,
        "operations": operations,
        "schemas": schemas,
        "schema_usage": {name: sorted(usage) for name, usage in sorted(schema_usage.items())},
    }


def compare_surfaces(baseline: dict[str, Any], current: dict[str, Any]) -> list[Issue]:
    issues: list[Issue] = []
    old_operations = baseline.get("operations", {})
    new_operations = current.get("operations", {})
    for operation, old in old_operations.items():
        if operation not in new_operations:
            issues.append(Issue("operation_removed", operation, "已发布接口被删除"))
            continue
        new = new_operations[operation]
        old_params = old.get("parameters", {})
        new_params = new.get("parameters", {})
        for key, old_param in old_params.items():
            if key not in new_params:
                issues.append(Issue("parameter_removed", f"{operation} {key}", "参数被删除"))
            elif not parameter_is_compatible(old_param, new_params[key]):
                issues.append(Issue("parameter_changed", f"{operation} {key}", "参数约束发生变化"))
        for key, new_param in new_params.items():
            if key not in old_params and new_param.get("required"):
                issues.append(Issue("required_parameter_added", f"{operation} {key}", "新增必填参数"))
        if not old.get("request_required") and new.get("request_required"):
            issues.append(Issue("request_body_required", operation, "请求体由可选改为必填"))
        if not schema_is_compatible(
            old.get("request_schema"), new.get("request_schema"), direction="request"
        ):
            issues.append(Issue("request_schema_changed", operation, "请求体 schema 发生变化"))
        for code, schema in old.get("responses", {}).items():
            if code not in new.get("responses", {}):
                issues.append(Issue("response_removed", f"{operation} {code}", "响应状态被删除"))
            elif not schema_is_compatible(schema, new["responses"][code], direction="response"):
                issues.append(Issue("response_schema_changed", f"{operation} {code}", "响应 schema 发生变化"))

    old_schemas = baseline.get("schemas", {})
    new_schemas = current.get("schemas", {})
    old_schema_usage = baseline.get("schema_usage", {})
    new_schema_usage = current.get("schema_usage", {})
    for name, old in old_schemas.items():
        if name not in new_schemas:
            issues.append(Issue("schema_removed", name, "组件 schema 被删除"))
            continue
        new = new_schemas[name]
        usage = set(old_schema_usage.get(name, [])) | set(new_schema_usage.get(name, []))
        direction = next(iter(usage)) if len(usage) == 1 else "both"
        old_properties = old.get("properties", {})
        new_properties = new.get("properties", {})
        for prop, signature in old_properties.items():
            if prop not in new_properties:
                issues.append(Issue("property_removed", f"{name}.{prop}", "响应/模型字段被删除"))
            elif not schema_is_compatible(signature, new_properties[prop], direction=direction):
                issues.append(Issue("property_changed", f"{name}.{prop}", "字段类型或约束发生变化"))
        newly_required = set(new.get("required", [])) - set(old.get("required", []))
        if direction in {"request", "both"}:
            for prop in sorted(newly_required):
                issues.append(Issue("required_property_added", f"{name}.{prop}", "新增必填字段"))
        no_longer_required = set(old.get("required", [])) - set(new.get("required", []))
        if direction in {"response", "both"}:
            for prop in sorted(no_longer_required):
                issues.append(Issue("required_property_removed", f"{name}.{prop}", "响应必填字段变为可选"))
        old_root = {key: value for key, value in old.get("schema", {}).items() if key not in {"properties", "required"}}
        new_root = {key: value for key, value in new.get("schema", {}).items() if key not in {"properties", "required"}}
        if not schema_is_compatible(old_root, new_root, direction=direction):
            issues.append(Issue("schema_changed", name, "组件根类型或约束发生变化"))
    return issues


def check(current_path: Path = OPENAPI, baseline_path: Path = BASELINE) -> tuple[list[Issue], dict[str, object]]:
    current = build_surface(load_json(current_path))
    if not baseline_path.exists():
        return [Issue("baseline_missing", str(baseline_path), "缺少版本化 OpenAPI 兼容基线")], current
    baseline = load_json(baseline_path)
    return compare_surfaces(baseline, current), current


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--update-baseline", action="store_true")
    args = parser.parse_args(argv)
    issues, current = check()
    if args.update_baseline:
        BASELINE.parent.mkdir(parents=True, exist_ok=True)
        BASELINE.write_text(json.dumps(current, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        issues = []
    payload = {
        "check": "check_api_compat",
        "tier": "T4",
        "baseline": str(BASELINE.relative_to(REPO_ROOT)),
        "operation_count": len(current["operations"]),
        "schema_count": len(current["schemas"]),
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_api_compat (T4) — operations={payload['operation_count']}, schemas={payload['schema_count']}")
        for issue in issues:
            print(f"  ✘ [{issue.rule}] {issue.location}: {issue.message}")
        if not issues:
            print("  ✓ OpenAPI 未出现基线不兼容变化")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
