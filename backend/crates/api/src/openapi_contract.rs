use std::collections::HashMap;

use utoipa::{
    openapi::{
        self,
        path::Operation,
        security::{
            ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityRequirement, SecurityScheme,
        },
        PathItemType,
    },
    Modify,
};

pub(crate) const BEARER_AUTH_SCHEME: &str = "BearerAuth";
pub(crate) const COLD_CHAIN_API_KEY_SCHEME: &str = "ColdChainApiKeyAuth";
pub(crate) const AUTH_EXEMPT_REASON: &str = "x-auth-exempt-reason";
pub(crate) const IDEMPOTENCY_EXEMPT_REASON: &str = "x-idempotency-exempt-reason";

struct IdempotencyExemptionGroup {
    operations: &'static [(&'static str, PathItemType)],
    reason: &'static str,
}

const IDEMPOTENCY_EXEMPTION_GROUPS: &[IdempotencyExemptionGroup] = &[
    IdempotencyExemptionGroup {
        operations: &[("/api/v1/auth/login", PathItemType::Post)],
        reason: "登录用于签发 JWT，本身不执行业务写入；重复登录由认证服务语义处理。",
    },
    IdempotencyExemptionGroup {
        operations: &[("/api/v1/wechat-notify/settings/test", PathItemType::Post)],
        reason: "企业微信参数测试只校验已保存配置，不写入业务数据；使用 POST 表达受控测试动作。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            ("/api/v1/billing/accounts", PathItemType::Post),
            ("/api/v1/billing/rules", PathItemType::Post),
        ],
        reason: "Wave 5 计费首批切片尚未接入业务幂等，本契约先显式标记豁免。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            ("/api/v1/cold-chain/devices", PathItemType::Post),
            (
                "/api/v1/cold-chain/excursions/{external_event_id}/dispose",
                PathItemType::Post,
            ),
        ],
        reason: "冷链首批切片尚未接入业务幂等，本契约先显式标记豁免。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            (
                "/api/v1/config-center/feature-flags/archive-file-source",
                PathItemType::Post,
            ),
            (
                "/api/v1/config-center/feature-flags/import",
                PathItemType::Post,
            ),
            (
                "/api/v1/config-center/feature-flags/migrate",
                PathItemType::Post,
            ),
            (
                "/api/v1/config-center/feature-flags/source",
                PathItemType::Post,
            ),
        ],
        reason: "配置中心迁移类受控运维动作后续补运行时幂等证据。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            ("/api/v1/inbound/receiving-orders/{id}", PathItemType::Patch),
            (
                "/api/v1/inbound/receiving-orders/{id}",
                PathItemType::Delete,
            ),
        ],
        reason: "收货单 CRUD 旧契约未接入 L11 幂等，本轮不扩大调用侧 header 面。",
    },
    IdempotencyExemptionGroup {
        operations: &[("/api/v1/inventory/batches/putaway", PathItemType::Post)],
        reason: "库存上架旧契约未接入 L11 幂等，本轮仅声明豁免并保留后续补齐路径。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            ("/api/v1/master-data/customers", PathItemType::Post),
            ("/api/v1/master-data/customers/{id}", PathItemType::Patch),
            ("/api/v1/master-data/customers/{id}", PathItemType::Delete),
            ("/api/v1/master-data/locations", PathItemType::Post),
            ("/api/v1/master-data/locations/{id}", PathItemType::Patch),
            ("/api/v1/master-data/locations/{id}", PathItemType::Delete),
            ("/api/v1/master-data/products", PathItemType::Post),
            ("/api/v1/master-data/products/{id}", PathItemType::Patch),
            ("/api/v1/master-data/products/{id}", PathItemType::Delete),
            (
                "/api/v1/master-data/special-drug-categories",
                PathItemType::Post,
            ),
            (
                "/api/v1/master-data/special-drug-categories/{id}",
                PathItemType::Patch,
            ),
            (
                "/api/v1/master-data/special-drug-categories/{id}",
                PathItemType::Delete,
            ),
            ("/api/v1/master-data/suppliers", PathItemType::Post),
            ("/api/v1/master-data/suppliers/{id}", PathItemType::Patch),
            ("/api/v1/master-data/suppliers/{id}", PathItemType::Delete),
            ("/api/v1/master-data/warehouses", PathItemType::Post),
            ("/api/v1/master-data/warehouses/{id}", PathItemType::Patch),
            ("/api/v1/master-data/warehouses/{id}", PathItemType::Delete),
        ],
        reason: "M1 基础档案 CRUD 旧契约未接入 L11 幂等，本轮不扩大调用侧 header 面。",
    },
    IdempotencyExemptionGroup {
        operations: &[("/api/v1/parameter-mapping/execute", PathItemType::Post)],
        reason: "参数对照执行使用业务追踪 ID，L11 幂等运行时证据后续切片补齐。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            ("/api/v1/reports/gsp/inbound-ledger", PathItemType::Post),
            ("/api/v1/reports/gsp/inventory-ledger", PathItemType::Post),
            ("/api/v1/reports/gsp/outbound-ledger", PathItemType::Post),
            ("/api/v1/reports/query", PathItemType::Post),
        ],
        reason: "报表查询为只读语义，使用 POST 承载复杂查询条件，不要求 Idempotency-Key。",
    },
    IdempotencyExemptionGroup {
        operations: &[("/api/v1/traceability/outbound-reports", PathItemType::Post)],
        reason: "追溯码上报首批契约未接入 L11 幂等，本轮仅声明豁免并保留后续补齐路径。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            (
                "/api/v1/event-bus/deliveries/{delivery_id}/ack",
                PathItemType::Post,
            ),
            (
                "/api/v1/event-bus/deliveries/{delivery_id}/nack",
                PathItemType::Post,
            ),
        ],
        reason: "事件投递确认以 delivery_id 标识单次投递状态，本轮不扩大调用侧 header 面。",
    },
    IdempotencyExemptionGroup {
        operations: &[
            ("/api/v1/print-templates/resolve", PathItemType::Post),
            ("/api/v1/print-templates/preview", PathItemType::Post),
        ],
        reason:
            "打印模板解析和预览为只读语义，使用 POST 承载复杂模板数据，不要求 Idempotency-Key。",
    },
];

