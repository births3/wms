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
        warehouse_scope: None,
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

    let archived = store.archive_file_feature_flags("s3://wms-dev/archive/feature_flags.toml", now);
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
