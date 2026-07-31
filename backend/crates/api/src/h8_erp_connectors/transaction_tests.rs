//! H8 ERP 连接状态切换的事务与错误边界回归测试。

use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
    response::IntoResponse,
};
use sqlx::PgPool;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::Notify;
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::{H8ErpConnector, H8ErpConnectorRuntimeConfig, H8ErpConnectorTestResult};

use super::{
    error::{H8ErpConnectorHandlerError, H8ErpConnectorRepoError},
    handlers::h8_erp_connector_router,
    idempotency::H8IdempotencyWrite,
    repository::{H8ConnectorStatusTransition, H8ErpConnectorRepository},
    state::{H8ErpConnectorAppState, H8_CONFIG_WRITE},
};
use crate::{audit::AuditWriteRequest, auth::AuthContext};

struct DeleteRaceRepository {
    inner: Arc<dyn H8ErpConnectorRepository>,
    get_calls: AtomicUsize,
    second_get_reached: Notify,
    first_committed: Notify,
}

#[axum::async_trait]
impl H8ErpConnectorRepository for DeleteRaceRepository {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        self.inner.list(owner_id).await
    }

    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        if self.get_calls.fetch_add(1, Ordering::SeqCst) == 1 {
            self.second_get_reached.notify_one();
            self.first_committed.notified().await;
        }
        self.inner.get(owner_id, id).await
    }

    async fn get_version(
        &self,
        owner_id: Uuid,
        id: Uuid,
        config_version: i64,
    ) -> Result<H8ErpConnectorRuntimeConfig, H8ErpConnectorRepoError> {
        self.inner.get_version(owner_id, id, config_version).await
    }

    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.inner.insert(connector).await
    }

    async fn commit_create(
        &self,
        connector: &H8ErpConnector,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.inner
            .commit_create(connector, audit_request, idempotency)
            .await
    }

    async fn save(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.inner
            .save(connector, observed_version, observed_probe_version)
            .await
    }

    async fn commit_update(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.inner
            .commit_update(
                connector,
                observed_version,
                observed_probe_version,
                audit_request,
                idempotency,
            )
            .await
    }

    async fn commit_test(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        result: &H8ErpConnectorTestResult,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnectorTestResult, H8ErpConnectorRepoError> {
        self.inner
            .commit_test(
                connector,
                observed_version,
                observed_probe_version,
                result,
                audit_request,
                idempotency,
            )
            .await
    }

    async fn commit_status_transition(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        transition: H8ConnectorStatusTransition,
        audit_request: &AuditWriteRequest,
        inflight_audit_request: Option<&AuditWriteRequest>,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<(H8ErpConnector, u64), H8ErpConnectorRepoError> {
        self.inner
            .commit_status_transition(
                connector,
                observed_version,
                observed_probe_version,
                transition,
                audit_request,
                inflight_audit_request,
                idempotency,
            )
            .await
    }

    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError> {
        self.inner.delete(owner_id, id).await
    }

    async fn commit_delete(
        &self,
        owner_id: Uuid,
        id: Uuid,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<(), H8ErpConnectorRepoError> {
        self.second_get_reached.notified().await;
        let result = self
            .inner
            .commit_delete(owner_id, id, audit_request, idempotency)
            .await;
        self.first_committed.notify_waiters();
        result
    }

    async fn list_active(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        self.inner.list_active(owner_id).await
    }

    async fn has_inflight_refs(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<bool, H8ErpConnectorRepoError> {
        self.inner.has_inflight_refs(owner_id, connector_id).await
    }

    async fn pause_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        self.inner.pause_inflight(owner_id, connector_id).await
    }

    async fn resume_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        self.inner.resume_inflight(owner_id, connector_id).await
    }

    async fn load_api_key_scopes(
        &self,
        owner_id: Uuid,
        api_key_id: Uuid,
    ) -> Result<Option<Vec<String>>, H8ErpConnectorRepoError> {
        self.inner.load_api_key_scopes(owner_id, api_key_id).await
    }

    async fn bind_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
        idempotency_key: &str,
        direction: &str,
        message_type: &str,
        channel_stage: &str,
        status: &str,
    ) -> Result<(), H8ErpConnectorRepoError> {
        self.inner
            .bind_inflight(
                owner_id,
                connector_id,
                idempotency_key,
                direction,
                message_type,
                channel_stage,
                status,
            )
            .await
    }
}

