use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthContext,
        AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    document_numbering::{
        DocumentNumberAllocation, DocumentNumberingError, GenerateDocumentNumberRequest,
        IdempotentMutation, PgDocumentNumberingService, SetDocumentNumberRuleEnabledRequest,
        UpsertDocumentNumberRuleRequest,
    },
    document_numbering_handlers::{document_numbering_router, DocumentNumberingAppState},
};
use wms_domain::{
    DocumentNumberAllocationListResponse, DOCUMENT_TYPE_PURCHASE_INBOUND,
    DOCUMENT_TYPE_SALES_RETURN,
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
        actor_name: "document-numbering-test".to_string(),
        permissions: vec![],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_owner(pool: &PgPool, owner_id: Uuid, owner_code: &str) {
    sqlx::query(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(owner_id)
    .bind(owner_code)
    .bind(format!("{owner_code} owner"))
    .execute(pool)
    .await
    .expect("owner seed should insert");
}

async fn seed_daily_rule(pool: &PgPool, owner_id: Uuid, document_type: &str, width: i32) -> Uuid {
    let rule_id = Uuid::new_v4();
    let rule_code = format!("{document_type}-daily");
    sqlx::query(
        r#"
        INSERT INTO document_number_rules (
            id, owner_id, document_type, rule_code, rule_name, template,
            reset_policy, sequence_width, enabled, effective_from, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $6, '单据号日流水',
            '{OWNER}-{DOCUMENT_TYPE}-{YYYY}{MM}{DD}-{SEQ}',
            'daily', $4, TRUE, $5, $5, $5
        )
        "#,
    )
    .bind(rule_id)
    .bind(owner_id)
    .bind(document_type)
    .bind(width)
    .bind(
        Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0)
            .single()
            .expect("valid time"),
    )
    .bind(rule_code)
    .execute(pool)
    .await
    .expect("document number rule seed should insert");
    rule_id
}

fn request(idempotency_key: &str) -> GenerateDocumentNumberRequest {
    request_for(DOCUMENT_TYPE_PURCHASE_INBOUND, idempotency_key)
}

fn request_for(document_type: &str, idempotency_key: &str) -> GenerateDocumentNumberRequest {
    GenerateDocumentNumberRequest {
        document_type: document_type.to_string(),
        idempotency_key: idempotency_key.to_string(),
        source_module: "M2".to_string(),
        source_document_id: Some(Uuid::new_v4()),
    }
}

fn bearer_token(owner_id: Uuid, permissions: Vec<&str>) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "document-numbering-reader",
        permissions.into_iter().map(str::to_string).collect(),
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
}

fn rule_request(width: i32) -> UpsertDocumentNumberRuleRequest {
    UpsertDocumentNumberRuleRequest {
        document_type: DOCUMENT_TYPE_PURCHASE_INBOUND.to_string(),
        rule_name: "采购入库 API 日流水".to_string(),
        template: "{OWNER}-{DOCUMENT_TYPE}-{YYYY}{MM}{DD}-{SEQ}".to_string(),
        reset_policy: "daily".to_string(),
        sequence_width: width,
        enabled: true,
        effective_from: Some(
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0)
                .single()
                .expect("valid time"),
        ),
        effective_to: None,
    }
}

