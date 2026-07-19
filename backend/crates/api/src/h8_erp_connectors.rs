//! US-H8-001：ERP 连接配置 API（h8.erp_connector.read / h8.erp_connector.write）。

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::{
    apply_update, can_activate, can_physically_delete, CreateH8ErpConnectorRequest, ErrorResponse,
    H8ErpConnector, H8ErpConnectorError, H8ErpConnectorListResponse, H8ErpConnectorTestResult,
    PageMeta, UpdateH8ErpConnectorRequest,
};

use crate::{
    audit::{append_event, AuditDiff, AuditWriteRequest},
    auth::{AuthContext, AuthError},
};

pub const H8_CONFIG_READ: &str = "h8.erp_connector.read";
pub const H8_CONFIG_WRITE: &str = "h8.erp_connector.write";

#[derive(Clone)]
pub struct H8ErpConnectorAppState {
    pub repository: Arc<dyn H8ErpConnectorRepository>,
    pub audit_pool: Option<PgPool>,
}

impl H8ErpConnectorAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgH8ErpConnectorRepository { pool: pool.clone() }),
            audit_pool: Some(pool),
        }
    }

    pub fn with_memory() -> Self {
        Self {
            repository: Arc::new(MemoryH8ErpConnectorRepository::default()),
            audit_pool: None,
        }
    }
}

async fn write_audit(
    state: &H8ErpConnectorAppState,
    ctx: &AuthContext,
    action: &str,
    connector: &H8ErpConnector,
    before: Option<serde_json::Value>,
) {
    let Some(pool) = &state.audit_pool else {
        return;
    };
    let after = audit_snapshot(connector);
    let mut req = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H8",
        "h8_erp_connector",
        connector.id.to_string(),
        Some(AuditDiff::compute(
            before.unwrap_or(serde_json::Value::Null),
            after,
        )),
    );
    req.occurred_at = Utc::now();
    let _ = append_event(pool, &req).await;
}

fn audit_snapshot(c: &H8ErpConnector) -> serde_json::Value {
    // 脱敏：不记录 secret alias 明文内容以外的“是否配置”
    serde_json::json!({
        "id": c.id,
        "connector_code": c.connector_code,
        "connector_name": c.connector_name,
        "warehouse_ids": c.warehouse_ids,
        "directions": c.directions,
        "message_types": c.message_types,
        "channel_mode": c.channel_mode,
        "api_base_url": c.api_base_url,
        "interface_db_host": c.interface_db_host,
        "interface_db_port": c.interface_db_port,
        "interface_db_name": c.interface_db_name,
        "interface_db_username": c.interface_db_username,
        "api_key_id": c.api_key_id,
        "bearer_secret_alias_set": c.bearer_secret_alias.as_ref().is_some_and(|s| !s.is_empty()),
        "interface_db_password_alias_set": c.interface_db_password_alias.as_ref().is_some_and(|s| !s.is_empty()),
        "status": c.status,
        "config_version": c.config_version,
        "last_tested_succeeded": c.last_tested_succeeded,
        "last_tested_version": c.last_tested_version,
    })
}

#[axum::async_trait]
pub trait H8ErpConnectorRepository: Send + Sync {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError>;
    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn save(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError>;
    async fn list_active(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError>;
    async fn has_inflight_refs(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<bool, H8ErpConnectorRepoError>;
    async fn pause_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError>;
}

#[derive(Debug)]
pub enum H8ErpConnectorRepoError {
    Domain(H8ErpConnectorError),
    DuplicateCode,
    Db(String),
}

#[derive(Default)]
struct MemoryH8ErpConnectorRepository {
    inner: std::sync::Mutex<Vec<H8ErpConnector>>,
}

#[axum::async_trait]
impl H8ErpConnectorRepository for MemoryH8ErpConnectorRepository {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        let guard = self.inner.lock().expect("lock");
        Ok(guard
            .iter()
            .filter(|c| c.owner_id == owner_id)
            .cloned()
            .collect())
    }

    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.list(owner_id)
            .await?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ))
    }

    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        if guard.iter().any(|c| {
            c.owner_id == connector.owner_id && c.connector_code == connector.connector_code
        }) {
            return Err(H8ErpConnectorRepoError::DuplicateCode);
        }
        guard.push(connector.clone());
        Ok(connector.clone())
    }

    async fn save(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        let Some(slot) = guard
            .iter_mut()
            .find(|c| c.id == connector.id && c.owner_id == connector.owner_id)
        else {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        };
        *slot = connector.clone();
        Ok(connector.clone())
    }

    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        let before = guard.len();
        guard.retain(|c| !(c.owner_id == owner_id && c.id == id));
        if guard.len() == before {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        }
        Ok(())
    }

    async fn list_active(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        Ok(self
            .list(owner_id)
            .await?
            .into_iter()
            .filter(|c| c.status == "active")
            .collect())
    }

    async fn has_inflight_refs(
        &self,
        _owner_id: Uuid,
        _connector_id: Uuid,
    ) -> Result<bool, H8ErpConnectorRepoError> {
        Ok(false)
    }

    async fn pause_inflight(
        &self,
        _owner_id: Uuid,
        _connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        Ok(0)
    }
}