fn connector_admin(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "H8 connector admin".into(),
        permissions: vec![H8_CONFIG_WRITE.into()],
        jti: format!("h8-connector-admin-{}", Uuid::new_v4()),
        warehouse_scope: None,
    }
}

async fn seed_connector_with_inflight(
    pool: &PgPool,
    connector_status: &str,
    inflight_status: &str,
) -> (Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$2)")
        .bind(owner_id)
        .bind(format!("H8-CONNECTOR-ATOMIC-{owner_id}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors (
               id, owner_id, connector_code, connector_name, directions, message_types,
               channel_mode, api_base_url, bearer_secret_alias, status, config_version,
               first_activated_at, last_tested_version, last_tested_at, last_tested_succeeded
           ) VALUES (
               $1,$2,'ATOMIC-ERP','Atomic ERP',ARRAY['outbound'],
               ARRAY['putaway_complete'],'rest','https://erp.example.com',
               'vault://h8/atomic/token',$3,1,
               CASE WHEN $3 = 'active' THEN now() ELSE NULL END,
               1,now(),true
           )"#,
    )
    .bind(connector_id)
    .bind(owner_id)
    .bind(connector_status)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO h8_erp_in_flight_messages (
               id, owner_id, connector_id, idempotency_key, direction, message_type,
               channel_stage, status
           ) VALUES ($1,$2,$3,$4,'outbound','putaway_complete','rest',$5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(connector_id)
    .bind(format!("atomic-inflight-{connector_id}"))
    .bind(inflight_status)
    .execute(pool)
    .await
    .unwrap();
    (owner_id, connector_id)
}

async fn seed_testing_connector(pool: &PgPool, probe_ready: bool) -> (Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$2)")
        .bind(owner_id)
        .bind(format!("H8-CONNECTOR-WRITE-{owner_id}"))
        .execute(pool)
        .await
        .unwrap();
    let api_key_id = seed_receipt_api_key(pool, owner_id).await;
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors (
               id, owner_id, connector_code, connector_name, directions, message_types,
               channel_mode, api_base_url, api_key_id, bearer_secret_alias, status, config_version
           ) VALUES (
               $1,$2,'WRITE-ERP','Write ERP',ARRAY['outbound'],
               ARRAY['putaway_complete'],'rest',$3,$4,$5,'testing',1
           )"#,
    )
    .bind(connector_id)
    .bind(owner_id)
    .bind(probe_ready.then_some("https://erp.example.com"))
    .bind(probe_ready.then_some(api_key_id))
    .bind(probe_ready.then_some("vault://h8/write/token"))
    .execute(pool)
    .await
    .unwrap();
    (owner_id, connector_id)
}

async fn seed_receipt_api_key(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash) VALUES ($1,$2,$2,'test-hash')",
    )
    .bind(user_id)
    .bind(format!("h8-receipt-user-{user_id}"))
    .execute(pool)
    .await
    .unwrap();
    let api_key_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_api_keys (
               id, owner_id, caller_name, purpose, scopes, responsible_user_id,
               key_hash, status, expires_at
           ) VALUES (
               $1,$2,'H8 ERP receipt','connector transaction test',
               ARRAY['outbound:receipt'],$3,$4,'active',now() + INTERVAL '30 days'
           )"#,
    )
    .bind(api_key_id)
    .bind(owner_id)
    .bind(user_id)
    .bind(format!("test-hash-{api_key_id}"))
    .execute(pool)
    .await
    .unwrap();
    api_key_id
}

