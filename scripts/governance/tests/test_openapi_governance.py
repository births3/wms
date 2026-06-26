"""OpenAPI 契约治理测试。

从 test_core_logic.py 拆出，覆盖 OpenAPI 文档结构、错误响应与
free-form JSON 边界。
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _ok_response() -> dict[str, str]:
    return {"description": "ok"}


def _error_response_401() -> dict[str, object]:
    return {
        "description": "unauthorized",
        "content": {
            "application/json": {
                "schema": {"$ref": "#/components/schemas/ErrorResponse"}
            }
        },
    }


def _responses_with_401() -> dict[str, object]:
    return {"200": _ok_response(), "401": _error_response_401()}


def _minimal_secured_paths(**overrides) -> dict[str, object]:
    paths = {
        "/api/v1/healthz": {"get": {"responses": {"200": _ok_response()}}},
        "/api/v1/auth/login": {"post": {"responses": _responses_with_401()}},
        "/api/v1/auth/me": {"get": {"responses": _responses_with_401()}},
        "/api/v1/audit/events": {"get": {"responses": _responses_with_401()}},
    }
    paths.update(overrides)
    return paths


def test_validate_openapi_document_requires_version_paths_and_schemas():
    """OpenAPI 产物最小结构缺一不可。"""
    from validate_openapi_artifacts import validate_openapi_document

    issues = validate_openapi_document({
        "openapi": "3.1.0",
        "paths": {},
        "components": {"schemas": {}},
    })

    assert [issue.kind for issue in issues] == ["openapi_version", "paths", "schemas"]


def test_check_openapi_contract_detects_missing_401_error_response():
    """非 healthz path 缺少 401 ErrorResponse 应失败。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract({
        "paths": _minimal_secured_paths(
            **{
                "/api/v1/auth/login": {
                    "post": {"responses": {"200": _ok_response()}}
                },
            }
        )
    })

    assert any(issue.kind == "missing_401_error_response" for issue in issues)


def test_check_openapi_contract_requires_free_form_json_properties():
    """serde_json::Value 契约必须导出为可承载任意对象属性。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract({
        "paths": _minimal_secured_paths(),
        "components": {
            "schemas": {
                "AuditEvent": {"properties": {"diff": {"type": "object"}}},
                "ErrorResponse": {
                    "properties": {
                        "details": {"type": "object", "additionalProperties": True}
                    }
                },
            }
        },
    })

    assert any(issue.kind == "missing_free_form_object" for issue in issues)


def test_check_openapi_contract_requires_wave2_and_wave3_paths_and_schemas():
    """Wave 2/3 核心合同缺失时必须失败。"""
    from check_openapi_contract import check_openapi_contract

    issues, stats = check_openapi_contract({
        "paths": _minimal_secured_paths(),
        "components": {"schemas": {"ErrorResponse": {"properties": {}}}},
    })

    assert "/api/v1/master-data/products" in stats["required_paths"]
    assert "/api/v1/inbound/receiving-orders/{id}/receive" in stats["required_paths"]
    assert "/api/v1/inventory/batches/status" in stats["required_paths"]
    assert "/api/v1/cold-chain/excursions" in stats["required_paths"]
    assert "/api/v1/billing/contracts" in stats["required_paths"]
    assert "Product" in stats["required_schemas"]
    assert "InventoryBatch" in stats["required_schemas"]
    assert "TemperatureExcursionEvent" in stats["required_schemas"]
    assert "BillingContract" in stats["required_schemas"]
    assert any(issue.kind == "missing_path" for issue in issues)
    assert any(issue.kind == "missing_schema" for issue in issues)
