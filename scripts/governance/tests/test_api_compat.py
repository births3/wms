import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def write_openapi(path: Path, *, required: list[str], include_get: bool = True) -> None:
    operations = {
        "post": {
            "requestBody": {
                "required": True,
                "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Item"}}},
            },
            "responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Item"}}}}},
        }
    }
    if include_get:
        operations["get"] = {"responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Item"}}}}}}
    path.write_text(json.dumps({
        "paths": {"/items": operations},
        "components": {"schemas": {"Item": {
            "type": "object",
            "required": required,
            "properties": {"id": {"type": "string"}, "name": {"type": "string"}},
        }}},
    }), encoding="utf-8")


def test_api_compat_detects_removed_operation_and_new_required_property(tmp_path):
    from check_api_compat import build_surface, check, load_json

    old = tmp_path / "old.json"
    current = tmp_path / "current.json"
    baseline = tmp_path / "baseline.json"
    write_openapi(old, required=["id"])
    write_openapi(current, required=["id", "name"], include_get=False)
    baseline.write_text(json.dumps(build_surface(load_json(old))), encoding="utf-8")

    issues, _ = check(current, baseline)

    assert {issue.rule for issue in issues} == {"operation_removed", "required_property_added"}


def test_api_compat_allows_new_optional_property(tmp_path):
    from check_api_compat import build_surface, check, load_json

    current = tmp_path / "current.json"
    baseline = tmp_path / "baseline.json"
    write_openapi(current, required=["id"])
    surface = build_surface(load_json(current))
    surface["schemas"]["Item"]["properties"].pop("name")
    baseline.write_text(json.dumps(surface), encoding="utf-8")

    issues, _ = check(current, baseline)

    assert issues == []


def test_api_compat_detects_response_required_property_becoming_optional(tmp_path):
    from check_api_compat import build_surface, check, load_json

    old = tmp_path / "old.json"
    current = tmp_path / "current.json"
    baseline = tmp_path / "baseline.json"
    write_openapi(old, required=["id", "name"])
    write_openapi(current, required=["id"])
    baseline.write_text(json.dumps(build_surface(load_json(old))), encoding="utf-8")

    issues, _ = check(current, baseline)

    assert {issue.rule for issue in issues} == {"required_property_removed"}


def test_api_compat_detects_inline_nested_schema_change(tmp_path):
    from check_api_compat import build_surface, compare_surfaces

    old = {
        "paths": {"/items": {"post": {
            "requestBody": {"content": {"application/json": {"schema": {
                "type": "object",
                "required": ["payload"],
                "properties": {"payload": {
                    "type": "object",
                    "required": ["id"],
                    "properties": {"id": {"type": "string"}},
                }},
            }}}},
            "responses": {},
        }}},
        "components": {"schemas": {}},
    }
    current = json.loads(json.dumps(old))
    current["paths"]["/items"]["post"]["requestBody"]["content"]["application/json"]["schema"]["properties"]["payload"]["properties"]["id"]["type"] = "integer"

    issues = compare_surfaces(build_surface(old), build_surface(current))

    assert {issue.rule for issue in issues} == {"request_schema_changed"}


def test_api_compat_allows_inline_nested_optional_property(tmp_path):
    from check_api_compat import build_surface, compare_surfaces

    old = {
        "paths": {"/items": {"post": {
            "requestBody": {"content": {"application/json": {"schema": {
                "type": "object",
                "properties": {"payload": {"type": "object", "properties": {}}},
            }}}},
            "responses": {},
        }}},
        "components": {"schemas": {}},
    }
    current = json.loads(json.dumps(old))
    current["paths"]["/items"]["post"]["requestBody"]["content"]["application/json"]["schema"]["properties"]["payload"]["properties"]["note"] = {"type": "string"}

    assert compare_surfaces(build_surface(old), build_surface(current)) == []


def test_api_compat_detects_component_root_type_change():
    from check_api_compat import build_surface, compare_surfaces

    old = {"paths": {}, "components": {"schemas": {"Status": {"type": "string", "enum": ["draft"]}}}}
    current = {"paths": {}, "components": {"schemas": {"Status": {"type": "integer", "enum": [1]}}}}

    issues = compare_surfaces(build_surface(old), build_surface(current))

    assert {issue.rule for issue in issues} == {"schema_changed"}