fn json_request(
    owner_id: Uuid,
    method: Method,
    uri: &str,
    idempotency_key: &str,
    body: serde_json::Value,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(body.to_string()))
        .unwrap();
    request.extensions_mut().insert(connector_admin(owner_id));
    request
}

fn empty_request(
    owner_id: Uuid,
    method: Method,
    uri: &str,
    idempotency_key: &str,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("Idempotency-Key", idempotency_key)
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(connector_admin(owner_id));
    request
}

async fn reject_audit_action(pool: &PgPool, action: &str) {
    sqlx::query(
        r#"
        CREATE FUNCTION reject_h8_connector_write_audit() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'forced H8 connector audit failure';
        END;
        $$;
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE TRIGGER reject_h8_connector_write_audit \
         BEFORE INSERT ON audit_event FOR EACH ROW \
         WHEN (NEW.action = '{action}') \
         EXECUTE FUNCTION reject_h8_connector_write_audit()"
    ))
    .execute(pool)
    .await
    .unwrap();
}

async fn reject_idempotency_path(pool: &PgPool, path: &str) {
    crate::idempotency::reject_insert_path_for_test(pool, path)
        .await
        .unwrap();
}

async fn count_idempotency(pool: &PgPool, owner_id: Uuid, key: &str) -> i64 {
    crate::idempotency::count_for_test(pool, owner_id, key)
        .await
        .unwrap()
}

