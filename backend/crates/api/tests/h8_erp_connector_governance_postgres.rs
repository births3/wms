use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    h8_erp_connectors::{h8_erp_connector_router, H8ErpConnectorAppState, H8_CONFIG_WRITE},
};

async fn seed_owner_user_and_connectors(pool: &PgPool) -> (AuthContext, Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let test_connector_id = Uuid::new_v4();
    let status_connector_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$2)")
        .bind(owner_id)
        .bind(format!("H8-CONNECTOR-GOV-{owner_id}"))
        .execute(pool)
        .await
        .expect("owner should insert");
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status)
        VALUES ($1,$2,'H8 连接器治理测试用户','not-used-in-test','active')
        "#,
    )
    .bind(user_id)
    .bind(format!("h8-connector-gov-{user_id}"))
    .execute(pool)
    .await
    .expect("user should insert");
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, directions, message_types,
            channel_mode, status, config_version
        )
        VALUES (
            $1,$2,'GOV-TEST','治理连接测试',ARRAY['outbound'],
            ARRAY['putaway_complete'],'rest','testing',1
        )
        "#,
    )
    .bind(test_connector_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("test connector should insert");
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, directions, message_types,
            channel_mode, api_base_url, bearer_secret_alias, status, config_version,
            first_activated_at, last_tested_version, last_tested_at, last_tested_succeeded
        )
        VALUES (
            $1,$2,'GOV-STATUS','治理状态切换',ARRAY['outbound'],
            ARRAY['putaway_complete'],'rest','https://erp.example.com',
            'vault://h8/governance/token','disabled',1,now(),1,now(),true
        )
        "#,
    )
    .bind(status_connector_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("status connector should insert");
    (
        AuthContext {
            user_id,
            owner_id,
            actor_name: "H8 连接器治理测试用户".to_string(),
            permissions: vec![H8_CONFIG_WRITE.to_string()],
            jti: format!("h8-connector-governance-{user_id}"),
            warehouse_scope: None,
        },
        test_connector_id,
        status_connector_id,
    )
}

fn mutation_request(ctx: &AuthContext, path: &str, idempotency_key: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("Idempotency-Key", idempotency_key)
        .body(Body::empty())
        .expect("request should build");
    request.extensions_mut().insert(ctx.clone());
    request
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_h8_erp_connector_activate_h8_erp_connector_and_disable_h8_erp_connector_replay_once(
    pool: PgPool,
) {
    let (ctx, test_connector_id, status_connector_id) = seed_owner_user_and_connectors(&pool).await;
    let app = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()));

    for (path, key) in [
        (
            format!("/api/v1/config/erp-connectors/{test_connector_id}/test"),
            "h8-governance-test",
        ),
        (
            format!("/api/v1/config/erp-connectors/{status_connector_id}/activate"),
            "h8-governance-activate",
        ),
        (
            format!("/api/v1/config/erp-connectors/{status_connector_id}/disable"),
            "h8-governance-disable",
        ),
    ] {
        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(mutation_request(&ctx, &path, key))
                .await
                .expect("connector mutation should respond");
            assert_eq!(response.status(), StatusCode::OK, "failed path: {path}");
        }
    }

    for action in [
        "h8_connector_test",
        "h8_connector_activate",
        "h8_connector_disable",
    ] {
        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event WHERE owner_id=$1 AND module='H8' AND action=$2",
        )
        .bind(ctx.owner_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("connector audit count should load");
        assert_eq!(audit_count, 1, "audit should be unique for {action}");
    }
    let idempotency_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM idempotency_request
         WHERE owner_id=$1
           AND idempotency_key=ANY($2)
        "#,
    )
    .bind(ctx.owner_id)
    .bind([
        "h8-governance-test",
        "h8-governance-activate",
        "h8-governance-disable",
    ])
    .fetch_one(&pool)
    .await
    .expect("connector idempotency rows should load");
    assert_eq!(idempotency_count, 3);
}