def test_api_compat_detects_tightened_input_constraint():
    from check_api_compat import build_surface, compare_surfaces

    old = {"paths": {"/items": {"post": {
        "requestBody": {"content": {"application/json": {"schema": {
            "type": "string", "minLength": 1,
        }}}},
        "responses": {},
    }}}, "components": {"schemas": {}}}
    current = json.loads(json.dumps(old))
    current["paths"]["/items"]["post"]["requestBody"]["content"]["application/json"]["schema"]["minLength"] = 99

    issues = compare_surfaces(build_surface(old), build_surface(current))

    assert {issue.rule for issue in issues} == {"request_schema_changed"}


def test_api_compat_allows_optional_nested_parameter_property():
    from check_api_compat import build_surface, compare_surfaces

    old = {"paths": {"/items": {"get": {
        "parameters": [{
            "in": "query",
            "name": "filter",
            "schema": {"type": "object", "properties": {}},
        }],
        "responses": {},
    }}}, "components": {"schemas": {}}}
    current = json.loads(json.dumps(old))
    current["paths"]["/items"]["get"]["parameters"][0]["schema"]["properties"]["note"] = {"type": "string"}

    assert compare_surfaces(build_surface(old), build_surface(current)) == []


def test_api_compat_allows_request_required_field_becoming_optional():
    from check_api_compat import build_surface, compare_surfaces

    old = {
        "paths": {"/items": {"post": {
            "requestBody": {"content": {"application/json": {"schema": {
                "$ref": "#/components/schemas/ItemRequest",
            }}}},
            "responses": {},
        }}},
        "components": {"schemas": {"ItemRequest": {
            "type": "object",
            "required": ["name"],
            "properties": {"name": {"type": "string"}},
        }}},
    }
    current = json.loads(json.dumps(old))
    current["components"]["schemas"]["ItemRequest"]["required"] = []

    assert compare_surfaces(build_surface(old), build_surface(current)) == []


def test_api_compat_allows_response_optional_field_becoming_required():
    from check_api_compat import build_surface, compare_surfaces

    old = {
        "paths": {"/items": {"get": {"responses": {"200": {
            "content": {"application/json": {"schema": {
                "$ref": "#/components/schemas/ItemResponse",
            }}},
        }}}}},
        "components": {"schemas": {"ItemResponse": {
            "type": "object",
            "properties": {"id": {"type": "string"}},
        }}},
    }
    current = json.loads(json.dumps(old))
    current["components"]["schemas"]["ItemResponse"]["required"] = ["id"]

    assert compare_surfaces(build_surface(old), build_surface(current)) == []


def test_api_compat_allows_required_parameter_becoming_optional():
    from check_api_compat import build_surface, compare_surfaces

    old = {"paths": {"/items": {"get": {
        "parameters": [{
            "in": "query", "name": "filter", "required": True,
            "schema": {"type": "string"},
        }],
        "responses": {},
    }}}, "components": {"schemas": {}}}
    current = json.loads(json.dumps(old))
    current["paths"]["/items"]["get"]["parameters"][0]["required"] = False

    assert compare_surfaces(build_surface(old), build_surface(current)) == []


def test_api_compat_rejects_optional_parameter_becoming_required():
    from check_api_compat import build_surface, compare_surfaces

    old = {"paths": {"/items": {"get": {
        "parameters": [{
            "in": "query", "name": "filter", "required": False,
            "schema": {"type": "string"},
        }],
        "responses": {},
    }}}, "components": {"schemas": {}}}
    current = json.loads(json.dumps(old))
    current["paths"]["/items"]["get"]["parameters"][0]["required"] = True

    issues = compare_surfaces(build_surface(old), build_surface(current))

    assert {issue.rule for issue in issues} == {"parameter_changed"}


def test_api_compat_propagates_request_usage_from_parameter_schema():
    from check_api_compat import build_surface, compare_surfaces

    old = {"paths": {"/items": {"get": {
        "parameters": [{
            "in": "query", "name": "filter",
            "schema": {"$ref": "#/components/schemas/Filter"},
        }],
        "responses": {},
    }}}, "components": {"schemas": {
        "Filter": {
            "type": "object", "required": ["code"],
            "properties": {"code": {"type": "string"}},
        },
        "NestedFilter": {
            "type": "object", "required": ["code"],
            "properties": {"code": {"type": "string"}},
        },
    }}}
    old["components"]["schemas"]["Filter"]["properties"]["nested"] = {
        "$ref": "#/components/schemas/NestedFilter",
    }
    current = json.loads(json.dumps(old))
    current["components"]["schemas"]["NestedFilter"]["required"] = []

    old_surface = build_surface(old)
    assert old_surface["schema_usage"]["Filter"] == ["request"]
    assert old_surface["schema_usage"]["NestedFilter"] == ["request"]
    assert compare_surfaces(old_surface, build_surface(current)) == []
