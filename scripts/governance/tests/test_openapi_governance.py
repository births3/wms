"""OpenAPI 契约治理测试。

覆盖 OpenAPI 文档结构、错误响应与 free-form JSON 边界。
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


def _h3_security_schemes() -> dict[str, object]:
    return {
        "BearerAuth": {"type": "http", "scheme": "bearer", "bearerFormat": "JWT"},
        "ColdChainApiKeyAuth": {
            "type": "apiKey",
            "in": "header",
            "name": "X-WMS-API-Key",
        },
    }


def _secured_contract(**paths) -> dict[str, object]:
    return {
        "security": [{"BearerAuth": []}],
        "paths": _minimal_secured_paths(**paths),
        "components": {"schemas": {}, "securitySchemes": _h3_security_schemes()},
    }


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


def test_check_openapi_contract_requires_h3_security_schemes_and_global_bearer():
    """H3 P0 要求 OpenAPI 声明 Bearer/JWT 与全局鉴权。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract({
        "paths": _minimal_secured_paths(),
        "components": {"schemas": {}, "securitySchemes": {}},
    })

    assert any(issue.kind == "missing_security_scheme" for issue in issues)
    assert any(issue.kind == "missing_global_bearer_auth" for issue in issues)


def test_check_openapi_contract_requires_public_auth_exempt_reason():
    """公开接口必须显式覆盖全局鉴权并写明免鉴权原因。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract(_secured_contract(
        **{
            "/api/v1/healthz": {
                "get": {"responses": {"200": _ok_response()}, "security": []}
            },
            "/api/v1/auth/login": {
                "post": {"responses": _responses_with_401(), "security": []}
            },
        }
    ))

    assert any(issue.kind == "missing_auth_exempt_reason" for issue in issues)


def test_check_openapi_contract_rejects_unexpected_public_operation():
    """非白名单接口不能绕过全局 Bearer 鉴权。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract(_secured_contract(
        **{
            "/api/v1/auth/me": {
                "get": {"responses": _responses_with_401(), "security": []}
            },
        }
    ))

    assert any(issue.kind == "unexpected_auth_exempt_security" for issue in issues)


def test_check_openapi_contract_allows_documentation_and_metrics_public_operations():
    """文档、指标和韧性探测接口由内网边界保护，可显式免 Bearer。"""
    from check_openapi_contract import AUTH_EXEMPT_REASON, check_openapi_contract

    issues, _ = check_openapi_contract(_secured_contract(
        **{
            path: {
                "get": {
                    "responses": _responses_with_401(),
                    "security": [],
                    AUTH_EXEMPT_REASON: "内网访问边界保护。",
                }
            }
            for path in [
                "/openapi.json",
                "/api-docs",
                "/redoc",
                "/api/v1/resilience/status",
                "/metrics",
            ]
        }
    ))

    assert not any(issue.kind == "unexpected_auth_exempt_security" for issue in issues)


def test_check_openapi_contract_requires_cold_chain_api_key_security():
    """外部冷链写入接口必须使用 API Key security scheme。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract(_secured_contract(
        **{
            "/api/v1/cold-chain/readings": {
                "post": {
                    "responses": _responses_with_401(),
                    "parameters": [
                        {"name": "Idempotency-Key", "in": "header"},
                        {"name": "X-WMS-API-Key", "in": "header"},
                    ],
                }
            }
        }
    ))

    assert any(issue.kind == "missing_cold_chain_api_key_auth" for issue in issues)


def test_check_openapi_contract_requires_idempotency_key_or_exemption():
    """写操作必须声明 Idempotency-Key 或明确豁免原因。"""
    from check_openapi_contract import check_openapi_contract

    issues, _ = check_openapi_contract(_secured_contract(
        **{
            "/api/v1/master-data/products": {
                "post": {"responses": _responses_with_401()}
            }
        }
    ))

    assert any(issue.kind == "missing_idempotency_contract" for issue in issues)


def test_curl_example_uses_declared_api_key_security_instead_of_bearer():
    """外部 API operation 的 curl 必须使用其声明的 API Key。"""
    from generate_openapi_curl_examples import curl_for

    parts = curl_for(
        "post",
        "/api/v1/integration/erp-messages/{id}/receipt",
        {
            "security": [{"ColdChainApiKeyAuth": []}],
            "parameters": [
                {"name": "X-WMS-API-Key", "in": "header", "required": True},
            ],
        },
    )

    assert '"X-WMS-API-Key: $WMS_API_KEY"' in parts
    assert '"Authorization: Bearer $WMS_TOKEN"' not in parts


def test_curl_example_includes_required_idempotency_header_once():
    """operation 必填幂等头不得从生成示例中丢失或重复。"""
    from generate_openapi_curl_examples import curl_for

    parts = curl_for(
        "post",
        "/api/v1/integration/erp-messages/{id}/receipt",
        {
            "security": [{"ColdChainApiKeyAuth": []}],
            "parameters": [
                {"name": "Idempotency-Key", "in": "header", "required": True},
                {"name": "X-WMS-API-Key", "in": "header", "required": True},
            ],
        },
    )

    assert parts.count('"Idempotency-Key: $IDEMPOTENCY_KEY"') == 1
    assert parts.count('"X-WMS-API-Key: $WMS_API_KEY"') == 1