pub(crate) struct ContractSecurityAddon;

impl Modify for ContractSecurityAddon {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(|| openapi::ComponentsBuilder::new().build());
        components.add_security_scheme(
            BEARER_AUTH_SCHEME,
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
        components.add_security_scheme(
            COLD_CHAIN_API_KEY_SCHEME,
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "X-WMS-API-Key",
                "外部冷链系统 API Key",
            ))),
        );

        openapi.security = Some(vec![SecurityRequirement::new(
            BEARER_AUTH_SCHEME,
            Vec::<String>::new(),
        )]);

        mark_public_operation(
            openapi,
            "/api/v1/healthz",
            PathItemType::Get,
            "健康探针不依赖登录态，用于负载均衡和运行时就绪检查。",
        );
        mark_public_operation(
            openapi,
            "/openapi.json",
            PathItemType::Get,
            "OpenAPI JSON 是公开契约产物；生产访问边界由网关或内网 ACL 控制。",
        );
        mark_public_operation(
            openapi,
            "/api-docs",
            PathItemType::Get,
            "API 文档浏览页只读取公开契约；生产访问边界由网关或内网 ACL 控制。",
        );
        mark_public_operation(
            openapi,
            "/redoc",
            PathItemType::Get,
            "生产只读 API 文档浏览页；访问边界由网关或内网 ACL 控制。",
        );
        mark_public_operation(
            openapi,
            "/api/v1/resilience/status",
            PathItemType::Get,
            "H3 韧性状态供运行时探测；生产访问边界由网关或内网 ACL 控制。",
        );
        mark_public_operation(
            openapi,
            "/metrics",
            PathItemType::Get,
            "Prometheus 指标由内网抓取；生产访问边界由网关或内网 ACL 控制。",
        );
        mark_public_operation(
            openapi,
            "/api/v1/auth/login",
            PathItemType::Post,
            "登录接口用于签发 JWT，调用前尚无 Bearer token。",
        );

        for path in [
            "/api/v1/cold-chain/readings",
            "/api/v1/cold-chain/excursions",
        ] {
            if let Some(operation) = operation_mut(openapi, path, PathItemType::Post) {
                operation.security = Some(vec![SecurityRequirement::new(
                    COLD_CHAIN_API_KEY_SCHEME,
                    Vec::<String>::new(),
                )]);
            }
        }

        for group in IDEMPOTENCY_EXEMPTION_GROUPS {
            for (path, method) in group.operations {
                if let Some(operation) = operation_mut(openapi, path, method.clone()) {
                    set_extension(operation, IDEMPOTENCY_EXEMPT_REASON, group.reason);
                }
            }
        }
    }
}

fn mark_public_operation(
    openapi: &mut openapi::OpenApi,
    path: &str,
    method: PathItemType,
    reason: &str,
) {
    if let Some(operation) = operation_mut(openapi, path, method) {
        operation.security = Some(Vec::new());
        set_extension(operation, AUTH_EXEMPT_REASON, reason);
    }
}

fn operation_mut<'a>(
    openapi: &'a mut openapi::OpenApi,
    path: &str,
    method: PathItemType,
) -> Option<&'a mut Operation> {
    openapi
        .paths
        .paths
        .get_mut(path)?
        .operations
        .get_mut(&method)
}

fn set_extension(operation: &mut Operation, key: &str, value: &str) {
    let extensions = operation.extensions.get_or_insert_with(HashMap::new);
    extensions.insert(key.to_owned(), serde_json::Value::String(value.to_owned()));
}
