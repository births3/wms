//! Wave 2 M1-008 config-center backed Feature Flag service.

use std::{collections::BTreeMap, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    ConfigEntry, ErrorResponse, FeatureFlagArchiveRequest, FeatureFlagArchiveResult,
    FeatureFlagBatchImportRequest, FeatureFlagBatchImportResult, FeatureFlagConfig,
    FeatureFlagExportResponse, FeatureFlagMigrationResult, FeatureFlagReconcileReport,
    FeatureFlagSourceSwitchRequest, FeatureFlagSourceSwitchResponse,
};

use crate::{
    audit::{append_event, AuditWriteRequest},
    auth::{AuthContext, AuthError},
    feature_flags::FeatureFlagRegistry,
};

mod persistence;

pub const CONFIG_FLAG_MISSING_CODE: &str = "M1_CONFIG_FLAG_MISSING";
pub const CONFIG_FLAG_DISABLED_CODE: &str = "M1_CONFIG_FLAG_DISABLED";
pub const CONFIG_FLAG_SOURCE_INVALID_CODE: &str = "M1_CONFIG_FLAG_SOURCE_INVALID";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigCenterError {
    MissingFlag(String),
    DisabledFlag(String),
    InvalidFeatureFlagSource(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeatureFlagSource {
    File,
    ConfigCenter,
}

#[derive(Clone, Debug)]
pub struct ConfigCenterStore {
    entries: BTreeMap<String, ConfigEntry>,
    feature_flags: BTreeMap<String, FeatureFlagConfig>,
    active_source: FeatureFlagSource,
}

#[derive(Clone, Debug)]
pub struct ConfigCenterAppState {
    store: Arc<Mutex<ConfigCenterStore>>,
    file_registry: Arc<FeatureFlagRegistry>,
    pool: Option<PgPool>,
}

impl Default for ConfigCenterAppState {
    fn default() -> Self {
        Self::from_registry(FeatureFlagRegistry::empty())
    }
}

impl ConfigCenterAppState {
    pub fn from_registry(file_registry: FeatureFlagRegistry) -> Self {
        Self {
            store: Arc::new(Mutex::new(ConfigCenterStore::default())),
            file_registry: Arc::new(file_registry),
            pool: None,
        }
    }

    pub fn with_postgres(file_registry: FeatureFlagRegistry, pool: PgPool) -> Self {
        Self {
            pool: Some(pool),
            ..Self::from_registry(file_registry)
        }
    }

    async fn append_feature_flag_audit(
        &self,
        ctx: &AuthContext,
        action: &str,
    ) -> Result<(), ConfigCenterHandlerError> {
        if let Some(pool) = &self.pool {
            append_event(
                pool,
                &AuditWriteRequest::from_auth_context(
                    ctx,
                    action,
                    "M1",
                    "feature_flag",
                    "feature_flags",
                    None,
                ),
            )
            .await
            .map_err(|error| ConfigCenterHandlerError::Audit(format!("{error:?}")))?;
        }
        Ok(())
    }

    pub async fn migrate_feature_flags_from_file(&self) -> FeatureFlagMigrationResult {
        let mut store = self.store.lock().await;
        store.migrate_feature_flags_from_file(&self.file_registry)
    }

    pub async fn reconcile_feature_flags(&self) -> FeatureFlagReconcileReport {
        let store = self.store.lock().await;
        store.reconcile_feature_flags(&self.file_registry)
    }

    pub async fn export_feature_flags(&self) -> FeatureFlagExportResponse {
        let store = self.store.lock().await;
        store.export_feature_flags()
    }

    pub async fn import_feature_flags_batch(
        &self,
        flags: Vec<FeatureFlagConfig>,
    ) -> FeatureFlagBatchImportResult {
        let mut store = self.store.lock().await;
        store.import_feature_flags_batch(flags)
    }

    pub async fn switch_feature_flag_source(
        &self,
        source: FeatureFlagSource,
    ) -> FeatureFlagSourceSwitchResponse {
        let mut store = self.store.lock().await;
        store.switch_feature_flag_source(source)
    }

    pub async fn archive_file_feature_flags(
        &self,
        archive_ref: String,
        archived_at: DateTime<Utc>,
    ) -> FeatureFlagArchiveResult {
        let store = self.store.lock().await;
        store.archive_file_feature_flags(archive_ref, archived_at)
    }

    pub async fn is_feature_enabled(&self, key: &str) -> Result<bool, ConfigCenterError> {
        let store = self.store.lock().await;
        store.is_feature_enabled(key, &self.file_registry)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigCenterHandlerError {
    Auth(AuthError),
    ConfigCenter(ConfigCenterError),
    Audit(String),
    Storage(String),
}

impl From<AuthError> for ConfigCenterHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<ConfigCenterError> for ConfigCenterHandlerError {
    fn from(value: ConfigCenterError) -> Self {
        Self::ConfigCenter(value)
    }
}

impl IntoResponse for ConfigCenterHandlerError {
    fn into_response(self) -> Response {
        if let ConfigCenterHandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            ConfigCenterHandlerError::Audit(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1_CONFIG_AUDIT_FAILED",
                "Feature Flag 审计写入失败",
            ),
            ConfigCenterHandlerError::Storage(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1_CONFIG_STORAGE_FAILED",
                "Feature Flag 配置持久化失败",
            ),
            ConfigCenterHandlerError::ConfigCenter(ConfigCenterError::MissingFlag(_)) => (
                StatusCode::NOT_FOUND,
                CONFIG_FLAG_MISSING_CODE,
                "Feature Flag 不存在",
            ),
            ConfigCenterHandlerError::ConfigCenter(ConfigCenterError::DisabledFlag(_)) => (
                StatusCode::NOT_FOUND,
                CONFIG_FLAG_DISABLED_CODE,
                "Feature Flag 未启用",
            ),
            ConfigCenterHandlerError::ConfigCenter(
                ConfigCenterError::InvalidFeatureFlagSource(_),
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                CONFIG_FLAG_SOURCE_INVALID_CODE,
                "Feature Flag 读取源无效",
            ),
            ConfigCenterHandlerError::Auth(_) => unreachable!("auth error returned above"),
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

impl Default for ConfigCenterStore {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            feature_flags: BTreeMap::new(),
            active_source: FeatureFlagSource::File,
        }
    }
}

impl ConfigCenterStore {
    pub fn put_entry(
        &mut self,
        ctx: &AuthContext,
        key: impl Into<String>,
        value: serde_json::Value,
        now: DateTime<Utc>,
    ) -> ConfigEntry {
        let key = key.into();
        let version = self
            .entries
            .get(&key)
            .map(|entry| entry.version + 1)
            .unwrap_or(1);
        let entry = ConfigEntry {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            config_key: key.clone(),
            config_value: value,
            version,
            updated_at: now,
        };
        self.entries.insert(key, entry.clone());
        entry
    }

    pub fn migrate_feature_flags_from_file(
        &mut self,
        file_registry: &FeatureFlagRegistry,
    ) -> FeatureFlagMigrationResult {
        let mut migrated_count = 0;
        for flag in file_registry.flags() {
            self.feature_flags.insert(
                flag.key.clone(),
                FeatureFlagConfig {
                    key: flag.key.clone(),
                    owner: flag.owner.clone(),
                    created_at: flag.created_at.clone(),
                    cleanup_by: flag.cleanup_by.clone(),
                    enabled: flag.enabled,
                    source: "m1_config_center".to_string(),
                },
            );
            migrated_count += 1;
        }
        FeatureFlagMigrationResult {
            migrated_count,
            source: "deploy/feature_flags.toml".to_string(),
            target: "M1-008 config center".to_string(),
        }
    }

    pub fn import_feature_flags_batch(
        &mut self,
        flags: Vec<FeatureFlagConfig>,
    ) -> FeatureFlagBatchImportResult {
        let imported_count = flags.len() as u32;
        for mut flag in flags {
            flag.source = "m1_config_center".to_string();
            self.feature_flags.insert(flag.key.clone(), flag);
        }
        FeatureFlagBatchImportResult {
            imported_count,
            target: "M1-008 config center".to_string(),
        }
    }

    pub fn reconcile_feature_flags(
        &self,
        file_registry: &FeatureFlagRegistry,
    ) -> FeatureFlagReconcileReport {
        let mut matched = 0;
        let mut missing_in_config_center = Vec::new();
        let mut mismatched = Vec::new();

        for file_flag in file_registry.flags() {
            match self.feature_flags.get(&file_flag.key) {
                Some(config_flag)
                    if config_flag.owner == file_flag.owner
                        && config_flag.created_at == file_flag.created_at
                        && config_flag.cleanup_by == file_flag.cleanup_by
                        && config_flag.enabled == file_flag.enabled =>
                {
                    matched += 1;
                }
                Some(_) => mismatched.push(file_flag.key.clone()),
                None => missing_in_config_center.push(file_flag.key.clone()),
            }
        }

        FeatureFlagReconcileReport {
            matched,
            missing_in_config_center,
            mismatched,
        }
    }

    pub fn switch_feature_flag_source(
        &mut self,
        source: FeatureFlagSource,
    ) -> FeatureFlagSourceSwitchResponse {
        self.active_source = source;
        FeatureFlagSourceSwitchResponse {
            active_source: self.active_source_name().to_string(),
        }
    }

    pub fn is_feature_enabled(
        &self,
        key: &str,
        file_registry: &FeatureFlagRegistry,
    ) -> Result<bool, ConfigCenterError> {
        match self.active_source {
            FeatureFlagSource::File => Ok(file_registry.is_enabled(key)),
            FeatureFlagSource::ConfigCenter => self
                .feature_flags
                .get(key)
                .map(|flag| flag.enabled)
                .ok_or_else(|| ConfigCenterError::MissingFlag(key.to_string())),
        }
    }

    pub fn export_feature_flags(&self) -> FeatureFlagExportResponse {
        FeatureFlagExportResponse {
            source: self.active_source_name().to_string(),
            flags: self.feature_flags.values().cloned().collect(),
        }
    }

    pub fn archive_file_feature_flags(
        &self,
        archive_ref: impl Into<String>,
        archived_at: DateTime<Utc>,
    ) -> FeatureFlagArchiveResult {
        FeatureFlagArchiveResult {
            archived_source: "deploy/feature_flags.toml".to_string(),
            archive_ref: archive_ref.into(),
            archived_at,
        }
    }

    pub fn active_source_name(&self) -> &'static str {
        match self.active_source {
            FeatureFlagSource::File => "file",
            FeatureFlagSource::ConfigCenter => "config_center",
        }
    }
}

pub fn config_center_router(state: ConfigCenterAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/config-center/feature-flags/migrate",
            post(migrate_feature_flags_handler),
        )
        .route(
            "/api/v1/config-center/feature-flags/reconcile",
            get(reconcile_feature_flags_handler),
        )
        .route(
            "/api/v1/config-center/feature-flags/export",
            get(export_feature_flags_handler),
        )
        .route(
            "/api/v1/config-center/feature-flags/import",
            post(import_feature_flags_handler),
        )
        .route(
            "/api/v1/config-center/feature-flags/source",
            post(switch_feature_flag_source_handler),
        )
        .route(
            "/api/v1/config-center/feature-flags/archive-file-source",
            post(archive_feature_flag_file_source_handler),
        )
        .with_state(state)
}

async fn migrate_feature_flags_handler(
    ctx: AuthContext,
    State(state): State<ConfigCenterAppState>,
) -> Result<Json<FeatureFlagMigrationResult>, ConfigCenterHandlerError> {
    ctx.require_permission("m1.config.write")?;
    if state.pool.is_some() {
        let flags = state
            .file_registry
            .flags()
            .map(|flag| FeatureFlagConfig {
                key: flag.key.clone(),
                owner: flag.owner.clone(),
                created_at: flag.created_at.clone(),
                cleanup_by: flag.cleanup_by.clone(),
                enabled: flag.enabled,
                source: "m1_config_center".to_string(),
            })
            .collect::<Vec<_>>();
        state
            .persist_feature_flags(&ctx, "migrate_feature_flags", &flags)
            .await?;
        return Ok(Json(FeatureFlagMigrationResult {
            migrated_count: flags.len() as u32,
            source: "deploy/feature_flags.toml".to_string(),
            target: "M1-008 config center".to_string(),
        }));
    }
    let mut store = state.store.lock().await;
    let mut next = store.clone();
    let result = next.migrate_feature_flags_from_file(&state.file_registry);
    state
        .append_feature_flag_audit(&ctx, "migrate_feature_flags")
        .await?;
    *store = next;
    Ok(Json(result))
}

async fn reconcile_feature_flags_handler(
    ctx: AuthContext,
    State(state): State<ConfigCenterAppState>,
) -> Result<Json<FeatureFlagReconcileReport>, ConfigCenterHandlerError> {
    ctx.require_permission("m1.config.write")?;
    Ok(Json(state.reconcile_feature_flags().await))
}

async fn export_feature_flags_handler(
    ctx: AuthContext,
    State(state): State<ConfigCenterAppState>,
) -> Result<Json<FeatureFlagExportResponse>, ConfigCenterHandlerError> {
    ctx.require_permission("m1.config.write")?;
    if state.pool.is_some() {
        return Ok(Json(
            state
                .export_feature_flags_from_postgres(ctx.owner_id)
                .await?,
        ));
    }
    Ok(Json(state.export_feature_flags().await))
}

async fn import_feature_flags_handler(
    ctx: AuthContext,
    State(state): State<ConfigCenterAppState>,
    Json(req): Json<FeatureFlagBatchImportRequest>,
) -> Result<Json<FeatureFlagBatchImportResult>, ConfigCenterHandlerError> {
    ctx.require_permission("m1.config.write")?;
    let flags = req.flags;
    if state.pool.is_some() {
        state
            .persist_feature_flags(&ctx, "import_feature_flags", &flags)
            .await?;
        return Ok(Json(FeatureFlagBatchImportResult {
            imported_count: flags.len() as u32,
            target: "M1-008 config center".to_string(),
        }));
    }
    let mut store = state.store.lock().await;
    let mut next = store.clone();
    let result = next.import_feature_flags_batch(flags);
    state
        .append_feature_flag_audit(&ctx, "import_feature_flags")
        .await?;
    *store = next;
    Ok(Json(result))
}

async fn switch_feature_flag_source_handler(
    ctx: AuthContext,
    State(state): State<ConfigCenterAppState>,
    Json(req): Json<FeatureFlagSourceSwitchRequest>,
) -> Result<Json<FeatureFlagSourceSwitchResponse>, ConfigCenterHandlerError> {
    ctx.require_permission("m1.config.write")?;
    let source = match req.source.as_str() {
        "file" => FeatureFlagSource::File,
        "config_center" => FeatureFlagSource::ConfigCenter,
        other => {
            return Err(ConfigCenterError::InvalidFeatureFlagSource(other.to_string()).into());
        }
    };
    let mut store = state.store.lock().await;
    let mut next = store.clone();
    let result = next.switch_feature_flag_source(source);
    state
        .append_feature_flag_audit(&ctx, "switch_feature_flag_source")
        .await?;
    *store = next;
    Ok(Json(result))
}

async fn archive_feature_flag_file_source_handler(
    ctx: AuthContext,
    State(state): State<ConfigCenterAppState>,
    Json(req): Json<FeatureFlagArchiveRequest>,
) -> Result<Json<FeatureFlagArchiveResult>, ConfigCenterHandlerError> {
    ctx.require_permission("m1.config.write")?;
    let mut store = state.store.lock().await;
    let next = store.clone();
    let result = next.archive_file_feature_flags(req.archive_ref, Utc::now());
    state
        .append_feature_flag_audit(&ctx, "archive_feature_flag_file_source")
        .await?;
    *store = next;
    Ok(Json(result))
}

#[cfg(test)]
mod tests;
