//! Wave 1 W1.C 主仓 OpenAPI 契约骨架。

pub mod audit;
pub mod auth;
pub mod feature_flags;

use utoipa::OpenApi;
use wms_domain::{
    AuditActor, AuditEvent, AuditEventListResponse, CurrentUser, ErrorResponse, HealthzResponse,
    LoginRequest, LoginResponse,
};

#[utoipa::path(
    get,
    path = "/api/v1/healthz",
    tag = "system",
    responses(
        (status = 200, description = "服务健康", body = HealthzResponse),
    ),
)]
#[allow(dead_code)]
fn healthz() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn login() {}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "当前登录用户", body = CurrentUser),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn me() {}

#[utoipa::path(
    get,
    path = "/api/v1/audit/events",
    tag = "audit",
    params(
        ("resource_type" = Option<String>, Query, description = "按资源类型过滤"),
        ("actor_id" = Option<uuid::Uuid>, Query, description = "按操作者过滤"),
        ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "开始时间（RFC3339）"),
        ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "结束时间（RFC3339）"),
        ("limit" = Option<u32>, Query, description = "每页条数"),
        ("cursor" = Option<String>, Query, description = "分页游标"),
    ),
    responses(
        (status = 200, description = "审计事件列表", body = AuditEventListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn list_audit_events() {}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "WMS API",
        version = "0.0.1-wave-1-skeleton",
        description = "Wave 1 W1.C 主仓 OpenAPI 契约骨架",
    ),
    paths(healthz, login, me, list_audit_events),
    components(schemas(
        AuditActor,
        AuditEvent,
        AuditEventListResponse,
        CurrentUser,
        ErrorResponse,
        HealthzResponse,
        LoginRequest,
        LoginResponse,
    )),
    tags(
        (name = "system", description = "系统探针"),
        (name = "auth", description = "鉴权与会话"),
        (name = "audit", description = "审计追踪"),
    ),
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::ApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn openapi_contains_wave1_contract_paths() {
        let json = ApiDoc::openapi()
            .to_pretty_json()
            .expect("openapi json should serialize");

        for required_path in [
            "/api/v1/healthz",
            "/api/v1/auth/login",
            "/api/v1/auth/me",
            "/api/v1/audit/events",
        ] {
            assert!(
                json.contains(required_path),
                "missing required path: {required_path}"
            );
        }

        for required_schema in [
            "\"ErrorResponse\"",
            "\"LoginRequest\"",
            "\"LoginResponse\"",
            "\"CurrentUser\"",
            "\"AuditEvent\"",
        ] {
            assert!(
                json.contains(required_schema),
                "missing required schema: {required_schema}"
            );
        }
    }
}
