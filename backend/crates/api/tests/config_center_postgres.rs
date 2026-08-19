use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthRevocationStore,
        AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    config_center::{config_center_router, ConfigCenterAppState},
    feature_flags::FeatureFlagRegistry,
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

fn bearer_token(owner_id: Uuid) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "config-center-writer",
        vec!["m1.config.write".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
}

#[sqlx::test(migrations = "../../migrations")]
async fn feature_flag_writes_persist_owner_scoped_audit_events(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let registry = FeatureFlagRegistry::from_toml_str(
        r#"
        [[flags]]
        key = "config_center_audit_test"
        owner = "platform"
        created_at = 2026-07-11
        cleanup_by = 2026-08-31
        enabled = true
        "#,
    )
    .expect("feature flag registry should parse");
    let app = config_center_router(ConfigCenterAppState::with_postgres(registry, pool.clone()))
        .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
            AllowAllRevocationStore,
        ))));
    let token = bearer_token(owner_id);
    let writes = [
        ("/api/v1/config-center/feature-flags/migrate", None),
        (
            "/api/v1/config-center/feature-flags/import",
            Some(serde_json::json!({"flags": []})),
        ),
        (
            "/api/v1/config-center/feature-flags/source",
            Some(serde_json::json!({"source": "config_center"})),
        ),
        (
            "/api/v1/config-center/feature-flags/archive-file-source",
            Some(serde_json::json!({"archive_ref": "s3://audit/feature_flags.toml"})),
        ),
    ];

    for (path, body) in writes {
        let request = Request::post(path)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
            .expect("request should build");
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("route should respond");
        assert_eq!(response.status(), StatusCode::OK, "{path} should succeed");
    }

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE owner_id = $1 AND module = 'M1' AND resource_type = 'feature_flag' ORDER BY action",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("owner audit events should query");
    assert_eq!(
        actions,
        vec![
            "archive_feature_flag_file_source",
            "import_feature_flags",
            "migrate_feature_flags",
            "switch_feature_flag_source",
        ]
    );

    let other_owner_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_event WHERE owner_id = $1")
            .bind(other_owner_id)
            .fetch_one(&pool)
            .await
            .expect("other owner audit count should query");
    assert_eq!(other_owner_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn feature_flag_import_is_exported_from_postgres_for_same_owner(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let app = config_center_router(ConfigCenterAppState::with_postgres(
        FeatureFlagRegistry::empty(),
        pool.clone(),
    ))
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        AllowAllRevocationStore,
    ))));
    let token = bearer_token(owner_id);
    let response = app
        .oneshot(
            Request::post("/api/v1/config-center/feature-flags/import")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "flags": [{
                            "key": "config_center_pg_roundtrip",
                            "owner": "platform",
                            "created_at": "2026-07-14",
                            "cleanup_by": "2026-10-01",
                            "enabled": true,
                            "source": "operator_upload"
                        }]
                    })
                    .to_string(),
                ))
                .expect("import request should build"),
        )
        .await
        .expect("import route should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let reloaded = config_center_router(ConfigCenterAppState::with_postgres(
        FeatureFlagRegistry::empty(),
        pool,
    ))
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        AllowAllRevocationStore,
    ))));
    let response = reloaded
        .clone()
        .oneshot(
            Request::get("/api/v1/config-center/feature-flags/export")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("export request should build"),
        )
        .await
        .expect("export route should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let exported: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("export body should read"),
    )
    .expect("export should be json");
    assert_eq!(exported["flags"][0]["key"], "config_center_pg_roundtrip");

    let other_token = bearer_token(other_owner_id);
    let response = reloaded
        .oneshot(
            Request::get("/api/v1/config-center/feature-flags/export")
                .header(AUTHORIZATION, format!("Bearer {other_token}"))
                .body(Body::empty())
                .expect("other export request should build"),
        )
        .await
        .expect("other export route should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let other_exported: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("other export body should read"),
    )
    .expect("other export should be json");
    assert!(other_exported["flags"]
        .as_array()
        .is_some_and(Vec::is_empty));
}
