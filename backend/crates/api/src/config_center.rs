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
    Ok(Json(state.export_feature_flags().await))
}

async fn import_feature_flags_handler(
    ctx: AuthContext,
    State(state): State<ConfigCenterAppState>,
    Json(req): Json<FeatureFlagBatchImportRequest>,
) -> Result<Json<FeatureFlagBatchImportResult>, ConfigCenterHandlerError> {
    ctx.require_permission("m1.config.write")?;
    let mut store = state.store.lock().await;
    let mut next = store.clone();
    let result = next.import_feature_flags_batch(req.flags);
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
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;

    use super::{
        config_center_router, ConfigCenterAppState, ConfigCenterStore, FeatureFlagSource,
        CONFIG_FLAG_SOURCE_INVALID_CODE,
    };
    use crate::{
        auth::{
            auth_runtime_layer, build_access_claims, encode_access_token, AuthContext,
            AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
        },
        feature_flags::FeatureFlagRegistry,
    };
    use axum::{
        body::{to_bytes, Body},
        http::{header::AUTHORIZATION, Method, Request, StatusCode},
        Router,
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    use wms_domain::{
        FeatureFlagArchiveResult, FeatureFlagBatchImportResult, FeatureFlagExportResponse,
        FeatureFlagMigrationResult, FeatureFlagReconcileReport, FeatureFlagSourceSwitchResponse,
    };

    struct AllowAllRevocationStore;

    #[axum::async_trait]
    impl AuthRevocationStore for AllowAllRevocationStore {
        async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
            Ok(false)
        }

        async fn permissions_changed_at(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<i64>, AuthRevocationStoreError> {
            Ok(None)
        }

        async fn blacklist_jti(
            &self,
            _jti: &str,
            _ttl_seconds: u64,
        ) -> Result<(), AuthRevocationStoreError> {
            Ok(())
        }

        async fn set_permissions_changed_at(
            &self,
            _user_id: Uuid,
            _changed_at_unix: i64,
        ) -> Result<(), AuthRevocationStoreError> {
            Ok(())
        }
    }

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["m1.config.write".to_string()],
            jti: Uuid::new_v4().to_string(),
        }
    }

    fn test_registry() -> FeatureFlagRegistry {
        FeatureFlagRegistry::from_toml_str(
            r#"
            [[flags]]
            key = "w2_config_center_flags"
            owner = "platform"
            created_at = 2026-06-04
            cleanup_by = 2026-08-31
            enabled = true
            "#,
        )
        .expect("valid file registry")
    }

    fn bearer_token(permissions: Vec<String>) -> String {
        std::env::set_var(JWT_SECRET_ENV, "test-secret");
        let claims = build_access_claims(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "config-admin",
            permissions,
            Uuid::new_v4().to_string(),
            Utc::now(),
        );
        encode_access_token(&claims, "test-secret").expect("token should encode")
    }

    fn router(state: ConfigCenterAppState) -> Router {
        config_center_router(state).layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
            AllowAllRevocationStore,
        ))))
    }

    async fn json_response<T: serde::de::DeserializeOwned>(
        app: Router,
        method: Method,
        path: &str,
        token: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, T) {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header(AUTHORIZATION, format!("Bearer {token}"));
        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }
        let request = builder
            .body(match body {
                Some(value) => Body::from(value.to_string()),
                None => Body::empty(),
            })
            .expect("request should build");
        let response = app.oneshot(request).await.expect("router should respond");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload = serde_json::from_slice(&body).expect("response should be json");
        (status, payload)
    }

    #[test]
    fn migrates_reconciles_and_switches_feature_flags() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 12, 30, 0)
            .single()
            .expect("valid time");
        let file_registry = FeatureFlagRegistry::from_toml_str(
            r#"
            [[flags]]
            key = "w2_config_center_flags"
            owner = "platform"
            created_at = 2026-06-04
            cleanup_by = 2026-08-31
            enabled = true
            "#,
        )
        .expect("valid file registry");
        let mut store = ConfigCenterStore::default();

        let result = store.migrate_feature_flags_from_file(&file_registry);
        assert_eq!(result.migrated_count, 1);

        let imported = store.import_feature_flags_batch(vec![wms_domain::FeatureFlagConfig {
            key: "w2_bulk_imported_flag".to_string(),
            owner: "platform".to_string(),
            created_at: "2026-06-04".to_string(),
            cleanup_by: "2026-08-31".to_string(),
            enabled: false,
            source: "operator_upload".to_string(),
        }]);
        assert_eq!(imported.imported_count, 1);

        let report = store.reconcile_feature_flags(&file_registry);
        assert_eq!(report.matched, 1);
        assert!(report.missing_in_config_center.is_empty());
        assert!(report.mismatched.is_empty());

        let switched = store.switch_feature_flag_source(FeatureFlagSource::ConfigCenter);
        assert_eq!(switched.active_source, "config_center");
        assert!(store
            .is_feature_enabled("w2_config_center_flags", &file_registry)
            .expect("flag exists"));

        let exported = store.export_feature_flags();
        assert_eq!(exported.source, "config_center");
        assert_eq!(exported.flags.len(), 2);

        let archived =
            store.archive_file_feature_flags("s3://wms-dev/archive/feature_flags.toml", now);
        assert_eq!(archived.archived_source, "deploy/feature_flags.toml");
        assert_eq!(
            archived.archive_ref,
            "s3://wms-dev/archive/feature_flags.toml"
        );
    }

    #[test]
    fn config_entries_are_versioned() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = ConfigCenterStore::default();

        let first = store.put_entry(&ctx, "m1.default_page_size", json!({"value": 50}), now);
        let second = store.put_entry(&ctx, "m1.default_page_size", json!({"value": 100}), now);

        assert_eq!(first.version, 1);
        assert_eq!(second.version, 2);
    }

    #[tokio::test]
    async fn config_center_router_runs_wave2_feature_flag_runtime_flow() {
        let state = ConfigCenterAppState::from_registry(test_registry());
        let token = bearer_token(vec!["m1.config.write".to_string()]);

        let (status, migrated): (StatusCode, FeatureFlagMigrationResult) = json_response(
            router(state.clone()),
            Method::POST,
            "/api/v1/config-center/feature-flags/migrate",
            &token,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(migrated.migrated_count, 1);

        let (status, reconcile): (StatusCode, FeatureFlagReconcileReport) = json_response(
            router(state.clone()),
            Method::GET,
            "/api/v1/config-center/feature-flags/reconcile",
            &token,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(reconcile.matched, 1);
        assert!(reconcile.missing_in_config_center.is_empty());
        assert!(reconcile.mismatched.is_empty());

        let (status, imported): (StatusCode, FeatureFlagBatchImportResult) = json_response(
            router(state.clone()),
            Method::POST,
            "/api/v1/config-center/feature-flags/import",
            &token,
            Some(json!({
                "flags": [{
                    "key": "w2_bulk_imported_flag",
                    "owner": "platform",
                    "created_at": "2026-06-04",
                    "cleanup_by": "2026-08-31",
                    "enabled": false,
                    "source": "operator_upload"
                }]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(imported.imported_count, 1);

        let (status, switched): (StatusCode, FeatureFlagSourceSwitchResponse) = json_response(
            router(state.clone()),
            Method::POST,
            "/api/v1/config-center/feature-flags/source",
            &token,
            Some(json!({"source": "config_center"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(switched.active_source, "config_center");

        let (status, exported): (StatusCode, FeatureFlagExportResponse) = json_response(
            router(state.clone()),
            Method::GET,
            "/api/v1/config-center/feature-flags/export",
            &token,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(exported.source, "config_center");
        assert_eq!(exported.flags.len(), 2);

        let (status, archived): (StatusCode, FeatureFlagArchiveResult) = json_response(
            router(state),
            Method::POST,
            "/api/v1/config-center/feature-flags/archive-file-source",
            &token,
            Some(json!({"archive_ref": "s3://wms-dev/archive/feature_flags.toml"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(archived.archived_source, "deploy/feature_flags.toml");
        assert_eq!(
            archived.archive_ref,
            "s3://wms-dev/archive/feature_flags.toml"
        );
    }

    #[tokio::test]
    async fn config_center_router_requires_config_write_permission() {
        let state = ConfigCenterAppState::from_registry(test_registry());
        let token = bearer_token(vec!["m1.read".to_string()]);
        let (status, error): (StatusCode, wms_domain::ErrorResponse) = json_response(
            router(state),
            Method::POST,
            "/api/v1/config-center/feature-flags/migrate",
            &token,
            None,
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(error.code, "AUTH-005");
    }

    #[tokio::test]
    async fn config_center_router_rejects_unknown_feature_flag_source() {
        let state = ConfigCenterAppState::from_registry(test_registry());
        let token = bearer_token(vec!["m1.config.write".to_string()]);
        let (status, error): (StatusCode, wms_domain::ErrorResponse) = json_response(
            router(state),
            Method::POST,
            "/api/v1/config-center/feature-flags/source",
            &token,
            Some(json!({"source": "spreadsheet"})),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code, CONFIG_FLAG_SOURCE_INVALID_CODE);
    }
}
