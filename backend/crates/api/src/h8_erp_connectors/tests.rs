//! H8 ERP 连接单元测试。

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::{
    can_activate, inflight_status_after_activate, inflight_status_after_disable,
    CreateH8ErpConnectorRequest, H8ErpConnector, H8ErpConnectorError, H8_INFLIGHT_PAUSED,
    H8_INFLIGHT_RUNNING,
};

use super::error::{H8ErpConnectorHandlerError, H8ErpConnectorRepoError};
use super::handlers::h8_erp_connector_router;
use super::idempotency::{load_idempotent_response, store_idempotent_response};
use super::repository::{H8ErpConnectorRepository, PgH8ErpConnectorRepository};
use super::state::{H8ErpConnectorAppState, H8_CONFIG_READ, H8_CONFIG_WRITE};
use crate::auth::AuthContext;

fn versioned_connector(owner_id: Uuid) -> H8ErpConnector {
    let now = Utc::now();
    H8ErpConnector {
        id: Uuid::new_v4(),
        owner_id,
        connector_code: "versioned".into(),
        connector_name: "Versioned ERP".into(),
        warehouse_ids: vec![],
        directions: vec!["inbound".into()],
        message_types: vec!["asn".into()],
        channel_mode: "rest".into(),
        api_base_url: Some("https://erp-v1.example.com".into()),
        interface_db_host: None,
        interface_db_port: None,
        interface_db_name: None,
        interface_db_username: None,
        api_key_id: Some(Uuid::new_v4()),
        bearer_secret_alias: None,
        interface_db_password_alias: None,
        interface_probe_db_username: None,
        interface_probe_db_password_alias: None,
        interface_probe_db_password_alias_set: false,
        interface_probe_config_version: 1,
        status: "testing".into(),
        config_version: 1,
        first_activated_at: None,
        last_tested_version: None,
        last_tested_at: None,
        last_tested_succeeded: None,
        last_tested_error_summary: None,
        created_at: now,
        updated_at: now,
    }
}

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
        interface_probe_db_username: None,
        interface_probe_db_password_alias: None,
        interface_probe_db_password_alias_set: false,
        interface_probe_config_version: 1,
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
        interface_probe_db_username: None,
        interface_probe_db_password_alias: None,
    }
    .validate()
    .expect("valid");
    state.repository.insert(&c).await.expect("insert");
    c.last_tested_version = Some(1);
    c.last_tested_succeeded = Some(true);
    c.last_tested_at = Some(now);
    state.repository.save(&c, 1, 1).await.expect("save");
    let actives = state.repository.list_active(owner).await.unwrap();
    can_activate(&c, &actives).expect("can activate");
    c.status = "active".into();
    c.first_activated_at = Some(now);
    state.repository.save(&c, 1, 1).await.expect("activate");
    assert_eq!(state.repository.list_active(owner).await.unwrap().len(), 1);
}

#[tokio::test]
async fn memory_pause_and_resume_inflight() {
    let state = H8ErpConnectorAppState::with_memory();
    let repo = state.repository.clone();
    let owner = Uuid::nil();
    let id = Uuid::new_v4();
    repo.bind_inflight(
        owner,
        id,
        "idem-1",
        "outbound",
        "asn",
        "rest",
        H8_INFLIGHT_RUNNING,
    )
    .await
    .unwrap();
    assert!(repo.has_inflight_refs(owner, id).await.unwrap());
    assert_eq!(repo.pause_inflight(owner, id).await.unwrap(), 1);
    assert_eq!(
        inflight_status_after_disable(H8_INFLIGHT_RUNNING),
        Some(H8_INFLIGHT_PAUSED)
    );
    assert_eq!(repo.resume_inflight(owner, id).await.unwrap(), 1);
    assert_eq!(
        inflight_status_after_activate(H8_INFLIGHT_PAUSED),
        Some(H8_INFLIGHT_RUNNING)
    );
}

