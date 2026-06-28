//! Runtime Axum handlers for US-M1-011 system dictionary.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use wms_domain::{
    DisableSystemDictionaryItemRequest, ErrorResponse, PageMeta, SystemDictionaryImpactPreview,
    SystemDictionaryItem, SystemDictionaryItemListResponse, UpsertSystemDictionaryItemRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    system_dictionary::{PgSystemDictionaryRepository, SystemDictionaryError},
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "m1.system_dictionary.read";
const WRITE_PERMISSION: &str = "m1.system_dictionary.write";
const GLOBAL_WRITE_PERMISSION: &str = "m1.system_dictionary.global.write";

#[derive(Clone, Debug)]
pub struct SystemDictionaryAppState {
    repository: Arc<PgSystemDictionaryRepository>,
}

#[derive(Debug, Deserialize)]
struct ListSystemDictionaryQuery {
    effective_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct PreviewSystemDictionaryImpactQuery {
    owner_id: Option<uuid::Uuid>,
    effective_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SystemDictionaryHandlerError {
    Auth(AuthError),
    SystemDictionary(SystemDictionaryError),
    MissingIdempotencyKey,
}

impl SystemDictionaryAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgSystemDictionaryRepository::new(pool)),
        }
    }
}

impl From<AuthError> for SystemDictionaryHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<SystemDictionaryError> for SystemDictionaryHandlerError {
    fn from(value: SystemDictionaryError) -> Self {
        Self::SystemDictionary(value)
    }
}

