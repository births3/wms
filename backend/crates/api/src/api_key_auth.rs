//! H1 外部 API Key 统一入口。

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;
use wms_domain::{inbound_scope_for_catalog_type, ErrorResponse};

use crate::{
    api_key_service::{ApiKeyAuthError, ApiKeyService},
    auth::AuthContext,
    h8_erp_messages::H8_RECEIPT_WRITE,
};

pub const API_KEY_HEADER: &str = "x-wms-api-key";
pub const API_KEY_WAREHOUSE_HEADER: &str = "x-wms-warehouse-id";

#[derive(Clone)]
pub struct ApiKeyAuthState {
    service: ApiKeyService,
}

impl ApiKeyAuthState {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            service: ApiKeyService::new(pool),
        }
    }
}

pub async fn api_key_auth_middleware(
    State(state): State<ApiKeyAuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(raw_key) = request
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return next.run(request).await;
    };

    let path = request.uri().path().to_string();
    let Some(scope) = required_scope(&path) else {
        // M5 冷链等历史外部入口仍由各自防腐层校验；统一 Key 只接管已声明的路由。
        return next.run(request).await;
    };
    let ip = client_ip(request.headers());
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let context = match state
        .service
        .authenticate_any_owner(raw_key, scope, ip.as_deref(), user_agent.as_deref())
        .await
    {
        Ok(context) => context,
        Err(error) => return api_key_error_response(error),
    };
    let warehouse_scope =
        match warehouse_scope_for_request(&path, request.headers(), &context.warehouse_ids) {
            Ok(scope) => scope,
            Err(()) => return warehouse_scope_response(),
        };
    let auth_context = AuthContext {
        user_id: context.key_id,
        owner_id: context.owner_id,
        actor_name: format!("API Key / {}", context.caller_name),
        permissions: permissions_for_scope(scope),
        jti: format!("api-key:{}", context.key_id),
        warehouse_scope,
    };
    request.extensions_mut().insert(auth_context);

    let method = request.method().to_string();
    let response = next.run(request).await;
    if let Err(error) = state
        .service
        .record_request_audit(
            &context,
            &method,
            &path,
            response.status().as_u16(),
            ip.as_deref(),
            user_agent.as_deref(),
        )
        .await
    {
        tracing::error!(?error, path, "API Key 调用审计写入失败");
    }
    response
}

pub fn required_scope(path: &str) -> Option<&'static str> {
    if path.starts_with("/api/v1/master-data/") || path.starts_with("/api/v1/system-dictionaries/")
    {
        Some("master-data:write")
    } else if path == "/api/v1/inbound/receiving-orders" {
        // 外部 ERP 仅允许 ASN 推送创建，禁止用同一 Key 访问收货/验收/签字/上架等作业接口。
        Some("inbound:push")
    } else if let Some(message_type) =
        path.strip_prefix("/api/v1/integration/erp-messages/inbound/")
    {
        inbound_scope_for_catalog_type(message_type)
    } else if path.starts_with("/api/v1/integration/erp-messages/") && path.ends_with("/receipt") {
        Some("outbound:receipt")
    } else if path.starts_with("/api/v1/tms/") || path.starts_with("/api/v1/traceability/") {
        Some("tms:callback")
    } else {
        None
    }
}

fn warehouse_scope_for_request(
    path: &str,
    headers: &HeaderMap,
    allowed_warehouses: &[Uuid],
) -> Result<Option<Uuid>, ()> {
    let message_type = path.strip_prefix("/api/v1/integration/erp-messages/inbound/");
    if matches!(message_type, Some("product_master" | "product_change")) {
        return if allowed_warehouses.is_empty() && !headers.contains_key(API_KEY_WAREHOUSE_HEADER) {
            Ok(None)
        } else {
            Err(())
        };
    }
    let h8_inbound = message_type
        .and_then(inbound_scope_for_catalog_type)
        .is_some();
    if allowed_warehouses.is_empty() && !h8_inbound {
        return Ok(None);
    }
    let warehouse_id = headers
        .get(API_KEY_WAREHOUSE_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
        .ok_or(())?;
    if !allowed_warehouses.is_empty() && !allowed_warehouses.contains(&warehouse_id) {
        return Err(());
    }
    Ok(Some(warehouse_id))
}

fn permissions_for_scope(scope: &str) -> Vec<String> {
    let permissions = match scope {
        "master-data:write" => [
            "m1.master_data.read",
            "m1.master_data.write",
            "m1.system_dictionary.read",
            "m1.system_dictionary.write",
        ]
        .as_slice(),
        "inbound:push" => ["m2.write"].as_slice(),
        "outbound:push" => ["m4.write"].as_slice(),
        "outbound:receipt" => [H8_RECEIPT_WRITE].as_slice(),
        "return:push" => ["m2.write"].as_slice(),
        "tms:callback" => ["m10.write", "m5.write", "m-tc.write"].as_slice(),
        _ => &[],
    };
    permissions
        .iter()
        .map(|value| (*value).to_string())
        .collect()
}

fn client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    ["x-forwarded-for", "x-real-ip"].iter().find_map(|name| {
        headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.split(',').next().unwrap_or(value).trim().to_string())
    })
}

fn warehouse_scope_response() -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        "H1_APIKEY_WAREHOUSE_SCOPE_DENIED",
        "API Key 未授权该仓库范围",
    )
}