#[tokio::test]
async fn memory_idempotent_create_replays_same_body() {
    let state = H8ErpConnectorAppState::with_memory();
    let owner = Uuid::new_v4();
    let key = "idem-create-1";
    let body = serde_json::json!({"connector_code": "c1"});
    let hash = format!("{:x}", Sha256::digest(body.to_string().as_bytes()));
    store_idempotent_response(
        &state,
        owner,
        key,
        &hash,
        "POST",
        "/api/v1/config/erp-connectors",
        StatusCode::CREATED,
        &serde_json::json!({"id": "00000000-0000-0000-0000-000000000001"}),
    )
    .await
    .unwrap();
    let replay = load_idempotent_response(&state, owner, key, &hash)
        .await
        .unwrap()
        .expect("replay");
    assert_eq!(replay.0, StatusCode::CREATED);
    let conflict = load_idempotent_response(&state, owner, key, "other-hash").await;
    assert!(matches!(
        conflict,
        Err(H8ErpConnectorHandlerError::Repo(
            H8ErpConnectorRepoError::Domain(H8ErpConnectorError::IdempotencyConflict)
        ))
    ));
}

#[tokio::test]
async fn memory_optimistic_lock_rejects_stale_version() {
    let state = H8ErpConnectorAppState::with_memory();
    let owner = Uuid::nil();
    let now = Utc::now();
    let c = H8ErpConnector {
        id: Uuid::new_v4(),
        owner_id: owner,
        connector_code: "v1".into(),
        connector_name: "V1".into(),
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
        interface_probe_db_username: None,
        interface_probe_db_password_alias: None,
        interface_probe_db_password_alias_set: false,
        interface_probe_config_version: 1,
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
    state.repository.insert(&c).await.unwrap();
    let mut next = c.clone();
    next.connector_name = "V2".into();
    next.config_version = 2;
    let err = state.repository.save(&next, 99, 1).await.unwrap_err();
    assert!(matches!(
        err,
        H8ErpConnectorRepoError::Domain(H8ErpConnectorError::VersionConflict)
    ));
    next.interface_probe_config_version = 2;
    let probe_err = state.repository.save(&next, 1, 99).await.unwrap_err();
    assert!(matches!(
        probe_err,
        H8ErpConnectorRepoError::Domain(H8ErpConnectorError::ProbeVersionConflict)
    ));
    state.repository.save(&next, 1, 1).await.unwrap();
}

#[tokio::test]
async fn route_resolve_enforces_auth_context_warehouse_scope() {
    let state = H8ErpConnectorAppState::with_memory();
    let owner = Uuid::new_v4();
    let allowed_warehouse = Uuid::new_v4();
    let denied_warehouse = Uuid::new_v4();
    let now = Utc::now();
    let c = H8ErpConnector {
        id: Uuid::new_v4(),
        owner_id: owner,
        connector_code: "route1".into(),
        connector_name: "R1".into(),
        warehouse_ids: vec![allowed_warehouse],
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
        interface_probe_db_username: None,
        interface_probe_db_password_alias: None,
        interface_probe_db_password_alias_set: false,
        interface_probe_config_version: 1,
        status: "active".into(),
        config_version: 1,
        first_activated_at: Some(now),
        last_tested_version: Some(1),
        last_tested_at: Some(now),
        last_tested_succeeded: Some(true),
        last_tested_error_summary: None,
        created_at: now,
        updated_at: now,
    };
    state.repository.insert(&c).await.unwrap();
    let mut denied_connector = c.clone();
    denied_connector.id = Uuid::new_v4();
    denied_connector.connector_code = "denied-route".into();
    denied_connector.warehouse_ids = vec![denied_warehouse];
    state.repository.insert(&denied_connector).await.unwrap();
    let ctx = AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: owner,
        actor_name: "scoped-caller".into(),
        permissions: vec![H8_CONFIG_READ.into()],
        jti: "scoped-caller-test".into(),
        warehouse_scope: Some(allowed_warehouse),
    };

    let mut denied = Request::builder()
        .uri(format!(
            "/api/v1/config/erp-connectors/route-resolve?direction=inbound&message_type=asn&warehouse_id={denied_warehouse}"
        ))
        .body(Body::empty())
        .unwrap();
    denied.extensions_mut().insert(ctx.clone());
    let response = h8_erp_connector_router(state.clone())
        .oneshot(denied)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let mut allowed = Request::builder()
        .uri("/api/v1/config/erp-connectors/route-resolve?direction=inbound&message_type=asn")
        .body(Body::empty())
        .unwrap();
    allowed.extensions_mut().insert(ctx);
    let response = h8_erp_connector_router(state)
        .oneshot(allowed)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["connector"]["id"], c.id.to_string());
}