impl IntoResponse for SystemDictionaryHandlerError {
    fn into_response(self) -> Response {
        if let SystemDictionaryHandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            SystemDictionaryHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M1_SYSTEM_DICTIONARY_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            SystemDictionaryHandlerError::SystemDictionary(SystemDictionaryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M1_SYSTEM_DICTIONARY_NOT_FOUND",
                "系统字典或字典项不存在",
            ),
            SystemDictionaryHandlerError::SystemDictionary(
                SystemDictionaryError::CrossOwnerAccess,
            ) => (
                StatusCode::FORBIDDEN,
                "M1_SYSTEM_DICTIONARY_CROSS_OWNER",
                "跨货主字典配置越权",
            ),
            SystemDictionaryHandlerError::SystemDictionary(
                SystemDictionaryError::IdempotencyConflict,
            ) => (
                StatusCode::CONFLICT,
                "M1_SYSTEM_DICTIONARY_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            SystemDictionaryHandlerError::SystemDictionary(
                SystemDictionaryError::InvalidScope
                | SystemDictionaryError::InvalidEffectiveWindow
                | SystemDictionaryError::ParamInvalid { .. },
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_SYSTEM_DICTIONARY_INVALID",
                "系统字典参数或作用域非法",
            ),
            SystemDictionaryHandlerError::SystemDictionary(
                SystemDictionaryError::Audit(_)
                | SystemDictionaryError::Database(_)
                | SystemDictionaryError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1_SYSTEM_DICTIONARY_INTERNAL",
                "系统字典处理失败",
            ),
            SystemDictionaryHandlerError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn system_dictionary_router(state: SystemDictionaryAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/system-dictionaries/:dict_code/items",
            get(list_system_dictionary_items_handler),
        )
        .route(
            "/api/v1/system-dictionaries/:dict_code/items/:item_code",
            put(upsert_system_dictionary_item_handler),
        )
        .route(
            "/api/v1/system-dictionaries/:dict_code/items/:item_code/impact-preview",
            get(preview_system_dictionary_impact_handler),
        )
        .route(
            "/api/v1/system-dictionaries/:dict_code/items/:item_code/disable",
            patch(disable_system_dictionary_item_handler),
        )
        .with_state(state)
}

async fn list_system_dictionary_items_handler(
    ctx: AuthContext,
    State(state): State<SystemDictionaryAppState>,
    Path(dict_code): Path<String>,
    Query(query): Query<ListSystemDictionaryQuery>,
) -> Result<Json<SystemDictionaryItemListResponse>, SystemDictionaryHandlerError> {
    require_any_permission(&ctx, &[READ_PERMISSION, WRITE_PERMISSION])?;
    let items = state
        .repository
        .list_effective_items(
            &ctx,
            &dict_code,
            query.effective_at.unwrap_or_else(Utc::now),
        )
        .await?;
    Ok(Json(item_list_response(items)))
}

async fn preview_system_dictionary_impact_handler(
    ctx: AuthContext,
    State(state): State<SystemDictionaryAppState>,
    Path((dict_code, item_code)): Path<(String, String)>,
    Query(query): Query<PreviewSystemDictionaryImpactQuery>,
) -> Result<Json<SystemDictionaryImpactPreview>, SystemDictionaryHandlerError> {
    require_any_permission(&ctx, &[READ_PERMISSION, WRITE_PERMISSION])?;
    let preview = state
        .repository
        .preview_item_impact(
            &ctx,
            &dict_code,
            &item_code,
            query.owner_id.unwrap_or(ctx.owner_id),
            query.effective_at.unwrap_or_else(Utc::now),
        )
        .await?;
    Ok(Json(preview))
}

async fn upsert_system_dictionary_item_handler(
    ctx: AuthContext,
    State(state): State<SystemDictionaryAppState>,
    Path((dict_code, item_code)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<UpsertSystemDictionaryItemRequest>,
) -> Result<Json<SystemDictionaryItem>, SystemDictionaryHandlerError> {
    require_write_permission(&ctx, req.owner_id)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .repository
        .upsert_item(
            &ctx,
            &dict_code,
            &item_code,
            req,
            Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(result.value))
}

async fn disable_system_dictionary_item_handler(
    ctx: AuthContext,
    State(state): State<SystemDictionaryAppState>,
    Path((dict_code, item_code)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<DisableSystemDictionaryItemRequest>,
) -> Result<Json<SystemDictionaryItem>, SystemDictionaryHandlerError> {
    require_write_permission(&ctx, req.owner_id)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .repository
        .disable_item(
            &ctx,
            &dict_code,
            &item_code,
            req,
            Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(result.value))
}

fn require_any_permission(ctx: &AuthContext, permissions: &[&str]) -> Result<(), AuthError> {
    if permissions
        .iter()
        .any(|permission| ctx.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(permissions.join("|")))
    }
}

fn require_write_permission(
    ctx: &AuthContext,
    owner_id: Option<uuid::Uuid>,
) -> Result<(), AuthError> {
    if owner_id.is_some() {
        ctx.require_permission(WRITE_PERMISSION)
    } else {
        ctx.require_permission(GLOBAL_WRITE_PERMISSION)
    }
}

fn idempotency_key_from_headers(
    headers: &HeaderMap,
) -> Result<String, SystemDictionaryHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(SystemDictionaryHandlerError::MissingIdempotencyKey)
}

fn item_list_response(items: Vec<SystemDictionaryItem>) -> SystemDictionaryItemListResponse {
    SystemDictionaryItemListResponse {
        page: PageMeta {
            next_cursor: None,
            count: items.len() as u32,
        },
        data: items,
    }
}

#[cfg(test)]
mod tests {
    use axum::{extract::Path, http::HeaderMap, Json};
    use serde_json::json;
    use sqlx::PgPool;
    use uuid::Uuid;
    use wms_domain::UpsertSystemDictionaryItemRequest;

    use super::{
        idempotency_key_from_headers, preview_system_dictionary_impact_handler,
        upsert_system_dictionary_item_handler, PreviewSystemDictionaryImpactQuery,
        SystemDictionaryAppState, SystemDictionaryHandlerError, GLOBAL_WRITE_PERMISSION,
        READ_PERMISSION, WRITE_PERMISSION,
    };
    use crate::auth::{AuthContext, AuthError};

