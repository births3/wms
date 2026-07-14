//! H1 外部 API Key 统一入口。

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;
use wms_domain::ErrorResponse;

use crate::{
    api_key_service::{ApiKeyAuthError, ApiKeyService},
    auth::AuthContext,
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
    if !context.warehouse_ids.is_empty() {
        let Some(warehouse_id) = request
            .headers()
            .get(API_KEY_WAREHOUSE_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<Uuid>().ok())
        else {
            return warehouse_scope_response();
        };
        if !context.warehouse_ids.contains(&warehouse_id) {
            return warehouse_scope_response();
        }
    }
    let auth_context = AuthContext {
        user_id: context.key_id,
        owner_id: context.owner_id,
        actor_name: format!("API Key / {}", context.caller_name),
        permissions: permissions_for_scope(scope),
        jti: format!("api-key:{}", context.key_id),
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
    } else if path.starts_with("/api/v1/inbound/") {
        Some("inbound:push")
    } else if path.starts_with("/api/v1/tms/") || path.starts_with("/api/v1/traceability/") {
        Some("tms:callback")
    } else {
        None
    }
}

fn permissions_for_scope(scope: &str) -> Vec<String> {
    let permissions = match scope {
        "master-data:write" => [
            "m1.master_data.write",
            "m1.system_dictionary.read",
            "m1.system_dictionary.write",
        ]
        .as_slice(),
        "inbound:push" => ["m2.read", "m2.write"].as_slice(),
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
            required_scope("/api/v1/tms/dispatches"),
            Some("tms:callback")
        );
        assert_eq!(required_scope("/api/v1/auth/api-keys"), None);
    }

    #[test]
    fn maps_scope_to_handler_permissions_without_admin_access() {
        let permissions = permissions_for_scope("inbound:push");
        assert!(permissions.iter().any(|value| value == "m2.write"));
        assert!(!permissions
            .iter()
            .any(|value| value == "h1.api_keys.manage"));
    }
}
