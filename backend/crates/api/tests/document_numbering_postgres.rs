use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    document_numbering::{
        DocumentNumberAllocation, DocumentNumberingError, GenerateDocumentNumberRequest,
        IdempotentMutation, PgDocumentNumberingService,
    },
};
use wms_domain::DOCUMENT_TYPE_PURCHASE_INBOUND;

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
    sqlx::query(
        r#"
        INSERT INTO document_number_rules (
            id, owner_id, document_type, rule_code, rule_name, template,
            reset_policy, sequence_width, enabled, effective_from, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, 'purchase-inbound-daily', '采购入库日流水',
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
    .execute(pool)
    .await
    .expect("document number rule seed should insert");
    rule_id
}

fn request(idempotency_key: &str) -> GenerateDocumentNumberRequest {
    GenerateDocumentNumberRequest {
        document_type: DOCUMENT_TYPE_PURCHASE_INBOUND.to_string(),
        idempotency_key: idempotency_key.to_string(),
        source_module: "M2".to_string(),
        source_document_id: Some(Uuid::new_v4()),
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
