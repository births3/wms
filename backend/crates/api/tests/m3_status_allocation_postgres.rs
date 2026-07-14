use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    inventory::{
        STATUS_LOSS_DEDUCTED, STATUS_PENDING_DESTRUCTION, STATUS_QUALIFIED, STATUS_QUARANTINED,
        STATUS_UNQUALIFIED,
    },
    system_dictionary::PgSystemDictionaryRepository,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
    wave4_repository::{PgWave4Repository, Wave4RepositoryError},
};
use wms_domain::{
    ChangeInventoryStatusRequest, CreateOutboundOrderLineRequest, CreateOutboundOrderRequest,
    CreateOutboundWaveRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m3-status-allocation-test".to_string(),
        permissions: vec!["m3.write".to_string(), "m4.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_dictionary_item(pool: &PgPool, owner_id: Uuid, item_code: &str, item_name: &str) {
    sqlx::query(
        r#"
        INSERT INTO system_dictionary_items (
            id, dict_code, item_code, item_name, enabled, owner_id,
            params, source, created_at, updated_at
        )
        VALUES ($1, 'inventory_quality_status', $2, $3, TRUE, $4, '{}'::jsonb, 'owner', $5, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(item_code)
    .bind(item_name)
    .bind(owner_id)
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("seed owner inventory status dictionary item");
}

async fn seed_batch(pool: &PgPool, owner_id: Uuid, product_code: &str, batch_no: &str) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 10, 0, $7, $8, 'M3-STATUS-01', FALSE, $9, $9)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(product_code)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"))
    .bind(STATUS_QUALIFIED)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed inventory batch");
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_quality_status_dictionary_has_default_effective_items(pool: PgPool) {
    let repository = PgSystemDictionaryRepository::new(pool);
    let items = repository
        .list_effective_items(&ctx(Uuid::new_v4()), "inventory_quality_status", Utc::now())
        .await
        .expect("default inventory quality status dictionary should be readable");

    let codes = items
        .into_iter()
        .map(|item| item.item_code)
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![
            STATUS_LOSS_DEDUCTED.to_string(),
            STATUS_PENDING_DESTRUCTION.to_string(),
            STATUS_QUALIFIED.to_string(),
            STATUS_QUARANTINED.to_string(),
            STATUS_UNQUALIFIED.to_string(),
        ]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_status_change_rejects_unknown_dictionary_target(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = seed_batch(&pool, owner_id, "P-M3-STATUS-003", "B-M3-STATUS-003").await;
    let repository = PgWave3Repository::new(pool.clone());

    let error = repository
        .change_inventory_status_with_audit(
            &ctx(owner_id),
            ChangeInventoryStatusRequest {
                batch_id,
                target_status: "not-configured".to_string(),
                reason: "unknown target must be rejected".to_string(),
                approval_source: "M-QL".to_string(),
                approval_id: "QL-M3-STATUS-003".to_string(),
            },
            Utc::now(),
            "m3-status-unknown-target",
            None,
        )
        .await
        .expect_err("unknown target status must be rejected");

    assert_eq!(error, Wave3RepositoryError::InvalidQualityStatus);
    let evidence: (String, i64) = sqlx::query_as(
        "SELECT quality_status, (SELECT COUNT(*) FROM inventory_status_changes WHERE batch_id = $1) FROM inventory_batches WHERE id = $1",
    )
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("unknown target evidence should query");
    assert_eq!(evidence, (STATUS_QUALIFIED.to_string(), 0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_quality_status_dictionary_prefers_owner_override_without_cross_owner_reads(
    pool: PgPool,
) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    seed_dictionary_item(&pool, owner_a, STATUS_QUARANTINED, "货主 A 隔离").await;
    seed_dictionary_item(&pool, owner_a, "owner-a-only", "货主 A 专属状态").await;

    let dictionary = PgSystemDictionaryRepository::new(pool.clone());
    let owner_a_items = dictionary
        .list_effective_items(&ctx(owner_a), "inventory_quality_status", Utc::now())
        .await
        .expect("owner A dictionary should be readable");
    let owner_b_items = dictionary
        .list_effective_items(&ctx(owner_b), "inventory_quality_status", Utc::now())
        .await
        .expect("owner B dictionary should be readable");

    let owner_a_quarantine = owner_a_items
        .iter()
        .find(|item| item.item_code == STATUS_QUARANTINED)
        .expect("owner A quarantine override should be effective");
    assert_eq!(owner_a_quarantine.owner_id, Some(owner_a));
    assert!(owner_a_items
        .iter()
        .any(|item| item.item_code == "owner-a-only"));
    assert_eq!(
        owner_b_items
            .iter()
            .find(|item| item.item_code == STATUS_QUARANTINED)
            .expect("owner B should fall back to global quarantine")
            .owner_id,
        None
    );
    assert!(!owner_b_items
        .iter()
        .any(|item| item.item_code == "owner-a-only"));

    let batch_id = seed_batch(&pool, owner_b, "P-M3-STATUS-004", "B-M3-STATUS-004").await;
    let error = PgWave3Repository::new(pool)
        .change_inventory_status_with_audit(
            &ctx(owner_b),
            ChangeInventoryStatusRequest {
                batch_id,
                target_status: "owner-a-only".to_string(),
                reason: "cross-owner dictionary item must be rejected".to_string(),
                approval_source: "M-QL".to_string(),
                approval_id: "QL-M3-STATUS-004".to_string(),
            },
            Utc::now(),
            "m3-status-cross-owner-target",
            None,
        )
        .await
        .expect_err("owner B must not read owner A status item");
    assert_eq!(error, Wave3RepositoryError::InvalidQualityStatus);
}

#[sqlx::test(migrations = "../../migrations")]
async fn legal_inventory_status_change_is_audited_and_idempotent(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let context = ctx(owner_id);
    let batch_id = seed_batch(&pool, owner_id, "P-M3-STATUS-005", "B-M3-STATUS-005").await;
    let repository = PgWave3Repository::new(pool.clone());
    let request = ChangeInventoryStatusRequest {
        batch_id,
        target_status: STATUS_QUARANTINED.to_string(),
        reason: "质量异常隔离".to_string(),
        approval_source: "M-QL".to_string(),
        approval_id: "QL-M3-STATUS-005".to_string(),
    };
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &context,
        "change_status",
        "M3",
        "inventory_batch",
        batch_id.to_string(),
        None,
    );

    let first = repository
        .change_inventory_status_with_audit(
            &context,
            request.clone(),
            now,
            "m3-status-idempotent",
            Some(audit.clone()),
        )
        .await
        .expect("legal status change should succeed");
    let replay = repository
        .change_inventory_status_with_audit(
            &context,
            request,
            now,
            "m3-status-idempotent",
            Some(audit),
        )
        .await
        .expect("same status change should replay");

    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);
    assert_eq!(first.value.quality_status, replay.value.quality_status);
    let writes: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM inventory_status_changes WHERE owner_id = $1 AND batch_id = $2),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'change_status' AND resource_id = $3)
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .bind(batch_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("status and audit evidence should query");
    assert_eq!(writes, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_status_change_rejects_blank_reason(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = seed_batch(&pool, owner_id, "P-M3-STATUS-001", "B-M3-STATUS-001").await;
    let repository = PgWave3Repository::new(pool);

    let error = repository
        .change_inventory_status_with_audit(
            &ctx(owner_id),
            ChangeInventoryStatusRequest {
                batch_id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "  ".to_string(),
                approval_source: "M-QL".to_string(),
                approval_id: "QL-M3-STATUS-001".to_string(),
            },
            Utc::now(),
            "m3-status-blank-reason",
            None,
        )
        .await
        .expect_err("blank status reason must be rejected");

    assert_eq!(error, Wave3RepositoryError::InvalidReason);
}

#[sqlx::test(migrations = "../../migrations")]
async fn quarantined_inventory_is_not_allocated_to_outbound_wave(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let context = ctx(owner_id);
    let product_code = "P-M3-STATUS-002";
    let batch_no = "B-M3-STATUS-002";
    let batch_id = seed_batch(&pool, owner_id, product_code, batch_no).await;
    let inventory = PgWave3Repository::new(pool.clone());
    inventory
        .change_inventory_status_with_audit(
            &context,
            ChangeInventoryStatusRequest {
                batch_id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "质量异常隔离".to_string(),
                approval_source: "M-QL".to_string(),
                approval_id: "QL-M3-STATUS-002".to_string(),
            },
            Utc::now(),
            "m3-status-quarantine-002",
            None,
        )
        .await
        .expect("quality status should change to quarantined");

    let outbound = PgWave4Repository::new(pool.clone());
    let order = outbound
        .create_outbound_order(
            &context,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "M3-STATUS-OUT-002".to_string(),
                erp_order_no: None,
                customer_id: Uuid::new_v4(),
                warehouse_id: Uuid::new_v4(),
                required_ship_at: None,
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: product_code.to_string(),
                    batch_no: batch_no.to_string(),
                    planned_qty: 1,
                }],
            },
            Utc::now(),
            "m3-status-outbound-order-002",
            None,
        )
        .await
        .expect("outbound order should be created")
        .value;

    let error = outbound
        .create_outbound_wave(
            &context,
            CreateOutboundWaveRequest {
                wave_no: "M3-STATUS-WAVE-002".to_string(),
                order_ids: vec![order.id],
            },
            Utc::now(),
            "m3-status-outbound-wave-002",
            None,
        )
        .await
        .expect_err("quarantined inventory must not be allocated");
    assert_eq!(error, Wave4RepositoryError::InvalidQuantity);

    let evidence: (String, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT quality_status FROM inventory_batches WHERE id = $1),
            (SELECT qty_locked FROM inventory_batches WHERE id = $1),
            (SELECT COUNT(*) FROM inventory_allocations WHERE owner_id = $2 AND outbound_order_id = $3)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("allocation evidence should query");
    assert_eq!(
        evidence,
        (STATUS_QUARANTINED.to_string(), 0, 0),
        "a failed wave must not lock or allocate isolated inventory"
    );
}
