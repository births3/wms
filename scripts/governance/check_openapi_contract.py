#!/usr/bin/env python3
"""check_openapi_contract.py — Wave 1/2/3 OpenAPI 合同最小校验

类别：5. 接口契约治理
Tier：T2（< 10s）
输入：shared/openapi/openapi.json
输出：人类可读 + --json
退出码：
  0  通过
  1  合同缺失或 401 ErrorResponse 约束不满足
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
REQUIRED_PATHS = (
    "/api/v1/healthz",
    "/api/v1/auth/login",
    "/api/v1/auth/me",
    "/api/v1/audit/events",
    "/api/v1/master-data/products",
    "/api/v1/master-data/products/{id}",
    "/api/v1/master-data/suppliers",
    "/api/v1/master-data/suppliers/{id}",
    "/api/v1/master-data/customers",
    "/api/v1/master-data/customers/{id}",
    "/api/v1/master-data/warehouses",
    "/api/v1/master-data/warehouses/{id}",
    "/api/v1/master-data/locations",
    "/api/v1/master-data/locations/{id}",
    "/api/v1/master-data/special-drug-categories",
    "/api/v1/master-data/special-drug-categories/{id}",
    "/api/v1/inbound/receiving-orders",
    "/api/v1/inbound/receiving-orders/{id}",
    "/api/v1/inbound/receiving-orders/{id}/receive",
    "/api/v1/inbound/receiving-orders/{id}/inspect",
    "/api/v1/inbound/receiving-orders/{id}/sign",
    "/api/v1/inbound/receiving-orders/{id}/putaway",
    "/api/v1/inventory/batches",
    "/api/v1/inventory/batches/putaway",
    "/api/v1/inventory/batches/status",
    "/api/v1/reports/query",
    "/api/v1/parameter-mapping/map",
    "/api/v1/config-center/feature-flags/migrate",
    "/api/v1/config-center/feature-flags/reconcile",
    "/api/v1/config-center/feature-flags/export",
    "/api/v1/config-center/feature-flags/import",
    "/api/v1/config-center/feature-flags/source",
    "/api/v1/config-center/feature-flags/archive-file-source",
    "/api/v1/cold-chain/devices",
    "/api/v1/cold-chain/readings",
    "/api/v1/cold-chain/excursions",
    "/api/v1/billing/accounts",
    "/api/v1/billing/contracts",
    "/api/v1/billing/rules",
)
REQUIRED_SCHEMAS = (
    "Product",
    "CreateProductRequest",
    "UpdateProductRequest",
    "ProductListResponse",
    "Supplier",
    "CreateSupplierRequest",
    "UpdateSupplierRequest",
    "SupplierListResponse",
    "Customer",
    "CreateCustomerRequest",
    "UpdateCustomerRequest",
    "CustomerListResponse",
    "Warehouse",
    "CreateWarehouseRequest",
    "UpdateWarehouseRequest",
    "WarehouseListResponse",
    "Location",
    "CreateLocationRequest",
    "UpdateLocationRequest",
    "LocationListResponse",
    "SpecialDrugCategory",
    "CreateSpecialDrugCategoryRequest",
    "UpdateSpecialDrugCategoryRequest",
    "SpecialDrugCategoryListResponse",
    "ReceivingOrder",
    "ReceivingOrderLine",
    "CreateReceivingOrderRequest",
    "UpdateReceivingOrderRequest",
    "ReceivingOrderListResponse",
    "ReceiveReceivingOrderRequest",
    "ReceivingOrderReceipt",
    "InspectReceivingOrderRequest",
    "ReceivingInspectionRecord",
    "SignInspectionRequest",
    "InspectionSignatureRecord",
    "PutawayRequest",
    "PutawayRecord",
    "InventoryBatch",
    "InventoryBatchListResponse",
    "PutawayInventoryRequest",
    "ChangeInventoryStatusRequest",
    "InventoryMovement",
    "ReportQueryRequest",
    "ReportQueryResponse",
    "ReportRow",
    "MapParameterRequest",
    "MapParameterResponse",
    "ParameterMappingStatus",
    "ConfigEntry",
    "FeatureFlagArchiveRequest",
    "FeatureFlagArchiveResult",
    "FeatureFlagBatchImportRequest",
    "FeatureFlagBatchImportResult",
    "FeatureFlagConfig",
    "FeatureFlagExportResponse",
    "FeatureFlagMigrationResult",
    "FeatureFlagReconcileReport",
    "FeatureFlagSourceSwitchRequest",
    "FeatureFlagSourceSwitchResponse",
    "ColdChainDevice",
    "CreateColdChainDeviceRequest",
    "TemperatureReading",
    "IngestTemperatureReadingRequest",
    "TemperatureExcursionEvent",
    "IngestTemperatureExcursionRequest",
    "BillingAccount",
    "CreateBillingAccountRequest",
    "BillingContract",
    "CreateBillingContractRequest",
    "BillingRule",
    "CreateBillingRuleRequest",
)
HTTP_METHODS = {"get", "post", "put", "patch", "delete", "options", "head"}
WRITE_METHODS = {"post", "put", "patch", "delete"}
ERROR_RESPONSE_REF = "#/components/schemas/ErrorResponse"
AUTH_EXEMPT_REASON = "x-auth-exempt-reason"
IDEMPOTENCY_EXEMPT_REASON = "x-idempotency-exempt-reason"
BEARER_AUTH_SCHEME = "BearerAuth"
COLD_CHAIN_API_KEY_SCHEME = "ColdChainApiKeyAuth"
IDEMPOTENCY_HEADER = "Idempotency-Key"
COLD_CHAIN_API_KEY_HEADER = "X-WMS-API-Key"
AUTH_EXEMPT_OPERATIONS = {
    ("/api/v1/healthz", "get"): "健康探针不依赖登录态，用于负载均衡和运行时就绪检查。",
    ("/api/v1/auth/login", "post"): "登录接口用于签发 JWT，调用前尚无 Bearer token。",
    ("/openapi.json", "get"): "OpenAPI JSON 是公开契约产物；生产访问边界由网关或内网 ACL 控制。",
    ("/api-docs", "get"): "API 文档浏览页只读取公开契约；生产访问边界由网关或内网 ACL 控制。",
    ("/redoc", "get"): "生产只读 API 文档浏览页；访问边界由网关或内网 ACL 控制。",
    ("/api/v1/resilience/status", "get"): "H3 韧性状态供运行时探测；生产访问边界由网关或内网 ACL 控制。",
    ("/metrics", "get"): "Prometheus 指标由内网抓取；生产访问边界由网关或内网 ACL 控制。",
}
COLD_CHAIN_API_KEY_OPERATIONS = {
    ("/api/v1/cold-chain/readings", "post"),
    ("/api/v1/cold-chain/excursions", "post"),
}
REQUIRED_FREE_FORM_PROPERTIES = {
    "AuditEvent": ("diff",),
    "ConfigEntry": ("config_value",),
    "ErrorResponse": ("details",),
    "ReportQueryRequest": ("filters",),
    "ReportRow": ("values",),
}


@dataclass
class Issue:
    kind: str
    detail: str


def _has_error_response(operation: object) -> bool:
    if not isinstance(operation, dict):
        return False
    responses = operation.get("responses")
    if not isinstance(responses, dict):
        return False
    resp_401 = responses.get("401")
    if not isinstance(resp_401, dict):
        return False
    content = resp_401.get("content")
    if not isinstance(content, dict):
        return False
    app_json = content.get("application/json")
    if not isinstance(app_json, dict):
        return False
    schema = app_json.get("schema")
    return isinstance(schema, dict) and schema.get("$ref") == ERROR_RESPONSE_REF


def _has_security_requirement(security: object, scheme_name: str) -> bool:
    if not isinstance(security, list):
        return False
    return any(isinstance(item, dict) and scheme_name in item for item in security)


def _is_public_security_override(operation: object) -> bool:
    if not isinstance(operation, dict):
        return False
    security = operation.get("security")
    if security == []:
        return True
    return isinstance(security, list) and any(item == {} for item in security)


def _has_non_empty_string(obj: object, key: str) -> bool:
    return isinstance(obj, dict) and isinstance(obj.get(key), str) and bool(obj[key].strip())


def _has_header_parameter(operation: object, header_name: str) -> bool:
    if not isinstance(operation, dict):
        return False
    parameters = operation.get("parameters")
    if not isinstance(parameters, list):
        return False
    return any(
        isinstance(parameter, dict)
        and parameter.get("name") == header_name
        and str(parameter.get("in", "")).lower() == "header"
        for parameter in parameters
    )


def _is_bearer_jwt_scheme(scheme: object) -> bool:
    return (
        isinstance(scheme, dict)
        and scheme.get("type") == "http"
        and scheme.get("scheme") == "bearer"
        and scheme.get("bearerFormat") == "JWT"
    )


def _is_cold_chain_api_key_scheme(scheme: object) -> bool:
    return (
        isinstance(scheme, dict)
        and scheme.get("type") == "apiKey"
        and scheme.get("in") == "header"
        and scheme.get("name") == COLD_CHAIN_API_KEY_HEADER
    )


def check_openapi_contract(data: object) -> tuple[list[Issue], dict[str, object]]:
    issues: list[Issue] = []
    if not isinstance(data, dict):
        return [Issue("invalid_type", "openapi.json 顶层必须是 JSON object")], {
            "required_paths": list(REQUIRED_PATHS),
            "present_paths": [],
        }

    paths = data.get("paths")
    if not isinstance(paths, dict):
        return [Issue("missing_paths", "缺少 paths object")], {
            "required_paths": list(REQUIRED_PATHS),
            "present_paths": [],
        }

    present_paths = sorted(paths.keys())
    for required_path in REQUIRED_PATHS:
        if required_path not in paths:
            issues.append(Issue("missing_path", f"缺少必需 path: {required_path}"))

    components = data.get("components")
    schemas = components.get("schemas") if isinstance(components, dict) else {}
    security_schemes = (
        components.get("securitySchemes")
        if isinstance(components, dict)
        else {}
    )
    if not isinstance(security_schemes, dict):
        security_schemes = {}

    if not _is_bearer_jwt_scheme(security_schemes.get(BEARER_AUTH_SCHEME)):
        issues.append(Issue(
            "missing_security_scheme",
            "components.securitySchemes.BearerAuth 必须声明 http bearer JWT",
        ))

    if not _is_cold_chain_api_key_scheme(security_schemes.get(COLD_CHAIN_API_KEY_SCHEME)):
        issues.append(Issue(
            "missing_security_scheme",
            "components.securitySchemes.ColdChainApiKeyAuth 必须声明 header X-WMS-API-Key",
        ))

    if not _has_security_requirement(data.get("security"), BEARER_AUTH_SCHEME):
        issues.append(Issue(
            "missing_global_bearer_auth",
            "OpenAPI 顶层 security 必须默认要求 BearerAuth",
        ))

    for path, path_item in paths.items():
        if path == "/api/v1/healthz" or not isinstance(path_item, dict):
            continue
        for method, operation in path_item.items():
            if method.lower() not in HTTP_METHODS:
                continue
            if not _has_error_response(operation):
                issues.append(Issue(
                    "missing_401_error_response",
                    f"{path} {method.upper()} 缺少 401 ErrorResponse",
                ))

    for (path, method), _reason in AUTH_EXEMPT_OPERATIONS.items():
        path_item = paths.get(path)
        operation = path_item.get(method) if isinstance(path_item, dict) else None
        if operation is None:
            continue
        if not _is_public_security_override(operation):
            issues.append(Issue(
                "missing_auth_exempt_security",
                f"{path} {method.upper()} 必须显式覆盖全局 BearerAuth",
            ))
        if not _has_non_empty_string(operation, AUTH_EXEMPT_REASON):
            issues.append(Issue(
                "missing_auth_exempt_reason",
                f"{path} {method.upper()} 必须声明 {AUTH_EXEMPT_REASON}",
            ))

    for path, path_item in paths.items():
        if not isinstance(path_item, dict):
            continue
        for method, operation in path_item.items():
            method_lower = method.lower()
            if method_lower not in HTTP_METHODS or not isinstance(operation, dict):
                continue
            if (
                (path, method_lower) not in AUTH_EXEMPT_OPERATIONS
                and _is_public_security_override(operation)
            ):
                issues.append(Issue(
                    "unexpected_auth_exempt_security",
                    f"{path} {method.upper()} 不在公开免鉴权白名单内，不能覆盖全局鉴权",
                ))

            if (path, method_lower) in COLD_CHAIN_API_KEY_OPERATIONS:
                requires_cold_chain_api_key = _has_security_requirement(
                    operation.get("security"),
                    COLD_CHAIN_API_KEY_SCHEME,
                )
                if not requires_cold_chain_api_key:
                    issues.append(Issue(
                        "missing_cold_chain_api_key_auth",
                        f"{path} {method.upper()} 必须声明 ColdChainApiKeyAuth",
                    ))

            if method_lower in WRITE_METHODS:
                if not (
                    _has_header_parameter(operation, IDEMPOTENCY_HEADER)
                    or _has_non_empty_string(operation, IDEMPOTENCY_EXEMPT_REASON)
                ):
                    issues.append(Issue(
                        "missing_idempotency_contract",
                        f"{path} {method.upper()} 必须声明 Idempotency-Key 或 {IDEMPOTENCY_EXEMPT_REASON}",
                    ))

    if isinstance(schemas, dict):
        for schema_name, property_names in REQUIRED_FREE_FORM_PROPERTIES.items():
            schema = schemas.get(schema_name)
            properties = schema.get("properties") if isinstance(schema, dict) else {}
            for property_name in property_names:
                prop = properties.get(property_name) if isinstance(properties, dict) else None
                if not (
                    isinstance(prop, dict)
                    and prop.get("type") == "object"
                    and prop.get("additionalProperties") is True
                ):
                    issues.append(Issue(
                        "missing_free_form_object",
                        f"{schema_name}.{property_name} 必须是 type=object + additionalProperties=true",
                    ))

    stats = {
        "required_paths": list(REQUIRED_PATHS),
        "present_paths": present_paths,
        "required_schemas": list(REQUIRED_SCHEMAS),
        "required_free_form_properties": {
            key: list(value) for key, value in REQUIRED_FREE_FORM_PROPERTIES.items()
        },
        "required_security_schemes": [
            BEARER_AUTH_SCHEME,
            COLD_CHAIN_API_KEY_SCHEME,
        ],
        "auth_exempt_operations": [
            {"path": path, "method": method, "reason": reason}
            for (path, method), reason in sorted(AUTH_EXEMPT_OPERATIONS.items())
        ],
        "cold_chain_api_key_operations": [
            {"path": path, "method": method}
            for path, method in sorted(COLD_CHAIN_API_KEY_OPERATIONS)
        ],
    }
    if isinstance(schemas, dict):
        present_schemas = set(schemas)
        for schema_name in REQUIRED_SCHEMAS:
            if schema_name not in present_schemas:
                issues.append(Issue("missing_schema", f"缺少必需 schema: {schema_name}"))

    return issues, stats


def load_and_check_contract(path: Path = OPENAPI_JSON) -> tuple[list[Issue], dict[str, object]]:
    if not path.exists():
        return [Issue("missing", f"缺少 {path.relative_to(REPO_ROOT)}")], {
            "required_paths": list(REQUIRED_PATHS),
            "required_schemas": list(REQUIRED_SCHEMAS),
            "present_paths": [],
            "path": str(path.relative_to(REPO_ROOT)),
        }

    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return [Issue("invalid_json", f"JSON 解析失败: {e}")], {
            "required_paths": list(REQUIRED_PATHS),
            "required_schemas": list(REQUIRED_SCHEMAS),
            "present_paths": [],
            "path": str(path.relative_to(REPO_ROOT)),
        }

    issues, stats = check_openapi_contract(data)
    stats["path"] = str(path.relative_to(REPO_ROOT))
    return issues, stats


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    args = parser.parse_args(argv)

    issues, stats = load_and_check_contract()
    ok = not issues

    if args.json:
        print(json.dumps({
            "check": "check_openapi_contract",
            "tier": "T2",
            "category": "接口契约治理",
            **stats,
            "issues": [asdict(i) for i in issues],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_openapi_contract (T2, 接口契约治理)")
        print(f"  · path: {stats.get('path', 'shared/openapi/openapi.json')}")
        if ok:
            print("  ✓ Wave 1/2/3 必需 path、schema 与 401 ErrorResponse 约束均满足")
        else:
            print(f"  ✘ 发现 {len(issues)} 个合同问题:")
            for issue in issues:
                print(f"    [{issue.kind}] {issue.detail}")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