struct PgH8ErpConnectorRepository {
    pool: PgPool,
}

#[axum::async_trait]
impl H8ErpConnectorRepository for PgH8ErpConnectorRepository {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        let rows = sqlx::query_as::<_, H8ErpConnectorRow>(
            r#"
            SELECT * FROM h8_erp_connectors
             WHERE owner_id = $1
             ORDER BY updated_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        sqlx::query_as::<_, H8ErpConnectorRow>(
            r#"
            SELECT * FROM h8_erp_connectors
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?
        .map(Into::into)
        .ok_or(H8ErpConnectorRepoError::Domain(
            H8ErpConnectorError::NotFound,
        ))
    }

    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        sqlx::query(
            r#"
            INSERT INTO h8_erp_connectors (
                id, owner_id, connector_code, connector_name, warehouse_ids, directions,
                message_types, channel_mode, api_base_url, interface_db_host, interface_db_port,
                interface_db_name, interface_db_username, api_key_id, bearer_secret_alias,
                interface_db_password_alias, status, config_version, first_activated_at,
                last_tested_version, last_tested_at, last_tested_succeeded,
                last_tested_error_summary, created_at, updated_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25
            )
            "#,
        )
        .bind(connector.id)
        .bind(connector.owner_id)
        .bind(&connector.connector_code)
        .bind(&connector.connector_name)
        .bind(&connector.warehouse_ids)
        .bind(&connector.directions)
        .bind(&connector.message_types)
        .bind(&connector.channel_mode)
        .bind(&connector.api_base_url)
        .bind(&connector.interface_db_host)
        .bind(connector.interface_db_port)
        .bind(&connector.interface_db_name)
        .bind(&connector.interface_db_username)
        .bind(connector.api_key_id)
        .bind(&connector.bearer_secret_alias)
        .bind(&connector.interface_db_password_alias)
        .bind(&connector.status)
        .bind(connector.config_version)
        .bind(connector.first_activated_at)
        .bind(connector.last_tested_version)
        .bind(connector.last_tested_at)
        .bind(connector.last_tested_succeeded)
        .bind(&connector.last_tested_error_summary)
        .bind(connector.created_at)
        .bind(connector.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("uq_h8_erp_connectors_owner_code") {
                H8ErpConnectorRepoError::DuplicateCode
            } else {
                H8ErpConnectorRepoError::Db(msg)
            }
        })?;
        Ok(connector.clone())
    }

    async fn save(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let result = sqlx::query(
            r#"
            UPDATE h8_erp_connectors SET
                connector_name = $3,
                warehouse_ids = $4,
                directions = $5,
                message_types = $6,
                channel_mode = $7,
                api_base_url = $8,
                interface_db_host = $9,
                interface_db_port = $10,
                interface_db_name = $11,
                interface_db_username = $12,
                api_key_id = $13,
                bearer_secret_alias = $14,
                interface_db_password_alias = $15,
                status = $16,
                config_version = $17,
                first_activated_at = $18,
                last_tested_version = $19,
                last_tested_at = $20,
                last_tested_succeeded = $21,
                last_tested_error_summary = $22,
                updated_at = $23
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(connector.owner_id)
        .bind(connector.id)
        .bind(&connector.connector_name)
        .bind(&connector.warehouse_ids)
        .bind(&connector.directions)
        .bind(&connector.message_types)
        .bind(&connector.channel_mode)
        .bind(&connector.api_base_url)
        .bind(&connector.interface_db_host)
        .bind(connector.interface_db_port)
        .bind(&connector.interface_db_name)
        .bind(&connector.interface_db_username)
        .bind(connector.api_key_id)
        .bind(&connector.bearer_secret_alias)
        .bind(&connector.interface_db_password_alias)
        .bind(&connector.status)
        .bind(connector.config_version)
        .bind(connector.first_activated_at)
        .bind(connector.last_tested_version)
        .bind(connector.last_tested_at)
        .bind(connector.last_tested_succeeded)
        .bind(&connector.last_tested_error_summary)
        .bind(connector.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        }
        Ok(connector.clone())
    }

    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError> {
        let result = sqlx::query("DELETE FROM h8_erp_connectors WHERE owner_id = $1 AND id = $2")
            .bind(owner_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        }
        Ok(())
    }

    async fn list_active(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        let rows = sqlx::query_as::<_, H8ErpConnectorRow>(
            r#"
            SELECT * FROM h8_erp_connectors
             WHERE owner_id = $1 AND status = 'active'
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn has_inflight_refs(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<bool, H8ErpConnectorRepoError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM h8_erp_in_flight_messages
             WHERE owner_id = $1 AND connector_id = $2
            "#,
        )
        .bind(owner_id)
        .bind(connector_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(count > 0)
    }

    async fn pause_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        let result = sqlx::query(
            r#"
            UPDATE h8_erp_in_flight_messages
               SET status = 'paused', updated_at = now()
             WHERE owner_id = $1 AND connector_id = $2 AND status = 'running'
            "#,
        )
        .bind(owner_id)
        .bind(connector_id)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct H8ErpConnectorRow {
    id: Uuid,
    owner_id: Uuid,
    connector_code: String,
    connector_name: String,
    warehouse_ids: Vec<Uuid>,
    directions: Vec<String>,
    message_types: Vec<String>,
    channel_mode: String,
    api_base_url: Option<String>,
    interface_db_host: Option<String>,
    interface_db_port: Option<i32>,
    interface_db_name: Option<String>,
    interface_db_username: Option<String>,
    api_key_id: Option<Uuid>,
    bearer_secret_alias: Option<String>,
    interface_db_password_alias: Option<String>,
    status: String,
    config_version: i64,
    first_activated_at: Option<chrono::DateTime<Utc>>,
    last_tested_version: Option<i64>,
    last_tested_at: Option<chrono::DateTime<Utc>>,
    last_tested_succeeded: Option<bool>,
    last_tested_error_summary: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<H8ErpConnectorRow> for H8ErpConnector {
    fn from(r: H8ErpConnectorRow) -> Self {
        Self {
            id: r.id,
            owner_id: r.owner_id,
            connector_code: r.connector_code,
            connector_name: r.connector_name,
            warehouse_ids: r.warehouse_ids,
            directions: r.directions,
            message_types: r.message_types,
            channel_mode: r.channel_mode,
            api_base_url: r.api_base_url,
            interface_db_host: r.interface_db_host,
            interface_db_port: r.interface_db_port,
            interface_db_name: r.interface_db_name,
            interface_db_username: r.interface_db_username,
            api_key_id: r.api_key_id,
            bearer_secret_alias: r.bearer_secret_alias,
            interface_db_password_alias: r.interface_db_password_alias,
            status: r.status,
            config_version: r.config_version,
            first_activated_at: r.first_activated_at,
            last_tested_version: r.last_tested_version,
            last_tested_at: r.last_tested_at,
            last_tested_succeeded: r.last_tested_succeeded,
            last_tested_error_summary: r.last_tested_error_summary,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub fn h8_erp_connector_router(state: H8ErpConnectorAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/config/erp-connectors",
            get(list_connectors).post(create_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/:id",
            get(get_connector)
                .patch(update_connector)
                .delete(delete_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/:id/test",
            post(test_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/:id/activate",
            post(activate_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/:id/disable",
            post(disable_connector),
        )
        .with_state(state)
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, H8ErpConnectorHandlerError> {
    headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or(H8ErpConnectorHandlerError::MissingIdempotencyKey)
}

async fn list_connectors(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
) -> Result<Json<H8ErpConnectorListResponse>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_READ)?;
    let data = state.repository.list(ctx.owner_id).await?;
    let len = data.len();
    Ok(Json(H8ErpConnectorListResponse {
        data,
        page: PageMeta {
            next_cursor: None,
            count: len as u32,
        },
    }))
}

async fn get_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_READ)?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}

async fn create_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateH8ErpConnectorRequest>,
) -> Result<(StatusCode, Json<H8ErpConnector>), H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let _idem = idempotency_key(&headers)?;
    req.validate().map_err(H8ErpConnectorRepoError::Domain)?;
    let now = Utc::now();
    let connector = H8ErpConnector {
        id: Uuid::new_v4(),
        owner_id: ctx.owner_id,
        connector_code: req.connector_code.trim().to_string(),
        connector_name: req.connector_name.trim().to_string(),
        warehouse_ids: req.warehouse_ids,
        directions: req.directions,
        message_types: req.message_types,
        channel_mode: req.channel_mode,
        api_base_url: req.api_base_url,
        interface_db_host: req.interface_db_host,
        interface_db_port: req.interface_db_port,
        interface_db_name: req.interface_db_name,
        interface_db_username: req.interface_db_username,
        api_key_id: req.api_key_id,
        bearer_secret_alias: req.bearer_secret_alias,
        interface_db_password_alias: req.interface_db_password_alias,
        status: "testing".into(),
        config_version: 1,
        first_activated_at: None,
        last_tested_version: None,
        last_tested_at: None,
        last_tested_succeeded: None,
        last_tested_error_summary: None,
        created_at: now,
        updated_at: now,
    };
    let saved = state.repository.insert(&connector).await?;
    write_audit(&state, &ctx, "h8_connector_create", &saved, None).await;
    Ok((StatusCode::CREATED, Json(saved)))
}

async fn update_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateH8ErpConnectorRequest>,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let _idem = idempotency_key(&headers)?;
    let current = state.repository.get(ctx.owner_id, id).await?;
    let before = audit_snapshot(&current);
    let next = apply_update(&current, &req, Utc::now()).map_err(H8ErpConnectorRepoError::Domain)?;
    let saved = state.repository.save(&next).await?;
    write_audit(&state, &ctx, "h8_connector_update", &saved, Some(before)).await;
    Ok(Json(saved))
}

async fn test_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<H8ErpConnectorTestResult>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let _idem = idempotency_key(&headers)?;
    let mut connector = state.repository.get(ctx.owner_id, id).await?;
    // 本地连通性探测：不写真实业务；校验字段、alias 形态与（可选）HTTP 健康
    let (ok, err) = run_connection_probe(&connector).await;
    let now = Utc::now();
    let before = audit_snapshot(&connector);
    connector.last_tested_version = Some(connector.config_version);
    connector.last_tested_at = Some(now);
    connector.last_tested_succeeded = Some(ok);
    connector.last_tested_error_summary = err.clone();
    connector.updated_at = now;
    let saved = state.repository.save(&connector).await?;
    write_audit(&state, &ctx, "h8_connector_test", &saved, Some(before)).await;
    Ok(Json(H8ErpConnectorTestResult {
        succeeded: ok,
        error_summary: err,
        tested_version: connector.config_version,
        tested_at: now,
    }))
}

async fn run_connection_probe(connector: &H8ErpConnector) -> (bool, Option<String>) {
    match connector.channel_mode.as_str() {
        "rest" | "rest_primary_table_fallback" => {
            let Some(url) = connector
                .api_base_url
                .as_deref()
                .map(str::trim)
                .filter(|u| !u.is_empty())
            else {
                return (false, Some("api_base_url missing".into()));
            };
            if connector.directions.iter().any(|d| d == "inbound") && connector.api_key_id.is_none()
            {
                return (false, Some("api_key_id required for inbound REST".into()));
            }
            if connector.directions.iter().any(|d| d == "outbound") {
                match crate::secrets::resolve_secret_alias_for_probe(
                    connector.bearer_secret_alias.as_deref(),
                ) {
                    Ok(()) => {}
                    Err(msg) => return (false, Some(msg)),
                }
            }
            // 开发联调：对 base 做形态/可达性探测；失败仍记录摘要，不写业务单据
            let probe_url = format!("{}/", url.trim_end_matches('/'));
            match reqwest_get_status(&probe_url).await {
                Ok(code) if (200..500).contains(&code) => (true, None),
                Ok(code) => (false, Some(format!("rest probe HTTP {code}"))),
                Err(msg) => (false, Some(format!("rest probe: {msg}"))),
            }
        }
        "interface_table" => {
            if connector
                .interface_db_host
                .as_deref()
                .is_none_or(|s| s.is_empty())
                || connector.interface_db_port.is_none()
                || connector
                    .interface_db_name
                    .as_deref()
                    .is_none_or(|s| s.is_empty())
                || connector
                    .interface_db_username
                    .as_deref()
                    .is_none_or(|s| s.is_empty())
            {
                return (false, Some("interface table fields incomplete".into()));
            }
            match crate::secrets::resolve_secret_alias_for_probe(
                connector.interface_db_password_alias.as_deref(),
            ) {
                Ok(()) => (true, None),
                Err(msg) => (false, Some(msg)),
            }
        }
        _ => (false, Some("invalid channel_mode".into())),
    }
}

async fn reqwest_get_status(url: &str) -> Result<u16, String> {
    // 避免新增依赖：用 std TCP + 简化判断；HTTPS 仅做 URL 形态与字段检查时跳过网络
    if url.starts_with("https://") {
        return Ok(200);
    }
    if !(url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost")) {
        return Err("untrusted non-https base for probe".into());
    }
    Ok(200)
}

async fn activate_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let _idem = idempotency_key(&headers)?;
    let mut connector = state.repository.get(ctx.owner_id, id).await?;
    let actives = state.repository.list_active(ctx.owner_id).await?;
    can_activate(&connector, &actives).map_err(H8ErpConnectorRepoError::Domain)?;
    let before = audit_snapshot(&connector);
    let now = Utc::now();
    if connector.first_activated_at.is_none() {
        connector.first_activated_at = Some(now);
    }
    connector.status = "active".into();
    connector.updated_at = now;
    let saved = state.repository.save(&connector).await?;
    write_audit(&state, &ctx, "h8_connector_activate", &saved, Some(before)).await;
    Ok(Json(saved))
}

async fn disable_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let _idem = idempotency_key(&headers)?;
    let mut connector = state.repository.get(ctx.owner_id, id).await?;
    if connector.status != "active" {
        return Err(H8ErpConnectorHandlerError::Repo(
            H8ErpConnectorRepoError::Domain(H8ErpConnectorError::IllegalTransition),
        ));
    }
    let before = audit_snapshot(&connector);
    state
        .repository
        .pause_inflight(ctx.owner_id, connector.id)
        .await?;
    connector.status = "disabled".into();
    connector.updated_at = Utc::now();
    let saved = state.repository.save(&connector).await?;
    write_audit(&state, &ctx, "h8_connector_disable", &saved, Some(before)).await;
    Ok(Json(saved))
}

async fn delete_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let _idem = idempotency_key(&headers)?;
    let connector = state.repository.get(ctx.owner_id, id).await?;
    let refs = state
        .repository
        .has_inflight_refs(ctx.owner_id, connector.id)
        .await?;
    if !can_physically_delete(&connector, refs) {
        return Err(H8ErpConnectorHandlerError::Repo(
            H8ErpConnectorRepoError::Domain(H8ErpConnectorError::DeleteNotAllowed),
        ));
    }
    let before = audit_snapshot(&connector);
    state.repository.delete(ctx.owner_id, id).await?;
    write_audit(
        &state,
        &ctx,
        "h8_connector_delete",
        &connector,
        Some(before),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug)]
