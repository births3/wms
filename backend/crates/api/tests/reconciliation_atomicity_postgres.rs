use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    reconciliation::{
        PgReconciliationRepository, ReconciliationDisposition, ReconciliationError,
        ReconciliationInventoryAllocation,
    },
};

struct ResolutionFixture {
    actor: AuthContext,
    item_id: Uuid,
    batch_id: Uuid,
}

async fn seed_resolution_fixture(pool: &PgPool, difference_qty: i64) -> ResolutionFixture {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let item_id = Uuid::new_v4();
    let suffix = owner_id.simple().to_string();
    let qty_on_hand = difference_qty.max(0) + 10;
    let erp_qty = qty_on_hand - difference_qty;
    let difference_type = if difference_qty > 0 {
        "wms_more"
    } else {
        "erp_more"
    };
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name)
         VALUES ($1, $2, 'RC 原子性测试货主')",
    )
    .bind(owner_id)
    .bind(format!("RC-ATOMIC-{}", &suffix[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status)
         VALUES ($1, $2, 'RC 原子性测试用户', 'test-hash', 'active')",
    )
    .bind(user_id)
    .bind(format!("rc-atomic-{}", &suffix[..8]))
    .execute(pool)
    .await
    .expect("seed user");
    sqlx::query(
        "INSERT INTO products
         (id, owner_id, product_code, product_name, specification, storage_condition,
          special_drug_category, status)
         VALUES ($1,$2,'P-ATOMIC','原子性测试商品','测试规格','normal','normal','active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed product");
    sqlx::query(
        "INSERT INTO warehouses
         (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status)
         VALUES ($1,$2,$3,'RC 原子性仓库','main','active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("RC-AWH-{}", &suffix[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones
         (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
          quality_color, status)
         VALUES ($1,$2,$3,$4,'RC 原子性库区','normal','qualified_green','active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("RC-AZ-{}", &suffix[..8]))
    .execute(pool)
    .await
    .expect("seed zone");
    sqlx::query(
        "INSERT INTO warehouse_locations
         (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
          max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status)
         VALUES ($1,$2,$3,$4,$5,1,1,1,100000,0,10,'storage','available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(format!("RC-AL-{}", &suffix[..8]))
    .execute(pool)
    .await
    .expect("seed location");
    sqlx::query(
        "INSERT INTO inventory_batches
         (id, owner_id, product_code, batch_no, production_date, expiry_date,
          qty_on_hand, qty_locked, quality_status, location_id, location_code)
         VALUES ($1,$2,'P-ATOMIC','B-ATOMIC',$3,$4,$5,0,'qualified',$6,$7)",
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"))
    .bind(qty_on_hand)
    .bind(location_id)
    .bind(format!("RC-AL-{}", &suffix[..8]))
    .execute(pool)
    .await
    .expect("seed inventory batch");
    sqlx::query(
        "INSERT INTO reconciliation_runs
         (id, owner_id, window_key, request_hash, snapshot_at, matched_count,
          wms_more_count, erp_more_count, created_by, created_at)
         VALUES ($1,$2,$3,'atomicity-fixture',$4,0,$5,$6,$7,$4)",
    )
    .bind(run_id)
    .bind(owner_id)
    .bind(format!("atomicity-{item_id}"))
    .bind(now)
    .bind(i32::from(difference_qty > 0))
    .bind(i32::from(difference_qty < 0))
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed reconciliation run");
    sqlx::query(
        "INSERT INTO reconciliation_items
         (id, owner_id, run_id, product_code, batch_no, wms_qty, erp_qty,
          difference_qty, difference_type, resolution_status, created_at, updated_at)
         VALUES ($1,$2,$3,'P-ATOMIC','B-ATOMIC',$4,$5,$6,$7,'open',$8,$8)",
    )
    .bind(item_id)
    .bind(owner_id)
    .bind(run_id)
    .bind(qty_on_hand)
    .bind(erp_qty)
    .bind(difference_qty)
    .bind(difference_type)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed reconciliation item");

    ResolutionFixture {
        actor: AuthContext {
            user_id,
            owner_id,
            actor_name: "rc-atomicity-test".into(),
            permissions: vec!["rc.reconciliation.resolve".into()],
            jti: Uuid::new_v4().to_string(),
            warehouse_scope: None,
        },
        item_id,
        batch_id,
    }
}

async fn assert_adjustment_side_effects_rolled_back(
    pool: &PgPool,
    fixture: &ResolutionFixture,
    resolve_idempotency_key: &str,
) {
    let derived_key = format!("rc-msa:{}", fixture.item_id);
    let external_ref = format!("reconciliation:{}", fixture.item_id);
    let (
        orders,
        allocations,
        counter_value,
        stock_idempotency,
        resolve_idempotency,
        adjustment_audits,
    ): (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM stock_adjustment_orders
              WHERE owner_id=$1 AND external_ref=$2),
            (SELECT COUNT(*) FROM document_number_allocations
              WHERE owner_id=$1 AND source_module='M-SA'),
            (SELECT COALESCE(MAX(counter.current_value), 0)
               FROM document_number_counters counter
               JOIN document_number_rules rule ON rule.id=counter.rule_id
              WHERE rule.document_type IN ('stock_loss','stock_surplus')),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id=$1 AND idempotency_key=$3),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id=$1 AND idempotency_key=$4),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id=$1
                AND action IN ('create_stock_loss_order','create_stock_surplus_order'))",
    )
    .bind(fixture.actor.owner_id)
    .bind(external_ref)
    .bind(derived_key)
    .bind(resolve_idempotency_key)
    .fetch_one(pool)
    .await
    .expect("read rolled-back adjustment side effects");
    assert_eq!(
        (
            orders,
            allocations,
            counter_value,
            stock_idempotency,
            resolve_idempotency,
            adjustment_audits,
        ),
        (0, 0, 0, 0, 0, 0)
    );

    let (status, disposition, link_count): (String, Option<String>, i64) = sqlx::query_as(
        "SELECT item.resolution_status, item.disposition,
                (SELECT COUNT(*) FROM reconciliation_item_adjustments link
                  WHERE link.item_id = item.id)
           FROM reconciliation_items item
          WHERE item.owner_id=$1 AND item.id=$2",
    )
    .bind(fixture.actor.owner_id)
    .bind(fixture.item_id)
    .fetch_one(pool)
    .await
    .expect("read unresolved reconciliation item");
    assert_eq!(status, "open");
    assert_eq!(disposition, None);
    assert_eq!(link_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn erp_truth_rolls_back_loss_order_when_reconciliation_update_fails(pool: PgPool) {
    let fixture = seed_resolution_fixture(&pool, 3).await;
    sqlx::query(
        "CREATE FUNCTION fail_rc_resolution_update() RETURNS TRIGGER AS $$
         BEGIN
             IF NEW.disposition = 'erp_truth' THEN
                 RAISE EXCEPTION 'forced reconciliation update failure';
             END IF;
             RETURN NEW;
         END;
         $$ LANGUAGE plpgsql",
    )
    .execute(&pool)
    .await
    .expect("create update failure function");
    sqlx::query(
        "CREATE TRIGGER fail_rc_resolution_update_trigger
         BEFORE UPDATE ON reconciliation_items
         FOR EACH ROW EXECUTE FUNCTION fail_rc_resolution_update()",
    )
    .execute(&pool)
    .await
    .expect("create update failure trigger");

    let error = PgReconciliationRepository::new(pool.clone())
        .resolve(
            &fixture.actor,
            fixture.item_id,
            ReconciliationDisposition::ErpTruth,
            vec![ReconciliationInventoryAllocation {
                inventory_batch_id: fixture.batch_id,
                quantity: 3,
            }],
            Utc::now(),
            "rc-resolve-update-failure",
        )
        .await
        .expect_err("forced reconciliation update must fail");
    assert!(matches!(error, ReconciliationError::Database(_)));
    assert_adjustment_side_effects_rolled_back(&pool, &fixture, "rc-resolve-update-failure").await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn erp_truth_rolls_back_surplus_order_when_resolution_audit_fails(pool: PgPool) {
    let fixture = seed_resolution_fixture(&pool, -3).await;
    sqlx::query(
        "CREATE FUNCTION fail_rc_resolution_audit() RETURNS TRIGGER AS $$
         BEGIN
             IF NEW.action = 'resolve_reconciliation_item' THEN
                 RAISE EXCEPTION 'forced reconciliation audit failure';
             END IF;
             RETURN NEW;
         END;
         $$ LANGUAGE plpgsql",
    )
    .execute(&pool)
    .await
    .expect("create audit failure function");
    sqlx::query(
        "CREATE TRIGGER fail_rc_resolution_audit_trigger
         BEFORE INSERT ON audit_event
         FOR EACH ROW EXECUTE FUNCTION fail_rc_resolution_audit()",
    )
    .execute(&pool)
    .await
    .expect("create audit failure trigger");

    let error = PgReconciliationRepository::new(pool.clone())
        .resolve(
            &fixture.actor,
            fixture.item_id,
            ReconciliationDisposition::ErpTruth,
            vec![ReconciliationInventoryAllocation {
                inventory_batch_id: fixture.batch_id,
                quantity: 3,
            }],
            Utc::now(),
            "rc-resolve-audit-failure",
        )
        .await
        .expect_err("forced reconciliation audit must fail");
    assert!(matches!(error, ReconciliationError::Audit(_)));
    assert_adjustment_side_effects_rolled_back(&pool, &fixture, "rc-resolve-audit-failure").await;
}