#[tokio::test]
async fn connector_runtime_versions_are_immutable_and_owner_scoped() {
    let state = H8ErpConnectorAppState::with_memory();
    let owner = Uuid::new_v4();
    let connector = versioned_connector(owner);
    state.repository.insert(&connector).await.unwrap();
    let mut v2 = connector.clone();
    v2.api_base_url = Some("https://erp-v2.example.com".into());
    v2.config_version = 2;
    state.repository.save(&v2, 1, 1).await.unwrap();

    let ctx = AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: owner,
        actor_name: "worker".into(),
        permissions: vec![H8_CONFIG_READ.into()],
        jti: "worker-version-test".into(),
        warehouse_scope: None,
    };
    for (version, expected_url) in [
        (1, "https://erp-v1.example.com"),
        (2, "https://erp-v2.example.com"),
    ] {
        let mut request = Request::builder()
            .uri(format!(
                "/api/v1/config/erp-connectors/{}/versions/{version}",
                connector.id
            ))
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = h8_erp_connector_router(state.clone())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["config_version"], version);
        assert_eq!(value["api_base_url"], expected_url);
    }

    let mut cross_owner = Request::builder()
        .uri(format!(
            "/api/v1/config/erp-connectors/{}/versions/1",
            connector.id
        ))
        .body(Body::empty())
        .unwrap();
    cross_owner.extensions_mut().insert(AuthContext {
        owner_id: Uuid::new_v4(),
        ..ctx
    });
    let response = h8_erp_connector_router(state.clone())
        .oneshot(cross_owner)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    state.repository.delete(owner, connector.id).await.unwrap();
    assert!(state
        .repository
        .get_version(owner, connector.id, 1)
        .await
        .is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn postgres_captures_each_runtime_version_and_rejects_unversioned_change(pool: PgPool) {
    let owner = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$2)")
        .bind(owner)
        .bind(format!("H8-VERSION-{owner}"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors
           (id, owner_id, connector_code, connector_name, directions, message_types,
            channel_mode, api_base_url, status, config_version)
           VALUES ($1,$2,'SELF-ERP','Self ERP',ARRAY['inbound'],ARRAY['asn'],
                   'rest','https://erp-v1.example.com','testing',1)"#,
    )
    .bind(connector_id)
    .bind(owner)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE h8_erp_connectors SET api_base_url = $3, config_version = 2 WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner)
    .bind(connector_id)
    .bind("https://erp-v2.example.com")
    .execute(&pool)
    .await
    .unwrap();

    let repository = PgH8ErpConnectorRepository { pool: pool.clone() };
    let v1 = repository
        .get_version(owner, connector_id, 1)
        .await
        .unwrap();
    let v2 = repository
        .get_version(owner, connector_id, 2)
        .await
        .unwrap();
    assert_eq!(
        v1.api_base_url.as_deref(),
        Some("https://erp-v1.example.com")
    );
    assert_eq!(
        v2.api_base_url.as_deref(),
        Some("https://erp-v2.example.com")
    );
    assert!(repository
        .get_version(Uuid::new_v4(), connector_id, 1)
        .await
        .is_err());

    let unversioned_change = sqlx::query(
        "UPDATE h8_erp_connectors SET api_base_url = $3 WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner)
    .bind(connector_id)
    .bind("https://silently-changed.example.com")
    .execute(&pool)
    .await;
    assert!(unversioned_change.is_err());
}

#[test]
fn dedicated_permissions_are_h8_scoped() {
    assert_eq!(H8_CONFIG_READ, "h8.erp_connector.read");
    assert_eq!(H8_CONFIG_WRITE, "h8.erp_connector.write");
    assert!(!H8_CONFIG_READ.starts_with("m1."));
    assert!(!H8_CONFIG_WRITE.starts_with("m1."));
}
