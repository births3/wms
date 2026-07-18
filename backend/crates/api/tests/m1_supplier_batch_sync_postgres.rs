use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    api_key_auth::{api_key_auth_middleware, ApiKeyAuthState},
    api_key_service::ApiKeyService,
    auth::AuthContext,
    master_data_handlers::{master_data_router, MasterDataAppState},
    master_data_postgres::PgMasterDataReadRepository,
};
use wms_domain::{CreateApiKeyRequest, CreateSupplierRequest, SupplierListResponse};

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m1-supplier-batch-test".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn request(code: &str) -> CreateSupplierRequest {
    CreateSupplierRequest {
        supplier_code: code.to_string(),
        supplier_name: format!("供应商 {code}"),
        license_no: Some(valid_uscc(code)),
        contact_name: Some("测试联系人".to_string()),
        source: Some("api_import".to_string()),
    }
}

fn valid_uscc(seed: &str) -> String {
    const CHARACTERS: &[u8] = b"0123456789ABCDEFGHJKLMNPQRTUWXY";
    const WEIGHTS: [usize; 17] = [
        1, 3, 9, 27, 19, 26, 16, 17, 20, 29, 25, 13, 8, 24, 10, 30, 28,
    ];
    let mut code = *b"91350100M000100Y0";
    code[16] = CHARACTERS[seed.bytes().map(usize::from).sum::<usize>() % CHARACTERS.len()];
    let checksum = WEIGHTS
        .iter()
        .zip(code)
        .map(|(weight, item)| {
            weight
                * CHARACTERS
                    .iter()
                    .position(|candidate| *candidate == item)
                    .unwrap_or_default()
        })
        .sum::<usize>();
    format!(
        "{}{}",
        String::from_utf8_lossy(&code),
        CHARACTERS[(31 - checksum % 31) % 31] as char
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn supplier_batch_sync_rejects_invalid_uscc(pool: PgPool) {
    let owner = context(Uuid::new_v4());
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone()));
    let mut invalid_request = request("INVALID-USCC");
    invalid_request.license_no = Some("INVALID-USCC".to_string());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/master-data/suppliers/batch-sync")
                .header("content-type", "application/json")
                .header("Idempotency-Key", "supplier-batch-invalid-uscc")
                .extension(owner.clone())
                .body(Body::from(
                    serde_json::to_vec(&vec![invalid_request])
                        .expect("invalid USCC request should serialize"),
                ))
                .expect("invalid USCC request should build"),
        )
        .await
        .expect("invalid USCC request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM suppliers WHERE owner_id = $1")
        .bind(owner.owner_id)
        .fetch_one(&pool)
        .await
        .expect("supplier count should query");
    assert_eq!(count, 0);
}

async fn seed_owner_and_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid) {
    sqlx::query("INSERT INTO auth_owners(id, owner_code, owner_name) VALUES ($1, $2, $3)")
        .bind(owner_id)
        .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
        .bind("供应商批量同步测试货主")
        .execute(pool)
        .await
        .expect("owner should seed");
    sqlx::query(
        "INSERT INTO auth_users(id, username, display_name, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(format!("supplier-batch-{}", &user_id.to_string()[..8]))
    .bind("供应商批量同步测试用户")
    .bind("test-hash")
    .execute(pool)
    .await
    .expect("user should seed");
    sqlx::query("INSERT INTO auth_user_owner_bindings(user_id, owner_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("owner binding should seed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn supplier_batch_sync_is_atomic_owner_scoped_idempotent_and_routed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let owner = context(owner_id);
    let other_owner = context(other_owner_id);
    let requests = vec![request("S-BATCH-001"), request("S-BATCH-002")];

    let created = repo
        .batch_create_suppliers(&owner, requests.clone(), Utc::now(), "supplier-batch-key-1")
        .await
        .expect("batch should create");
    assert_eq!(created.len(), 2);

    let replay = repo
        .batch_create_suppliers(&owner, requests, Utc::now(), "supplier-batch-key-1")
        .await
        .expect("same batch should replay");
    assert_eq!(
        replay.iter().map(|row| row.id).collect::<Vec<_>>(),
        created.iter().map(|row| row.id).collect::<Vec<_>>()
    );

    let conflict = repo
        .batch_create_suppliers(
            &owner,
            vec![request("S-BATCH-DIFFERENT")],
            Utc::now(),
            "supplier-batch-key-1",
        )
        .await;
    assert!(
        conflict.is_err(),
        "different request must conflict on reused key"
    );

    let failed = repo
        .batch_create_suppliers(
            &owner,
            vec![request("S-BATCH-003"), request("S-BATCH-001")],
            Utc::now(),
            "supplier-batch-key-2",
        )
        .await;
    assert!(failed.is_err(), "duplicate code must fail the whole batch");
    let owner_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM suppliers WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("owner supplier count");
    assert_eq!(owner_count, 2, "failed batch must roll back its first row");

    let same_code_other_owner = repo
        .batch_create_suppliers(
            &other_owner,
            vec![request("S-BATCH-001")],
            Utc::now(),
            "supplier-batch-key-other-owner",
        )
        .await
        .expect("same supplier code is isolated by owner");
    assert_eq!(same_code_other_owner[0].owner_id, other_owner_id);

    let app = master_data_router(MasterDataAppState::with_postgres(pool));
    let mut http_request = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/suppliers/batch-sync")
        .header("content-type", "application/json")
        .header("Idempotency-Key", "supplier-batch-route-key")
        .body(Body::from(
            serde_json::to_vec(&vec![request("S-BATCH-ROUTE")]).expect("request json"),
        ))
        .expect("batch request should build");
    http_request.extensions_mut().insert(owner);
    let response = app
        .oneshot(http_request)
        .await
        .expect("batch route should respond");
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let response: SupplierListResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("batch response body"),
    )
    .expect("batch response should decode");
    assert_eq!(response.data.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn supplier_batch_sync_accepts_shared_api_key_middleware(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let service = ApiKeyService::new(pool.clone());
    let key = service
        .create(
            &AuthContext {
                user_id,
                owner_id,
                actor_name: "供应商批量同步 API Key 测试".to_string(),
                permissions: vec!["h1.api_keys.manage".to_string()],
                jti: Uuid::new_v4().to_string(),
                warehouse_scope: None,
            },
            CreateApiKeyRequest {
                caller_name: "ERP 供应商同步".to_string(),
                purpose: "供应商档案批量同步".to_string(),
                warehouse_ids: Vec::new(),
                scopes: vec!["master-data:write".to_string()],
                expires_at: Some(Utc::now() + Duration::days(1)),
                responsible_user_id: user_id,
            },
            "supplier-batch-api-key-create",
        )
        .await
        .expect("api key should create")
        .secret
        .expect("api key secret should be returned once");

    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        from_fn_with_state(ApiKeyAuthState::new(pool.clone()), api_key_auth_middleware),
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/master-data/suppliers/batch-sync")
                .header("content-type", "application/json")
                .header("X-WMS-API-Key", key)
                .header("Idempotency-Key", "supplier-batch-api-key-request")
                .body(Body::from(
                    serde_json::to_vec(&vec![request("S-BATCH-API-KEY")]).expect("request json"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("batch route should respond");
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let response: SupplierListResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("batch response body"),
    )
    .expect("batch response should decode");
    assert_eq!(response.data[0].owner_id, owner_id);

    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_type = 'api_key_request' AND diff->'after'->>'path' = '/api/v1/master-data/suppliers/batch-sync'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("api key request audit should exist");
    assert_eq!(audited, 1);
}