fn api_key_error_response(error: ApiKeyAuthError) -> Response {
    match error {
        ApiKeyAuthError::Invalid | ApiKeyAuthError::Expired => error_response(
            StatusCode::UNAUTHORIZED,
            "H1_APIKEY_INVALID",
            "API Key 无效或已过期",
        ),
        ApiKeyAuthError::TemporarilyDisabled => error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "H1_APIKEY_TEMPORARILY_DISABLED",
            "API Key 已临时禁用",
        ),
        ApiKeyAuthError::RateLimited => error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "H1_APIKEY_RATE_LIMITED",
            "API Key 请求超过限流阈值",
        ),
        ApiKeyAuthError::InvalidScope => error_response(
            StatusCode::FORBIDDEN,
            "H1_APIKEY_SCOPE_DENIED",
            "API Key 不包含接口所需作用域",
        ),
        ApiKeyAuthError::CrossOwner => {
            error_response(StatusCode::FORBIDDEN, "AUTH-006", "跨货主越权")
        }
        ApiKeyAuthError::Database => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "H1_APIKEY_UNAVAILABLE",
            "API Key 鉴权暂不可用",
        ),
    }
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    let mut response = (
        status,
        Json(ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            severity: "error".to_string(),
            details: serde_json::json!({}),
            trace_id: Uuid::new_v4().to_string(),
            retry_hint: None,
        }),
    )
        .into_response();
    if status == StatusCode::TOO_MANY_REQUESTS {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
    }
    response
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use uuid::Uuid;

    use super::{permissions_for_scope, required_scope};

    #[test]
    fn maps_external_paths_to_declared_scopes() {
        assert_eq!(
            required_scope("/api/v1/master-data/products"),
            Some("master-data:write")
        );
        assert_eq!(
            required_scope("/api/v1/inbound/receiving-orders"),
            Some("inbound:push")
        );
        assert_eq!(
            required_scope("/api/v1/inbound/receiving-orders/abc/receive"),
            None
        );
        assert_eq!(required_scope("/api/v1/inbound/receiving-dashboard"), None);
        assert_eq!(
            required_scope("/api/v1/tms/dispatches"),
            Some("tms:callback")
        );
        assert_eq!(
            required_scope(
                "/api/v1/integration/erp-messages/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee/receipt"
            ),
            Some("outbound:receipt")
        );
        for (message_type, scope) in [
            ("asn", "inbound:push"),
            ("outbound_order", "outbound:push"),
            ("return_order", "return:push"),
            ("product_master", "master-data:write"),
            ("product_change", "master-data:write"),
        ] {
            assert_eq!(
                required_scope(&format!(
                    "/api/v1/integration/erp-messages/inbound/{message_type}"
                )),
                Some(scope)
            );
        }
        assert_eq!(
            required_scope("/api/v1/integration/erp-messages/inbound/unknown"),
            None
        );
        assert_eq!(required_scope("/api/v1/auth/api-keys"), None);
    }

    #[test]
    fn maps_scope_to_handler_permissions_without_admin_access() {
        let permissions = permissions_for_scope("inbound:push");
        assert!(permissions.iter().any(|value| value == "m2.write"));
        assert!(!permissions.iter().any(|value| value == "m2.read"));
        assert!(!permissions
            .iter()
            .any(|value| value == "h1.api_keys.manage"));
        assert!(!permissions
            .iter()
            .any(|value| value == "h8.erp_receipt.write"));
        assert!(!permissions
            .iter()
            .any(|value| value == "h8.erp_connector.write"));

        assert_eq!(
            permissions_for_scope("outbound:push"),
            vec!["m4.write".to_string()]
        );
        assert_eq!(
            permissions_for_scope("outbound:receipt"),
            vec!["h8.erp_receipt.write".to_string()]
        );
        assert_eq!(
            permissions_for_scope("return:push"),
            vec!["m2.write".to_string()]
        );
    }

    #[test]
    fn h8_inbound_binds_warehouse_only_for_warehouse_scoped_messages() {
        let path = "/api/v1/integration/erp-messages/inbound/outbound_order";
        let warehouse_id = Uuid::new_v4();
        let mut headers = HeaderMap::new();
        assert!(super::warehouse_scope_for_request(path, &headers, &[]).is_err());

        headers.insert(
            super::API_KEY_WAREHOUSE_HEADER,
            HeaderValue::from_str(&warehouse_id.to_string()).expect("warehouse header"),
        );
        assert_eq!(
            super::warehouse_scope_for_request(path, &headers, &[]),
            Ok(Some(warehouse_id))
        );
        assert_eq!(
            super::warehouse_scope_for_request(path, &headers, &[warehouse_id]),
            Ok(Some(warehouse_id))
        );
        assert!(super::warehouse_scope_for_request(path, &headers, &[Uuid::new_v4()]).is_err());

        let owner_level = "/api/v1/integration/erp-messages/inbound/product_master";
        assert_eq!(
            super::warehouse_scope_for_request(owner_level, &HeaderMap::new(), &[]),
            Ok(None)
        );
        assert!(super::warehouse_scope_for_request(
            owner_level,
            &HeaderMap::new(),
            &[warehouse_id]
        )
        .is_err());
        assert!(super::warehouse_scope_for_request(owner_level, &headers, &[]).is_err());
        assert_eq!(
            super::warehouse_scope_for_request("/api/v1/tms/dispatches", &HeaderMap::new(), &[]),
            Ok(None)
        );
    }
}