    fn ctx(permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "system-dictionary-handler-test".to_string(),
            permissions: permissions.iter().map(|item| item.to_string()).collect(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    fn request(owner_id: Option<Uuid>) -> UpsertSystemDictionaryItemRequest {
        UpsertSystemDictionaryItemRequest {
            owner_id,
            item_name: "采购入库".to_string(),
            enabled: true,
            params: json!({
                "direction": "inbound",
                "workflow_template": "purchase_inbound",
                "batch_policy": "standard_batch"
            }),
            effective_from: None,
            effective_to: None,
        }
    }

    #[test]
    fn idempotency_header_requires_non_empty_value() {
        assert_eq!(
            idempotency_key_from_headers(&HeaderMap::new()),
            Err(SystemDictionaryHandlerError::MissingIdempotencyKey)
        );

        let mut headers = HeaderMap::new();
        headers.insert("idempotency-key", "dict-1".parse().expect("valid header"));
        assert_eq!(
            idempotency_key_from_headers(&headers).expect("idempotency key"),
            "dict-1"
        );
    }

    #[tokio::test]
    async fn write_handler_checks_permission_before_idempotency_or_database() {
        let pool =
            PgPool::connect_lazy("postgres://localhost/wms").expect("lazy pool should not connect");
        let error = upsert_system_dictionary_item_handler(
            ctx(&[]),
            axum::extract::State(SystemDictionaryAppState::with_postgres(pool)),
            Path(("document_type".to_string(), "purchase_inbound".to_string())),
            HeaderMap::new(),
            Json(request(Some(Uuid::new_v4()))),
        )
        .await
        .expect_err("permission should be required");

        assert!(matches!(
            error,
            SystemDictionaryHandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == WRITE_PERMISSION
        ));
    }

    #[tokio::test]
    async fn preview_handler_checks_permission_before_database() {
        let pool =
            PgPool::connect_lazy("postgres://localhost/wms").expect("lazy pool should not connect");
        let error = preview_system_dictionary_impact_handler(
            ctx(&[]),
            axum::extract::State(SystemDictionaryAppState::with_postgres(pool)),
            Path(("document_type".to_string(), "purchase_inbound".to_string())),
            axum::extract::Query(PreviewSystemDictionaryImpactQuery {
                owner_id: None,
                effective_at: None,
            }),
        )
        .await
        .expect_err("read permission should be required");

        assert!(matches!(
            error,
            SystemDictionaryHandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == format!("{READ_PERMISSION}|{WRITE_PERMISSION}")
        ));
    }

    #[tokio::test]
    async fn write_handler_requires_idempotency_before_database() {
        let pool =
            PgPool::connect_lazy("postgres://localhost/wms").expect("lazy pool should not connect");
        let owner_id = Uuid::new_v4();
        let error = upsert_system_dictionary_item_handler(
            AuthContext {
                owner_id,
                ..ctx(&[WRITE_PERMISSION])
            },
            axum::extract::State(SystemDictionaryAppState::with_postgres(pool)),
            Path(("document_type".to_string(), "purchase_inbound".to_string())),
            HeaderMap::new(),
            Json(request(Some(owner_id))),
        )
        .await
        .expect_err("idempotency key should be required");

        assert_eq!(error, SystemDictionaryHandlerError::MissingIdempotencyKey);
    }

    #[tokio::test]
    async fn global_write_handler_requires_global_permission() {
        let pool =
            PgPool::connect_lazy("postgres://localhost/wms").expect("lazy pool should not connect");
        let error = upsert_system_dictionary_item_handler(
            ctx(&[WRITE_PERMISSION]),
            axum::extract::State(SystemDictionaryAppState::with_postgres(pool)),
            Path(("document_type".to_string(), "purchase_inbound".to_string())),
            HeaderMap::new(),
            Json(request(None)),
        )
        .await
        .expect_err("global write should require global permission");

        assert!(matches!(
            error,
            SystemDictionaryHandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == GLOBAL_WRITE_PERMISSION
        ));
    }
}
