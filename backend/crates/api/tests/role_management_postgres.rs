use std::{collections::HashMap, sync::Mutex};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthError,
        AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy,
    },
    role_management::{role_management_router, RoleListResponse, RoleManagementState},
};

#[derive(Default)]
struct MemoryRevocations(Mutex<HashMap<Uuid, i64>>);

struct FailingRevocations;

#[axum::async_trait]
impl AuthRevocationStore for FailingRevocations {
    async fn jti_is_blacklisted(&self, _: &str) -> Result<bool, AuthRevocationStoreError> {
        Err(AuthRevocationStoreError::Unavailable("offline".into()))
    }
    async fn permissions_changed_at(
        &self,
        _: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Err(AuthRevocationStoreError::Unavailable("offline".into()))
    }
    async fn blacklist_jti(&self, _: &str, _: u64) -> Result<(), AuthRevocationStoreError> {
        Err(AuthRevocationStoreError::Unavailable("offline".into()))
    }
    async fn set_permissions_changed_at(
        &self,
        _: Uuid,
        _: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Err(AuthRevocationStoreError::Unavailable("offline".into()))
    }
}

#[axum::async_trait]
impl AuthRevocationStore for MemoryRevocations {
    async fn jti_is_blacklisted(&self, _: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(false)
    }
    async fn permissions_changed_at(
        &self,
        user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|e| AuthRevocationStoreError::Unavailable(e.to_string()))?
            .get(&user_id)
            .copied())
    }
    async fn blacklist_jti(&self, _: &str, _: u64) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
    async fn set_permissions_changed_at(
        &self,
        user_id: Uuid,
        changed_at: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        self.0
            .lock()
            .map_err(|e| AuthRevocationStoreError::Unavailable(e.to_string()))?
            .insert(user_id, changed_at);
        Ok(())
    }
}

async fn seed(pool: &PgPool) -> (Uuid, Uuid, Uuid, String) {
    let owner = Uuid::new_v4();
    let other = Uuid::new_v4();
    let admin = Uuid::new_v4();
    for (id, code) in [(owner, "OWNER-A"), (other, "OWNER-B")] {
        sqlx::query("INSERT INTO auth_owners(id, owner_code, owner_name) VALUES ($1,$2,$2)")
            .bind(id)
            .bind(code)
            .execute(pool)
            .await
            .expect("seed owner");
    }
    sqlx::query(
        "INSERT INTO auth_users(id,username,display_name,password_hash) VALUES ($1,$2,$2,'x')",
    )
    .bind(admin)
    .bind(format!("admin-{admin}"))
    .execute(pool)
    .await
    .expect("seed admin");
    sqlx::query("INSERT INTO auth_user_owner_bindings(user_id,owner_id) VALUES ($1,$2)")
        .bind(admin)
        .bind(owner)
        .execute(pool)
        .await
        .expect("bind admin");
    sqlx::query("INSERT INTO auth_permissions(id,permission_code,permission_name) VALUES ($1,'h1.roles.manage','角色管理'),($2,'m2.receive','收货') ON CONFLICT DO NOTHING").bind(Uuid::new_v4()).bind(Uuid::new_v4()).execute(pool).await.expect("seed permissions");
    let claims = build_access_claims(
        admin,
        owner,
        "管理员",
        vec!["h1.roles.manage".into()],
        "test-jti",
        chrono::Utc::now(),
    );
    (
        owner,
        other,
        admin,
        encode_access_token(&claims, "role-test-secret").expect("token"),
    )
}

fn json_request(
    method: &str,
    uri: String,
    token: &str,
    key: Option<&str>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");
    if let Some(key) = key {
        builder = builder.header("idempotency-key", key);
    }
    builder.body(Body::from(body.to_string())).expect("request")
}