fn status_request(owner_id: Uuid, connector_id: Uuid, action: &str) -> Request<Body> {
    empty_request(
        owner_id,
        Method::POST,
        &format!("/api/v1/config/erp-connectors/{connector_id}/{action}"),
        &format!("atomic-{action}-{connector_id}"),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_audit_failure_rolls_back_connector_and_idempotency(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$2)")
        .bind(owner_id)
        .bind(format!("H8-CONNECTOR-CREATE-{owner_id}"))
        .execute(&pool)
        .await
        .unwrap();
    let api_key_id = seed_receipt_api_key(&pool, owner_id).await;
    reject_audit_action(&pool, "h8_connector_create").await;
    let request = json_request(
        owner_id,
        Method::POST,
        "/api/v1/config/erp-connectors",
        "atomic-create",
        serde_json::json!({
            "connector_code": "ATOMIC-CREATE",
            "connector_name": "Atomic Create",
            "warehouse_ids": [],
            "directions": ["outbound"],
            "message_types": ["putaway_complete"],
            "channel_mode": "rest",
            "api_base_url": "https://erp.example.com",
            "api_key_id": api_key_id,
            "bearer_secret_alias": "vault://h8/atomic/token"
        }),
    );

    let response = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let evidence: (i64, i64) = sqlx::query_as(
        r#"SELECT
               (SELECT COUNT(*) FROM h8_erp_connectors WHERE owner_id=$1),
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id=$1 AND action='h8_connector_create')"#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            evidence.0,
            evidence.1,
            count_idempotency(&pool, owner_id, "atomic-create").await
        ),
        (0, 0, 0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_idempotency_failure_rolls_back_connector_and_audit(pool: PgPool) {
    let (owner_id, connector_id) = seed_testing_connector(&pool, true).await;
    let path = format!("/api/v1/config/erp-connectors/{connector_id}");
    reject_idempotency_path(&pool, &path).await;
    let request = json_request(
        owner_id,
        Method::PATCH,
        &path,
        "atomic-update",
        serde_json::json!({
            "expected_config_version": 1,
            "expected_probe_config_version": 1,
            "connector_name": "Changed ERP"
        }),
    );

    let response = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let evidence: (String, i64) = sqlx::query_as(
        r#"SELECT c.connector_name,
                  (SELECT COUNT(*) FROM audit_event
                    WHERE owner_id=$1 AND action='h8_connector_update')
             FROM h8_erp_connectors c
            WHERE c.owner_id=$1 AND c.id=$2"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            evidence.0,
            evidence.1,
            count_idempotency(&pool, owner_id, "atomic-update").await
        ),
        ("Write ERP".into(), 0, 0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_audit_failure_rolls_back_result_and_idempotency(pool: PgPool) {
    let (owner_id, connector_id) = seed_testing_connector(&pool, false).await;
    reject_audit_action(&pool, "h8_connector_test").await;
    let request = empty_request(
        owner_id,
        Method::POST,
        &format!("/api/v1/config/erp-connectors/{connector_id}/test"),
        "atomic-test",
    );

    let response = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let evidence: (Option<bool>, i64) = sqlx::query_as(
        r#"SELECT c.last_tested_succeeded,
                  (SELECT COUNT(*) FROM audit_event
                    WHERE owner_id=$1 AND action='h8_connector_test')
             FROM h8_erp_connectors c
            WHERE c.owner_id=$1 AND c.id=$2"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            evidence.0,
            evidence.1,
            count_idempotency(&pool, owner_id, "atomic-test").await
        ),
        (None, 0, 0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_idempotency_failure_rolls_back_connector_and_audit(pool: PgPool) {
    let (owner_id, connector_id) = seed_testing_connector(&pool, true).await;
    let path = format!("/api/v1/config/erp-connectors/{connector_id}");
    reject_idempotency_path(&pool, &path).await;
    let request = empty_request(owner_id, Method::DELETE, &path, "atomic-delete");

    let response = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let evidence: (i64, i64) = sqlx::query_as(
        r#"SELECT
               (SELECT COUNT(*) FROM h8_erp_connectors WHERE owner_id=$1 AND id=$2),
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id=$1 AND action='h8_connector_delete')"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            evidence.0,
            evidence.1,
            count_idempotency(&pool, owner_id, "atomic-delete").await
        ),
        (1, 0, 0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_idempotency_failure_rolls_back_connector_inflight_and_audit(pool: PgPool) {
    let (owner_id, connector_id) = seed_connector_with_inflight(&pool, "active", "running").await;
    let path = format!("/api/v1/config/erp-connectors/{connector_id}/disable");
    reject_idempotency_path(&pool, &path).await;

    let response = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()))
        .oneshot(status_request(owner_id, connector_id, "disable"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let evidence: (String, String, i64) = sqlx::query_as(
        r#"SELECT c.status, f.status,
                  (SELECT COUNT(*) FROM audit_event
                    WHERE owner_id=$1 AND action='h8_connector_disable')
             FROM h8_erp_connectors c
             JOIN h8_erp_in_flight_messages f ON f.connector_id=c.id
            WHERE c.owner_id=$1 AND c.id=$2"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            evidence.0,
            evidence.1,
            evidence.2,
            count_idempotency(&pool, owner_id, &format!("atomic-disable-{connector_id}")).await
        ),
        ("active".into(), "running".into(), 0, 0)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_replay_returns_no_content_without_duplicate_audit(pool: PgPool) {
    let (owner_id, connector_id) = seed_testing_connector(&pool, true).await;
    let path = format!("/api/v1/config/erp-connectors/{connector_id}");
    let app = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()));

    for attempt in 1..=2 {
        let response = app
            .clone()
            .oneshot(empty_request(
                owner_id,
                Method::DELETE,
                &path,
                "atomic-delete-replay",
            ))
            .await
            .unwrap();
        let status = response.status();
        if status != StatusCode::NO_CONTENT {
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            panic!(
                "attempt {attempt}: expected 204, got {status}: {}",
                String::from_utf8_lossy(&body)
            );
        }
    }

    let evidence: (i64, i64) = sqlx::query_as(
        r#"SELECT
               (SELECT COUNT(*) FROM h8_erp_connectors WHERE owner_id=$1 AND id=$2),
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id=$1 AND action='h8_connector_delete')"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            evidence.0,
            evidence.1,
            count_idempotency(&pool, owner_id, "atomic-delete-replay").await
        ),
        (0, 1, 1)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_delete_rechecks_idempotency_after_not_found(pool: PgPool) {
    let (owner_id, connector_id) = seed_testing_connector(&pool, true).await;
    let mut state = H8ErpConnectorAppState::with_postgres(pool.clone());
    state.repository = Arc::new(DeleteRaceRepository {
        inner: state.repository.clone(),
        get_calls: AtomicUsize::new(0),
        second_get_reached: Notify::new(),
        first_committed: Notify::new(),
    });
    let app = h8_erp_connector_router(state);
    let path = format!("/api/v1/config/erp-connectors/{connector_id}");

    let (first, second) = tokio::join!(
        app.clone().oneshot(empty_request(
            owner_id,
            Method::DELETE,
            &path,
            "atomic-delete-concurrent",
        )),
        app.oneshot(empty_request(
            owner_id,
            Method::DELETE,
            &path,
            "atomic-delete-concurrent",
        )),
    );
    assert_eq!(first.unwrap().status(), StatusCode::NO_CONTENT);
    assert_eq!(second.unwrap().status(), StatusCode::NO_CONTENT);

    let evidence: (i64, i64) = sqlx::query_as(
        r#"SELECT
               (SELECT COUNT(*) FROM h8_erp_connectors WHERE owner_id=$1 AND id=$2),
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id=$1 AND action='h8_connector_delete')"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            evidence.0,
            evidence.1,
            count_idempotency(&pool, owner_id, "atomic-delete-concurrent").await
        ),
        (0, 1, 1)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn disable_audit_failure_rolls_back_connector_and_inflight(pool: PgPool) {
    let (owner_id, connector_id) = seed_connector_with_inflight(&pool, "active", "running").await;
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION reject_h8_connector_disable_audit() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.action = 'h8_connector_disable' THEN
                RAISE EXCEPTION 'forced H8 connector disable audit failure';
            END IF;
            RETURN NEW;
        END;
        $$;
        CREATE TRIGGER reject_h8_connector_disable_audit
        BEFORE INSERT ON audit_event
        FOR EACH ROW EXECUTE FUNCTION reject_h8_connector_disable_audit();
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let response = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()))
        .oneshot(status_request(owner_id, connector_id, "disable"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let evidence: (String, String, i64) = sqlx::query_as(
        r#"SELECT c.status, f.status,
                  (SELECT COUNT(*) FROM audit_event
                    WHERE owner_id=$1 AND action='h8_connector_disable')
             FROM h8_erp_connectors c
             JOIN h8_erp_in_flight_messages f ON f.connector_id=c.id
            WHERE c.owner_id=$1 AND c.id=$2"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(evidence, ("active".into(), "running".into(), 0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn activate_resume_failure_rolls_back_connector_and_audit(pool: PgPool) {
    let (owner_id, connector_id) = seed_connector_with_inflight(&pool, "disabled", "paused").await;
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION reject_h8_connector_resume() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            IF OLD.status = 'paused' AND NEW.status = 'running' THEN
                RAISE EXCEPTION 'forced H8 connector resume failure';
            END IF;
            RETURN NEW;
        END;
        $$;
        CREATE TRIGGER reject_h8_connector_resume
        BEFORE UPDATE ON h8_erp_in_flight_messages
        FOR EACH ROW EXECUTE FUNCTION reject_h8_connector_resume();
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let response = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()))
        .oneshot(status_request(owner_id, connector_id, "activate"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let evidence: (String, String, i64) = sqlx::query_as(
        r#"SELECT c.status, f.status,
                  (SELECT COUNT(*) FROM audit_event
                    WHERE owner_id=$1 AND action='h8_connector_activate')
             FROM h8_erp_connectors c
             JOIN h8_erp_in_flight_messages f ON f.connector_id=c.id
            WHERE c.owner_id=$1 AND c.id=$2"#,
    )
    .bind(owner_id)
    .bind(connector_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(evidence, ("disabled".into(), "paused".into(), 0));
}

#[tokio::test]
async fn database_error_response_does_not_expose_internal_details() {
    let secret = "password=plain-text-secret relation=h8_private_table";
    let response = H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(secret.into()))
        .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["message"], "database operation failed");
    assert!(!String::from_utf8_lossy(&bytes).contains(secret));
}
