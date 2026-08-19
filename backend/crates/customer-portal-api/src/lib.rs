pub mod auth;
pub mod download;
pub mod export;
pub mod models;
pub mod projection;
pub mod query;
pub mod users;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct PortalState {
    pub pool: PgPool,
    pub jwt_secret: Arc<str>,
    pub projection_key: Arc<str>,
    pub storage_root: Arc<PathBuf>,
}

impl PortalState {
    pub fn new(
        pool: PgPool,
        jwt_secret: impl Into<Arc<str>>,
        projection_key: impl Into<Arc<str>>,
        storage_root: PathBuf,
    ) -> Self {
        Self {
            pool,
            jwt_secret: jwt_secret.into(),
            projection_key: projection_key.into(),
            storage_root: Arc::new(storage_root),
        }
    }
}

pub fn portal_router(state: PortalState) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/api/v1/auth/login", post(auth::login))
        .route(
            "/api/v1/internal/projections",
            post(projection::ingest_projection),
        )
        .route("/api/v1/addresses", get(query::list_addresses))
        .route("/api/v1/orders", get(query::list_orders))
        .route("/api/v1/orders/:order_id", get(query::get_order))
        .route(
            "/api/v1/report-versions/:report_version_id/download",
            post(download::create_report_download),
        )
        .route("/api/v1/files/:token", get(download::serve_download))
        .route(
            "/api/v1/exports",
            post(export::create_export).get(export::list_exports),
        )
        .route(
            "/api/v1/exports/:export_id/download",
            post(download::create_export_download),
        )
        .route(
            "/api/v1/users",
            post(users::create_user).get(users::list_users),
        )
        .route("/api/v1/users/:user_id", put(users::update_user))
        .with_state(state)
}

#[derive(Debug)]
pub enum PortalError {
    Unauthorized,
    Forbidden,
    NotFound,
    Validation(String),
    Conflict(String),
    Database(sqlx::Error),
    Json(serde_json::Error),
    Internal(String),
}

impl std::fmt::Display for PortalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(formatter, "认证失败"),
            Self::Forbidden => write!(formatter, "无权访问"),
            Self::NotFound => write!(formatter, "资源不存在"),
            Self::Validation(message) | Self::Conflict(message) | Self::Internal(message) => {
                write!(formatter, "{message}")
            }
            Self::Database(_) => write!(formatter, "数据库操作失败"),
            Self::Json(_) => write!(formatter, "事件数据格式错误"),
        }
    }
}

impl std::error::Error for PortalError {}

impl From<sqlx::Error> for PortalError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<serde_json::Error> for PortalError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl IntoResponse for PortalError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "认证失败".to_string(),
            ),
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "无权访问".to_string()),
            Self::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", "资源不存在".to_string()),
            Self::Validation(message) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "CONFLICT", message),
            Self::Database(error) => {
                tracing::error!(error = %error, "customer portal database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "数据库操作失败".to_string(),
                )
            }
            Self::Json(error) => (
                StatusCode::BAD_REQUEST,
                "INVALID_PROJECTION_PAYLOAD",
                format!("事件数据格式错误：{error}"),
            ),
            Self::Internal(error) => {
                tracing::error!(error = %error, "customer portal internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "服务内部错误".to_string(),
                )
            }
        };
        (status, Json(json!({ "code": code, "message": message }))).into_response()
    }
}

pub async fn audit(
    pool: &PgPool,
    user_id: Option<Uuid>,
    customer_id: Option<Uuid>,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    detail: Value,
) -> Result<(), PortalError> {
    sqlx::query(
        "INSERT INTO portal_audit_events (
            id, occurred_at, user_id, customer_id, action,
            resource_type, resource_id, detail
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(Utc::now())
    .bind(user_id)
    .bind(customer_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn resolve_storage_key(root: &Path, storage_key: &str) -> Result<PathBuf, PortalError> {
    let key_path = Path::new(storage_key);
    if storage_key.trim().is_empty()
        || key_path.is_absolute()
        || key_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(PortalError::Validation("非法的文件存储键".to_string()));
    }
    Ok(root.join(key_path))
}

pub(crate) fn report_download_file_name(
    product_name: &str,
    product_code: &str,
    batch_no: &str,
    version_number: i32,
) -> String {
    let safe = |value: &str| {
        value
            .chars()
            .map(|character| {
                if character.is_control()
                    || matches!(
                        character,
                        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                    )
                {
                    '_'
                } else {
                    character
                }
            })
            .collect::<String>()
    };
    format!(
        "{}_{}_{}_药检单_V{version_number}.pdf",
        safe(product_name),
        safe(product_code),
        safe(batch_no)
    )
}

#[cfg(test)]
mod tests {
    use super::{report_download_file_name, resolve_storage_key};
    use std::path::Path;

    #[test]
    fn report_download_name_contains_drug_product_batch_and_version() {
        assert_eq!(
            report_download_file_name("阿莫/西林", "AM:001", "B\\2026", 2),
            "阿莫_西林_AM_001_B_2026_药检单_V2.pdf"
        );
    }

    #[test]
    fn storage_key_must_stay_below_root() {
        assert!(resolve_storage_key(Path::new("/tmp/portal"), "reports/a.pdf").is_ok());
        assert!(resolve_storage_key(Path::new("/tmp/portal"), "../secret").is_err());
        assert!(resolve_storage_key(Path::new("/tmp/portal"), "/etc/passwd").is_err());
    }
}