async fn generate_and_commit(
    pool: &PgPool,
    service: &PgDocumentNumberingService,
    ctx: &AuthContext,
    req: GenerateDocumentNumberRequest,
    now: chrono::DateTime<Utc>,
) -> Result<IdempotentMutation<DocumentNumberAllocation>, DocumentNumberingError> {
    let mut tx = pool.begin().await.expect("transaction should begin");
    let generated = service.generate_in_tx(&mut tx, ctx, req, now).await?;
    tx.commit().await.expect("transaction should commit");
    Ok(generated)
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_number_rule_management_is_owner_scoped_idempotent_and_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "HZ005").await;
    seed_owner(&pool, other_owner_id, "HZ006").await;
    let service = PgDocumentNumberingService::new();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 2, 9, 0, 0)
        .single()
        .expect("valid time");

    let created = service
        .upsert_rule(
            &pool,
            &auth,
            "purchase-inbound-api",
            rule_request(6),
            now,
            "mcg-rule-upsert-1",
        )
        .await
        .expect("rule should upsert");
    let replay = service
        .upsert_rule(
            &pool,
            &auth,
            "purchase-inbound-api",
            rule_request(6),
            now,
            "mcg-rule-upsert-1",
        )
        .await
        .expect("same rule idempotency key should replay");
    assert_eq!(created.value.id, replay.value.id);
    assert!(replay.replayed);
    let upserted_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM document_number_rules WHERE owner_id = $1 AND rule_code = 'purchase-inbound-api'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("upserted rule count should query");
    let upsert_audit_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'upsert_document_number_rule' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("upsert rule audit count should query");
    assert_eq!(upserted_rows, 1);
    assert_eq!(upsert_audit_rows, 1);

    let disabled = service
        .set_rule_enabled(
            &pool,
            &auth,
            "purchase-inbound-api",
            SetDocumentNumberRuleEnabledRequest { enabled: false },
            now,
            "mcg-rule-disable-1",
        )
        .await
        .expect("rule should disable");
    assert!(!disabled.value.enabled);
    service
        .set_rule_enabled(
            &pool,
            &auth,
            "purchase-inbound-api",
            SetDocumentNumberRuleEnabledRequest { enabled: true },
            now,
            "mcg-rule-enable-1",
        )
        .await
        .expect("rule should enable");

    let rules = service
        .list_rules(&pool, &auth, Some(DOCUMENT_TYPE_PURCHASE_INBOUND))
        .await
        .expect("rules should list");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].rule_code, "purchase-inbound-api");

    let other_owner_rules = service
        .list_rules(
            &pool,
            &ctx(other_owner_id),
            Some(DOCUMENT_TYPE_PURCHASE_INBOUND),
        )
        .await
        .expect("other owner rules should list");
    assert!(other_owner_rules.is_empty());

    let generated = generate_and_commit(&pool, &service, &auth, request("mcg-api-generate-1"), now)
        .await
        .expect("number should generate with public-managed rule");
    assert_eq!(
        generated.value.generated_no,
        "HZ005-purchase_inbound-20260702-000001"
    );

    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
          FROM audit_event
         WHERE owner_id = $1 AND module = 'M-CG'
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should query");
    assert_eq!(audit_count, 4);
}

#[sqlx::test(migrations = "../../migrations")]
async fn no_gap_generation_is_sequential_under_concurrency(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "HZ001").await;
    seed_daily_rule(&pool, owner_id, DOCUMENT_TYPE_PURCHASE_INBOUND, 5).await;
    let service = PgDocumentNumberingService::new();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 10, 0, 0)
        .single()
        .expect("valid time");

    let first_ctx = ctx(owner_id);
    let second_ctx = ctx(owner_id);
    let (first, second) = tokio::join!(
        generate_and_commit(&pool, &service, &first_ctx, request("mcg-race-1"), now),
        generate_and_commit(&pool, &service, &second_ctx, request("mcg-race-2"), now)
    );
    let mut generated = vec![
        first
            .expect("first no-gap number should generate")
            .value
            .generated_no,
        second
            .expect("second no-gap number should generate")
            .value
            .generated_no,
    ];
    generated.sort();

    assert_eq!(
        generated,
        vec![
            "HZ001-purchase_inbound-20260701-00001",
            "HZ001-purchase_inbound-20260701-00002",
        ]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_idempotency_key_replays_first_generated_number(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "HZ002").await;
    seed_daily_rule(&pool, owner_id, DOCUMENT_TYPE_PURCHASE_INBOUND, 4).await;
    let service = PgDocumentNumberingService::new();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 11, 0, 0)
        .single()
        .expect("valid time");
    let req = request("mcg-idem-1");

    let first = generate_and_commit(&pool, &service, &ctx(owner_id), req.clone(), now)
        .await
        .expect("first number should generate");
    let replay = generate_and_commit(&pool, &service, &ctx(owner_id), req, now)
        .await
        .expect("same idempotency key should replay");
    let allocation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM document_number_allocations WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("allocation count should query");

    assert_eq!(
        first.value.generated_no,
        "HZ002-purchase_inbound-20260701-0001"
    );
    assert_eq!(replay.value.generated_no, first.value.generated_no);
    assert!(replay.replayed);
    assert_eq!(allocation_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn transaction_rollback_does_not_advance_no_gap_counter(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "HZ003").await;
    seed_daily_rule(&pool, owner_id, DOCUMENT_TYPE_PURCHASE_INBOUND, 3).await;
    let service = PgDocumentNumberingService::new();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 12, 0, 0)
        .single()
        .expect("valid time");

    let mut tx = pool.begin().await.expect("transaction should begin");
    let generated_in_tx = service
        .generate_in_tx(&mut tx, &ctx(owner_id), request("mcg-rollback-1"), now)
        .await
        .expect("number should generate inside caller transaction");
    assert_eq!(
        generated_in_tx.value.generated_no,
        "HZ003-purchase_inbound-20260701-001"
    );
    tx.rollback().await.expect("rollback should succeed");

    let committed = generate_and_commit(
        &pool,
        &service,
        &ctx(owner_id),
        request("mcg-after-rollback"),
        now,
    )
    .await
    .expect("first committed number should reuse rolled-back value");

    assert_eq!(
        committed.value.generated_no,
        "HZ003-purchase_inbound-20260701-001"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_document_number_rule_returns_rule_not_found(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "HZ004").await;
    let service = PgDocumentNumberingService::new();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 13, 0, 0)
        .single()
        .expect("valid time");

    let error = generate_and_commit(
        &pool,
        &service,
        &ctx(owner_id),
        request("mcg-missing-rule"),
        now,
    )
    .await
    .expect_err("valid document_type without rule should fail closed");

    assert_eq!(error, DocumentNumberingError::RuleNotFound);
}