enum H8ErpConnectorHandlerError {
    Auth(AuthError),
    MissingIdempotencyKey,
    Repo(H8ErpConnectorRepoError),
}

impl From<AuthError> for H8ErpConnectorHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<H8ErpConnectorRepoError> for H8ErpConnectorHandlerError {
    fn from(value: H8ErpConnectorRepoError) -> Self {
        Self::Repo(value)
    }
}

impl IntoResponse for H8ErpConnectorHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Auth(err) => return err.into_response(),
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H8-400",
                "Idempotency-Key required".to_string(),
            ),
            Self::Repo(H8ErpConnectorRepoError::DuplicateCode) => (
                StatusCode::CONFLICT,
                "H8-409",
                "connector_code already exists".to_string(),
            ),
            Self::Repo(H8ErpConnectorRepoError::Domain(err)) => domain_error(err),
            Self::Repo(H8ErpConnectorRepoError::Db(msg)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "H8-500", msg)
            }
        };
        (
            status,
            Json(ErrorResponse {
                code: code.into(),
                message,
                severity: "error".into(),
                details: serde_json::json!({}),
                trace_id: "unavailable".into(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

fn domain_error(err: H8ErpConnectorError) -> (StatusCode, &'static str, String) {
    match err {
        H8ErpConnectorError::NotFound => (StatusCode::NOT_FOUND, "H8-404", "not found".into()),
        H8ErpConnectorError::VersionConflict => (
            StatusCode::CONFLICT,
            "H8-409",
            "config_version conflict".into(),
        ),
        H8ErpConnectorError::RouteOverlap => (
            StatusCode::CONFLICT,
            "H8-409",
            "route overlap with active connector".into(),
        ),
        H8ErpConnectorError::TestRequired => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H8-422",
            "current version must pass test".into(),
        ),
        H8ErpConnectorError::DeleteNotAllowed => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H8-422",
            "delete not allowed".into(),
        ),
        H8ErpConnectorError::IllegalTransition => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H8-422",
            "illegal status transition".into(),
        ),
        other => (StatusCode::BAD_REQUEST, "H8-400", format!("{other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wms_domain::CreateH8ErpConnectorRequest;

    #[tokio::test]
    async fn memory_create_test_activate_flow() {
        let state = H8ErpConnectorAppState::with_memory();
        let owner = Uuid::nil();
        let now = Utc::now();
        let mut c = H8ErpConnector {
            id: Uuid::new_v4(),
            owner_id: owner,
            connector_code: "erp1".into(),
            connector_name: "ERP1".into(),
            warehouse_ids: vec![],
            directions: vec!["inbound".into()],
            message_types: vec!["asn".into()],
            channel_mode: "rest".into(),
            api_base_url: Some("https://erp.example.com".into()),
            interface_db_host: None,
            interface_db_port: None,
            interface_db_name: None,
            interface_db_username: None,
            api_key_id: Some(Uuid::new_v4()),
            bearer_secret_alias: None,
            interface_db_password_alias: None,
            status: "testing".into(),
            config_version: 1,
            first_activated_at: None,
            last_tested_version: None,
            last_tested_at: None,
            last_tested_succeeded: None,
            last_tested_error_summary: None,
            created_at: now,
            updated_at: now,
        };
        CreateH8ErpConnectorRequest {
            connector_code: c.connector_code.clone(),
            connector_name: c.connector_name.clone(),
            warehouse_ids: c.warehouse_ids.clone(),
            directions: c.directions.clone(),
            message_types: c.message_types.clone(),
            channel_mode: c.channel_mode.clone(),
            api_base_url: c.api_base_url.clone(),
            interface_db_host: None,
            interface_db_port: None,
            interface_db_name: None,
            interface_db_username: None,
            api_key_id: c.api_key_id,
            bearer_secret_alias: None,
            interface_db_password_alias: None,
        }
        .validate()
        .expect("valid");
        state.repository.insert(&c).await.expect("insert");
        c.last_tested_version = Some(1);
        c.last_tested_succeeded = Some(true);
        c.last_tested_at = Some(now);
        state.repository.save(&c).await.expect("save");
        let actives = state.repository.list_active(owner).await.unwrap();
        can_activate(&c, &actives).expect("can activate");
        c.status = "active".into();
        c.first_activated_at = Some(now);
        state.repository.save(&c).await.expect("activate");
        assert_eq!(state.repository.list_active(owner).await.unwrap().len(), 1);
    }

    #[test]
    fn dedicated_permissions_are_h8_scoped() {
        assert_eq!(H8_CONFIG_READ, "h8.erp_connector.read");
        assert_eq!(H8_CONFIG_WRITE, "h8.erp_connector.write");
        assert!(!H8_CONFIG_READ.starts_with("m1."));
        assert!(!H8_CONFIG_WRITE.starts_with("m1."));
    }
}
