//! M4 采购退货出库（purchase_return_orders）仓储层 Postgres 集成测试。

use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wave4_repository::{PgWave4Repository, Wave4RepositoryError},
};
use wms_domain::{CreatePurchaseReturnRequest, PurchaseReturnOrder, RejectPurchaseReturnRequest};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m4-purchase-return-test".to_string(),
        permissions: vec!["m4.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn create_request(return_no: &str) -> CreatePurchaseReturnRequest {
    CreatePurchaseReturnRequest {
        return_no: return_no.to_string(),
        source_purchase_order_no: "ASN-M2-PC-0001".to_string(),
        supplier_id: Some(Uuid::new_v4()),
        supplier_name: "华东医药供应商".to_string(),
        reason: "供应商召回".to_string(),
        warehouse_id: Uuid::new_v4(),
        product_code: "P-M4-001".to_string(),
        qty: 6,
    }
}

async fn create_return(
    repo: &PgWave4Repository,
    ctx: &AuthContext,
    return_no: &str,
    now: chrono::DateTime<Utc>,
    idempotency_key: &str,
) -> PurchaseReturnOrder {
    repo.create_purchase_return(ctx, create_request(return_no), now, idempotency_key, None)
        .await
        .expect("purchase return should be created")
        .value
}

async fn audit_count(pool: &PgPool, owner_id: Uuid, action: &str, resource_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM audit_event
         WHERE owner_id = $1
           AND module = 'M4'
           AND action = $2
           AND resource_type = 'purchase_return_order'
           AND resource_id = $3::text
        "#,
    )
    .bind(owner_id)
    .bind(action)
    .bind(resource_id)
    .fetch_one(pool)
    .await
    .expect("audit count")
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_purchase_return_full_lifecycle_with_audit_evidence(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 9, 0, 0)
        .single()
        .expect("valid time");

    let created = create_return(&repo, &ctx, "PRTN-M4-LC-0001", now, "prtn-create-1").await;
    assert_eq!(created.status, "pending_approval");
    assert_eq!(created.document_type, "purchase_return_outbound");
    assert_eq!(created.approval_source, "purchase_return_approval");
    assert_eq!(created.qty, 6);
    assert!(created.shipped_at.is_none());

    let approved = repo
        .approve_purchase_return(&ctx, created.id, now, "prtn-approve-1", None)
        .await
        .expect("approve should succeed");
    assert!(!approved.replayed);
    assert_eq!(approved.value.status, "approved");

    let picking = repo
        .pick_purchase_return(&ctx, created.id, now, "prtn-pick-1", None)
        .await
        .expect("pick should succeed");
    assert_eq!(picking.value.status, "picking");

    let reviewed = repo
        .review_purchase_return(&ctx, created.id, now, "prtn-review-1", None)
        .await
        .expect("review should succeed");
    assert_eq!(reviewed.value.status, "reviewed");

    let shipped = repo
        .ship_purchase_return(&ctx, created.id, now, "prtn-ship-1", None)
        .await
        .expect("ship should succeed");
    assert_eq!(shipped.value.status, "shipped");
    // 交接时间与操作人证据。
    assert_eq!(shipped.value.shipped_at, Some(now));
    assert_eq!(shipped.value.shipped_by, Some(ctx.user_id));
    assert_eq!(
        shipped.value.shipped_by_name.as_deref(),
        Some("m4-purchase-return-test")
    );

    // 全链路审计证据：每个动作各一条。
    for action in [
        "create_purchase_return",
        "approve_purchase_return",
        "pick_purchase_return",
        "review_purchase_return",
        "ship_purchase_return",
    ] {
        assert_eq!(
            audit_count(&pool, owner_id, action, created.id).await,
            1,
            "audit evidence missing for {action}"
        );
    }

    // 幂等请求证据：五个幂等键各落一行。
    let idempotency_rows: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM idempotency_request
         WHERE owner_id = $1
           AND idempotency_key IN
               ('prtn-create-1', 'prtn-approve-1', 'prtn-pick-1', 'prtn-review-1', 'prtn-ship-1')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("idempotency evidence");
    assert_eq!(idempotency_rows, 5);

    // 列表与详情可读。
    let listed = repo
        .list_purchase_returns(&ctx, Some("shipped"), Some("PRTN-M4-LC"), None)
        .await
        .expect("list should succeed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    let detail = repo
        .get_purchase_return(&ctx, created.id)
        .await
        .expect("detail should load");
    assert_eq!(detail.status, "shipped");
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_purchase_return_create_rejects_duplicate_return_no(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 10, 0, 0)
        .single()
        .expect("valid time");

    create_return(&repo, &ctx, "PRTN-M4-DUP-0001", now, "prtn-dup-1").await;

    // 相同 return_no + 新幂等键 → 唯一约束冲突（映射 409）。
    let conflict = repo
        .create_purchase_return(
            &ctx,
            create_request("PRTN-M4-DUP-0001"),
            now,
            "prtn-dup-2",
            None,
        )
        .await
        .expect_err("duplicate return_no must conflict");
    assert!(matches!(conflict, Wave4RepositoryError::DuplicateCode));

    // 数量非法直接 422。
    let mut invalid = create_request("PRTN-M4-DUP-0002");
    invalid.qty = 0;
    let rejected = repo
        .create_purchase_return(&ctx, invalid, now, "prtn-dup-3", None)
        .await
        .expect_err("non-positive qty must be rejected");
    assert!(matches!(rejected, Wave4RepositoryError::InvalidQuantity));
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_purchase_return_create_rejects_blank_required_fields(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 10, 30, 0)
        .single()
        .expect("valid time");

    type BlankCase = (&'static str, fn(&mut CreatePurchaseReturnRequest));
    let blank_cases: [BlankCase; 5] = [
        ("return_no", |request: &mut CreatePurchaseReturnRequest| {
            request.return_no = "  ".to_string()
        }),
        (
            "source_purchase_order_no",
            |request: &mut CreatePurchaseReturnRequest| {
                request.source_purchase_order_no = "  ".to_string();
            },
        ),
        (
            "supplier_name",
            |request: &mut CreatePurchaseReturnRequest| request.supplier_name = "  ".to_string(),
        ),
        ("reason", |request: &mut CreatePurchaseReturnRequest| {
            request.reason = "  ".to_string()
        }),
        (
            "product_code",
            |request: &mut CreatePurchaseReturnRequest| request.product_code = "  ".to_string(),
        ),
    ];
    for (field, mutate) in blank_cases {
        let mut request = create_request(&format!("PRTN-M4-BLANK-{field}"));
        mutate(&mut request);
        let rejected = repo
            .create_purchase_return(&ctx, request, now, &format!("prtn-blank-{field}"), None)
            .await
            .expect_err("blank required field must be rejected");
        assert_eq!(rejected, Wave4RepositoryError::MissingRequiredField(field));
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_purchase_return_idempotent_replay_and_conflict(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 11, 0, 0)
        .single()
        .expect("valid time");

    // 创建幂等重放：同键同请求返回同一单据且不重复落库。
    let request = create_request("PRTN-M4-IDEM-0001");
    let first = repo
        .create_purchase_return(&ctx, request.clone(), now, "prtn-idem-1", None)
        .await
        .expect("create should succeed");
    assert!(!first.replayed);
    let replay = repo
        .create_purchase_return(&ctx, request, now, "prtn-idem-1", None)
        .await
        .expect("same key + same payload should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value.id, first.value.id);

    let evidence: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM purchase_return_orders WHERE owner_id = $1),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'create_purchase_return')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("replay evidence");
    assert_eq!(evidence, (1, 1));

    // 同键不同请求体 → 幂等冲突。
    let conflict = repo
        .create_purchase_return(
            &ctx,
            create_request("PRTN-M4-IDEM-0002"),
            now,
            "prtn-idem-1",
            None,
        )
        .await
        .expect_err("same key + different payload must conflict");
    assert!(matches!(
        conflict,
        Wave4RepositoryError::IdempotencyConflict
    ));

    // 动作幂等重放。
    let approved = repo
        .approve_purchase_return(&ctx, first.value.id, now, "prtn-idem-approve", None)
        .await
        .expect("approve should succeed");
    assert!(!approved.replayed);
    let approve_replay = repo
        .approve_purchase_return(&ctx, first.value.id, now, "prtn-idem-approve", None)
        .await
        .expect("approve replay should succeed");
    assert!(approve_replay.replayed);
    assert_eq!(approve_replay.value.status, "approved");
    assert_eq!(
        audit_count(&pool, owner_id, "approve_purchase_return", first.value.id).await,
        1
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_purchase_return_reject_requires_reason_and_cancels(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 12, 0, 0)
        .single()
        .expect("valid time");
    let created = create_return(&repo, &ctx, "PRTN-M4-RJ-0001", now, "prtn-rj-create").await;

    // 空白驳回原因 → 422。
    let missing = repo
        .reject_purchase_return(
            &ctx,
            created.id,
            RejectPurchaseReturnRequest {
                reason: "   ".to_string(),
            },
            now,
            "prtn-rj-empty",
            None,
        )
        .await
        .expect_err("blank reject reason must be rejected");
    assert!(matches!(missing, Wave4RepositoryError::MissingRejectReason));

    let rejected = repo
        .reject_purchase_return(
            &ctx,
            created.id,
            RejectPurchaseReturnRequest {
                reason: "供应商取消退货".to_string(),
            },
            now,
            "prtn-rj-1",
            None,
        )
        .await
        .expect("reject should succeed");
    assert!(!rejected.replayed);
    assert_eq!(rejected.value.status, "cancelled");
    assert_eq!(
        rejected.value.reject_reason.as_deref(),
        Some("供应商取消退货")
    );
    assert_eq!(
        audit_count(&pool, owner_id, "reject_purchase_return", created.id).await,
        1
    );

    // 已取消后不允许再次审批或驳回。
    let approve_after = repo
        .approve_purchase_return(&ctx, created.id, now, "prtn-rj-2", None)
        .await
        .expect_err("cancelled return must not approve");
    assert!(matches!(
        approve_after,
        Wave4RepositoryError::InvalidStatus { ref actual, .. } if actual == "cancelled"
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_purchase_return_actions_reject_illegal_preconditions(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 13, 0, 0)
        .single()
        .expect("valid time");
    let created = create_return(&repo, &ctx, "PRTN-M4-ILL-0001", now, "prtn-ill-create").await;

    // pending_approval：拣货 / 复核 / 出库交接均为非法前置状态。
    let pick = repo
        .pick_purchase_return(&ctx, created.id, now, "prtn-ill-pick", None)
        .await
        .expect_err("pending_approval must not pick");
    assert!(matches!(
        pick,
        Wave4RepositoryError::InvalidStatus { ref expected, ref actual }
            if expected == "approved" && actual == "pending_approval"
    ));
    let review = repo
        .review_purchase_return(&ctx, created.id, now, "prtn-ill-review", None)
        .await
        .expect_err("pending_approval must not review");
    assert!(matches!(
        review,
        Wave4RepositoryError::InvalidStatus { ref expected, .. } if expected == "picking"
    ));
    let ship = repo
        .ship_purchase_return(&ctx, created.id, now, "prtn-ill-ship", None)
        .await
        .expect_err("pending_approval must not ship");
    assert!(matches!(
        ship,
        Wave4RepositoryError::InvalidStatus { ref expected, .. } if expected == "reviewed"
    ));

    // approved 后：重复审批 / 驳回为非法前置状态。
    repo.approve_purchase_return(&ctx, created.id, now, "prtn-ill-approve", None)
        .await
        .expect("approve should succeed");
    let approve_twice = repo
        .approve_purchase_return(&ctx, created.id, now, "prtn-ill-approve-2", None)
        .await
        .expect_err("approved return must not approve again");
    assert!(matches!(
        approve_twice,
        Wave4RepositoryError::InvalidStatus { ref actual, .. } if actual == "approved"
    ));
    let reject_after_approve = repo
        .reject_purchase_return(
            &ctx,
            created.id,
            RejectPurchaseReturnRequest {
                reason: "迟到的驳回".to_string(),
            },
            now,
            "prtn-ill-reject",
            None,
        )
        .await
        .expect_err("approved return must not reject");
    assert!(matches!(
        reject_after_approve,
        Wave4RepositoryError::InvalidStatus { ref actual, .. } if actual == "approved"
    ));

    // 非法前置状态被拒后未落审计与幂等记录（事务回滚）。
    let rollback_evidence: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action IN
                    ('pick_purchase_return', 'review_purchase_return', 'ship_purchase_return')),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id = $1 AND idempotency_key LIKE 'prtn-ill-%'
                AND idempotency_key <> 'prtn-ill-create'
                AND idempotency_key <> 'prtn-ill-approve')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("rollback evidence");
    assert_eq!(rollback_evidence, (0, 0));

    // 不存在的单据 → NotFound。
    let missing = repo
        .approve_purchase_return(&ctx, Uuid::new_v4(), now, "prtn-ill-missing", None)
        .await
        .expect_err("unknown return should not approve");
    assert!(matches!(missing, Wave4RepositoryError::NotFound));
}