#[sqlx::test(migrations = "../../migrations")]
async fn allocation_list_route_filters_by_owner_document_type_date_and_limit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "HZ005").await;
    seed_owner(&pool, other_owner_id, "HZ006").await;
    seed_daily_rule(&pool, owner_id, DOCUMENT_TYPE_PURCHASE_INBOUND, 4).await;
    seed_daily_rule(&pool, owner_id, DOCUMENT_TYPE_SALES_RETURN, 4).await;
    seed_daily_rule(&pool, other_owner_id, DOCUMENT_TYPE_PURCHASE_INBOUND, 4).await;
    let service = PgDocumentNumberingService::new();
    let july_first = Utc
        .with_ymd_and_hms(2026, 7, 1, 9, 0, 0)
        .single()
        .expect("valid time");
    let july_second_purchase = Utc
        .with_ymd_and_hms(2026, 7, 2, 9, 0, 0)
        .single()
        .expect("valid time");
    let july_second_sales_return = Utc
        .with_ymd_and_hms(2026, 7, 2, 10, 0, 0)
        .single()
        .expect("valid time");
    let other_owner_later = Utc
        .with_ymd_and_hms(2026, 7, 2, 11, 0, 0)
        .single()
        .expect("valid time");

    generate_and_commit(
        &pool,
        &service,
        &ctx(owner_id),
        request_for(DOCUMENT_TYPE_PURCHASE_INBOUND, "mcg-list-old"),
        july_first,
    )
    .await
    .expect("old owner number should generate");
    generate_and_commit(
        &pool,
        &service,
        &ctx(owner_id),
        request_for(DOCUMENT_TYPE_PURCHASE_INBOUND, "mcg-list-match"),
        july_second_purchase,
    )
    .await
    .expect("matching owner number should generate");
    generate_and_commit(
        &pool,
        &service,
        &ctx(owner_id),
        request_for(DOCUMENT_TYPE_SALES_RETURN, "mcg-list-other-type"),
        july_second_sales_return,
    )
    .await
    .expect("other document type should generate");
    generate_and_commit(
        &pool,
        &service,
        &ctx(other_owner_id),
        request_for(DOCUMENT_TYPE_PURCHASE_INBOUND, "mcg-list-other-owner"),
        other_owner_later,
    )
    .await
    .expect("other owner number should generate");

    let token = bearer_token(owner_id, vec!["mcg.document_numbering.read"]);
    let app = document_numbering_router(DocumentNumberingAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/code-generator/document-number-allocations?document_type=purchase_inbound&from=2026-07-02T00:00:00Z&to=2026-07-02T23:59:59Z&limit=1")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let payload: DocumentNumberAllocationListResponse =
        serde_json::from_slice(&body).expect("response should be allocation list");

    assert_eq!(payload.page.count, 1);
    assert_eq!(payload.data.len(), 1);
    assert_eq!(payload.data[0].owner_id, owner_id);
    assert_eq!(
        payload.data[0].document_type,
        DOCUMENT_TYPE_PURCHASE_INBOUND
    );
    assert_eq!(
        payload.data[0].generated_no,
        "HZ005-purchase_inbound-20260702-0001"
    );
}

#[tokio::test]
async fn allocation_list_route_requires_auth_context() {
    let pool = PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during auth rejection test");
    let app = document_numbering_router(DocumentNumberingAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/code-generator/document-number-allocations")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should reject unauthenticated request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn allocation_list_route_requires_document_numbering_permission() {
    let pool = PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during permission rejection test");
    let token = bearer_token(Uuid::new_v4(), vec![]);
    let app = document_numbering_router(DocumentNumberingAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/code-generator/document-number-allocations")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should reject missing M-CG permission");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn allocation_list_route_rejects_invalid_date_query() {
    let pool = PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during query rejection test");
    let token = bearer_token(Uuid::new_v4(), vec!["mcg.document_numbering.read"]);
    let app = document_numbering_router(DocumentNumberingAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/code-generator/document-number-allocations?from=not-a-date")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should reject malformed query");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