#[sqlx::test(migrations = "../../migrations")]
async fn role_writes_are_tenant_safe_atomic_idempotent_audited_and_revoke_tokens(pool: PgPool) {
    std::env::set_var("WMS_JWT_SECRET", "role-test-secret");
    let (owner, other, admin, token) = seed(&pool).await;
    let store = std::sync::Arc::new(MemoryRevocations::default());
    let app = role_management_router(RoleManagementState::new(pool.clone(), store.clone()))
        .layer(auth_runtime_layer(AuthRuntimePolicy::strict(store.clone())));

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth_roles WHERE owner_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .expect("default role count"),
        8
    );

    let create = serde_json::json!({"role_code":"checker","role_name":"复核岗","data_scope":"warehouse","parent_role_id":null});
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/roles".into(),
            &token,
            Some("create-1"),
            create.clone(),
        ))
        .await
        .expect("create response");
    assert_eq!(response.status(), StatusCode::OK);
    let replay = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/roles".into(),
            &token,
            Some("create-1"),
            create,
        ))
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM auth_roles WHERE owner_id=$1 AND role_code='checker'"
        )
        .bind(owner)
        .fetch_one(&pool)
        .await
        .expect("role count"),
        1
    );
    let role_id: Uuid =
        sqlx::query_scalar("SELECT id FROM auth_roles WHERE owner_id=$1 AND role_code='checker'")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .expect("role id");

    let duplicate = app.clone().oneshot(json_request("POST", "/api/v1/auth/roles".into(), &token, Some("create-2"), serde_json::json!({"role_code":"checker","role_name":"重复","data_scope":"self","parent_role_id":null}))).await.expect("duplicate response");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    let unknown = app
        .clone()
        .oneshot(json_request(
            "PUT",
            format!("/api/v1/auth/roles/{role_id}/permissions"),
            &token,
            Some("perm-1"),
            serde_json::json!({"permission_codes":["unknown.code"]}),
        ))
        .await
        .expect("unknown permission response");
    assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth_role_permissions WHERE role_id=$1")
            .bind(role_id)
            .fetch_one(&pool)
            .await
            .expect("permission count"),
        0
    );

    let foreign_role = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_roles(id,owner_id,role_code,role_name) VALUES ($1,$2,'foreign','Foreign')").bind(foreign_role).bind(other).execute(&pool).await.expect("foreign role");
    let cross = app
        .clone()
        .oneshot(json_request(
            "PUT",
            format!("/api/v1/auth/roles/{foreign_role}"),
            &token,
            Some("cross-1"),
            serde_json::json!({"role_name":"hacked","data_scope":"all","parent_role_id":null}),
        ))
        .await
        .expect("cross owner response");
    assert_eq!(cross.status(), StatusCode::FORBIDDEN);

    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();
    for user in [user1, user2] {
        sqlx::query(
            "INSERT INTO auth_users(id,username,display_name,password_hash) VALUES ($1,$2,$2,'x')",
        )
        .bind(user)
        .bind(user.to_string())
        .execute(&pool)
        .await
        .expect("user");
        sqlx::query("INSERT INTO auth_user_owner_bindings(user_id,owner_id) VALUES ($1,$2)")
            .bind(user)
            .bind(owner)
            .execute(&pool)
            .await
            .expect("binding");
    }
    let stale_claims = build_access_claims(
        user1,
        owner,
        "待授权用户",
        Vec::new(),
        "stale-jti",
        chrono::Utc::now(),
    );
    let batch = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/auth/user-roles/batch".into(),
            &token,
            Some("batch-1"),
            serde_json::json!({"user_ids":[user1,user2],"role_ids":[role_id,foreign_role]}),
        ))
        .await
        .expect("batch response");
    assert_eq!(batch.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth_user_roles WHERE user_id=ANY($1)")
            .bind(vec![user1, user2])
            .fetch_one(&pool)
            .await
            .expect("atomic role count"),
        0
    );

    let ok = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/auth/user-roles/batch".into(),
            &token,
            Some("batch-2"),
            serde_json::json!({"user_ids":[user1,user2],"role_ids":[role_id]}),
        ))
        .await
        .expect("batch ok");
    assert_eq!(ok.status(), StatusCode::OK);
    assert!(store
        .permissions_changed_at(user1)
        .await
        .expect("revocation read")
        .is_some());
    assert_eq!(
        AuthRuntimePolicy::strict(store.clone())
            .validate_claims(&stale_claims)
            .await,
        Err(AuthError::PermissionsRevoked)
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM audit_event WHERE owner_id=$1 AND actor_id=$2 AND resource_type IN ('auth_role','auth_user_roles')").bind(owner).bind(admin).fetch_one(&pool).await.expect("audit count"),2);

    for path in ["/api/v1/auth/permissions", "/api/v1/auth/users"] {
        let response = app
            .clone()
            .oneshot(json_request(
                "GET",
                path.into(),
                &token,
                None,
                serde_json::Value::Null,
            ))
            .await
            .expect("catalog response");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let in_use = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            format!("/api/v1/auth/roles/{role_id}"),
            &token,
            Some("delete-in-use"),
            serde_json::Value::Null,
        ))
        .await
        .expect("delete in-use role");
    assert_eq!(in_use.status(), StatusCode::CONFLICT);
    let disposable = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_roles(id,owner_id,role_code,role_name) VALUES($1,$2,'disposable','临时角色')",
    )
    .bind(disposable)
    .bind(owner)
    .execute(&pool)
    .await
    .expect("disposable role");
    for _ in 0..2 {
        let deleted = app
            .clone()
            .oneshot(json_request(
                "DELETE",
                format!("/api/v1/auth/roles/{disposable}"),
                &token,
                Some("delete-disposable"),
                serde_json::Value::Null,
            ))
            .await
            .expect("delete role");
        assert_eq!(deleted.status(), StatusCode::OK);
    }

    let parent = Uuid::new_v4();
    let child = Uuid::new_v4();
    let grandchild = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_roles(id,owner_id,role_code,role_name,parent_role_id) VALUES ($1,$4,'parent','父角色',NULL),($2,$4,'child','子角色',$1),($3,$4,'grandchild','孙角色',$2)",
    )
    .bind(parent)
    .bind(child)
    .bind(grandchild)
    .bind(owner)
    .execute(&pool)
    .await
    .expect("role hierarchy");
    let receive_permission: Uuid =
        sqlx::query_scalar("SELECT id FROM auth_permissions WHERE permission_code='m2.receive'")
            .fetch_one(&pool)
            .await
            .expect("receive permission");
    sqlx::query("INSERT INTO auth_role_permissions(role_id,permission_id) VALUES($1,$2)")
        .bind(parent)
        .bind(receive_permission)
        .execute(&pool)
        .await
        .expect("parent grant");
    sqlx::query("INSERT INTO auth_role_permission_exclusions(role_id,permission_id) VALUES($1,$2)")
        .bind(child)
        .bind(receive_permission)
        .execute(&pool)
        .await
        .expect("child exclusion");

    let list = app
        .oneshot(json_request(
            "GET",
            "/api/v1/auth/roles".into(),
            &token,
            None,
            serde_json::Value::Null,
        ))
        .await
        .expect("role list");
    assert_eq!(list.status(), StatusCode::OK);
    let list: RoleListResponse = serde_json::from_slice(
        &to_bytes(list.into_body(), usize::MAX)
            .await
            .expect("role list body"),
    )
    .expect("role list json");
    let grandchild = list
        .items
        .iter()
        .find(|role| role.id == grandchild)
        .expect("grandchild role");
    assert!(!grandchild
        .permission_codes
        .contains(&"m2.receive".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn permission_change_rolls_back_when_immediate_revocation_fails(pool: PgPool) {
    std::env::set_var("WMS_JWT_SECRET", "role-test-secret");
    let (owner, _, admin, token) = seed(&pool).await;
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id=$1 AND role_code='system_admin'",
    )
    .bind(owner)
    .fetch_one(&pool)
    .await
    .expect("system admin role");
    sqlx::query("INSERT INTO auth_user_roles(user_id,owner_id,role_id) VALUES($1,$2,$3)")
        .bind(admin)
        .bind(owner)
        .bind(role_id)
        .execute(&pool)
        .await
        .expect("admin role binding");

    let runtime_store = std::sync::Arc::new(MemoryRevocations::default());
    let app = role_management_router(RoleManagementState::new(
        pool.clone(),
        std::sync::Arc::new(FailingRevocations),
    ))
    .layer(auth_runtime_layer(AuthRuntimePolicy::strict(runtime_store)));
    let response = app
        .oneshot(json_request(
            "PUT",
            format!("/api/v1/auth/roles/{role_id}"),
            &token,
            Some("rollback-on-revocation-failure"),
            serde_json::json!({
                "role_name": "不应提交",
                "data_scope": "all",
                "parent_role_id": null
            }),
        ))
        .await
        .expect("role update response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let role_name: String = sqlx::query_scalar("SELECT role_name FROM auth_roles WHERE id=$1")
        .bind(role_id)
        .fetch_one(&pool)
        .await
        .expect("role name");
    assert_eq!(role_name, "系统管理员");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM idempotency_request WHERE owner_id=$1 AND idempotency_key='rollback-on-revocation-failure'",
        )
        .bind(owner)
        .fetch_one(&pool)
        .await
        .expect("idempotency count"),
        0
    );
}
