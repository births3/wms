use axum::{body::to_bytes, body::Body, http::Request, Extension};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{auth::AuthContext, state_machine::state_machine_router};

#[path = "support/adversarial.rs"]
mod adversarial_support;
use adversarial_support::{ctx_with_permissions, seed_owner_pair};

fn h6_ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    ctx_with_permissions(owner_id, "h6-adversarial", permissions)
}

async fn list_machine_codes(owner_id: Uuid, permissions: &[&str]) -> (u16, Vec<String>) {
    let response = state_machine_router()
        .layer(Extension(h6_ctx(owner_id, permissions)))
        .oneshot(
            Request::builder()
                .uri("/api/v1/state-machines")
                .body(Body::empty())
                .expect("state machine list request should build"),
        )
        .await
        .expect("state machine list should respond");
    let status = response.status().as_u16();
    if status != 200 {
        return (status, Vec::new());
    }
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("state machine list body should read"),
    )
    .expect("state machine list should be JSON");
    let codes = payload["data"]
        .as_array()
        .expect("data should be an array")
        .iter()
        .map(|item| {
            item["machine_code"]
                .as_str()
                .expect("machine_code should be text")
                .to_string()
        })
        .collect();
    (status, codes)
}

#[sqlx::test(migrations = "../../migrations")]
async fn h6_permission_is_granted_only_to_seeded_system_admin_role(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let operator_role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H6 permission owner')",
    )
    .bind(owner_id)
    .bind(format!("H6-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("owner should insert");
    let system_admin_role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'system_admin'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("system admin role should be seeded");
    sqlx::query(
        r#"
        INSERT INTO auth_roles (id, owner_id, role_code, role_name)
        VALUES ($1, $2, 'warehouse_operator', '仓库操作员')
        "#,
    )
    .bind(operator_role_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("operator role should insert after migrations");

    let rows = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT role_permission.role_id, permission.permission_code
          FROM auth_role_permissions role_permission
          JOIN auth_permissions permission ON permission.id = role_permission.permission_id
         WHERE role_permission.role_id = ANY($1)
           AND permission.permission_code = 'h6.state_machine.read'
         ORDER BY role_permission.role_id
        "#,
    )
    .bind(vec![system_admin_role_id, operator_role_id])
    .fetch_all(&pool)
    .await
    .expect("H6 permission grants should query");

    assert_eq!(
        rows,
        vec![(system_admin_role_id, "h6.state_machine.read".to_string())]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn h6_catalog_has_no_owner_private_machines(pool: PgPool) {
    let pair = seed_owner_pair(&pool).await;
    let (status_a, codes_a) = list_machine_codes(pair.owner_a, &["h6.state_machine.read"]).await;
    let (status_b, codes_b) = list_machine_codes(pair.owner_b, &["h6.state_machine.read"]).await;
    let (denied, _) = list_machine_codes(pair.owner_b, &[]).await;
    assert_eq!(status_a, 200);
    assert_eq!(status_b, 200);
    assert_eq!(codes_a, codes_b);
    assert!(codes_a.contains(&"outbound_order".to_string()));
    assert_eq!(denied, 403);
}

#[sqlx::test(migrations = "../../migrations")]
async fn h6_transition_validation_rejects_illegal_skip(pool: PgPool) {
    let pair = seed_owner_pair(&pool).await;
    let response = state_machine_router()
        .layer(Extension(h6_ctx(
            pair.owner_a,
            &["h6.state_machine.read"],
        )))
        .oneshot(
            Request::builder()
                .uri("/api/v1/state-machines/outbound_order/transition-validation?from_state=in_wave&to_state=shipped&event_code=handover_confirmed")
                .body(Body::empty())
                .expect("transition request should build"),
        )
        .await
        .expect("transition route should respond");
    assert_eq!(response.status(), 200);
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("transition body should read"),
    )
    .expect("transition response should be JSON");
    assert_eq!(payload["allowed"], false);
}
