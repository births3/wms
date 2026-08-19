use super::*;

pub(crate) async fn advance_from_stock_adjustment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    adjustment_order_id: Uuid,
    adjustment_status: &str,
    now: DateTime<Utc>,
) -> Result<(), ReconciliationError> {
    let item_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT item_id
           FROM reconciliation_item_adjustments
          WHERE owner_id = $1 AND adjustment_order_id = $2",
    )
    .bind(ctx.owner_id)
    .bind(adjustment_order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let Some(item_id) = item_id else {
        return Ok(());
    };
    let next_status = if matches!(
        adjustment_status,
        "rejected" | "cancelled" | "exception_suspended"
    ) {
        Some("exception")
    } else if adjustment_status == "completed" {
        let unfinished: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
               FROM reconciliation_item_adjustments link
               JOIN stock_adjustment_orders adjustment
                 ON adjustment.owner_id = link.owner_id
                AND adjustment.id = link.adjustment_order_id
              WHERE link.owner_id = $1
                AND link.item_id = $2
                AND adjustment.status <> 'completed'",
        )
        .bind(ctx.owner_id)
        .bind(item_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        (unfinished == 0).then_some("resolved")
    } else {
        None
    };
    let Some(next_status) = next_status else {
        return Ok(());
    };
    let updated = sqlx::query(
        "UPDATE reconciliation_items
            SET resolution_status = $3,
                resolved_at = CASE WHEN $3 = 'resolved' THEN $4 ELSE NULL END,
                updated_at = $4
          WHERE owner_id = $1 AND id = $2
            AND resolution_status IN ('adjustment_pending', 'exception')",
    )
    .bind(ctx.owner_id)
    .bind(item_id)
    .bind(next_status)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    if updated.rows_affected() == 0 {
        return Ok(());
    }
    let released_batches = if next_status == "resolved" {
        release_item_locks(tx, ctx.owner_id, item_id, now).await?
    } else {
        0
    };
    append_reconciliation_audit(
        tx,
        ctx,
        "advance_reconciliation_adjustment",
        item_id.to_string(),
        json!({
            "adjustment_order_id": adjustment_order_id,
            "adjustment_status": adjustment_status,
            "status": next_status,
            "released_batches": released_batches,
        }),
        now,
    )
    .await
}

pub(crate) async fn advance_from_h8_receipt_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    h8_status: &str,
    now: DateTime<Utc>,
    audit_seed: Option<&AuditWriteRequest>,
) -> Result<(), ReconciliationError> {
    let Some(outbox_id) = idempotency_key
        .strip_prefix("out:reconciliation_erp_feedback_outbox:")
        .and_then(|value| value.parse::<Uuid>().ok())
    else {
        return Ok(());
    };
    let item_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT recon_doc_no::UUID
           FROM reconciliation_erp_feedback_outbox
          WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(outbox_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let Some(item_id) = item_id else {
        return Ok(());
    };
    let next_status = match h8_status {
        "acked" => "resolved",
        "dead" => "exception",
        _ => return Ok(()),
    };
    let updated = sqlx::query(
        "UPDATE reconciliation_items
            SET resolution_status = $3,
                resolved_at = CASE WHEN $3 = 'resolved' THEN $4 ELSE NULL END,
                updated_at = $4
          WHERE owner_id = $1 AND id = $2
            AND resolution_status IN ('erp_feedback_pending', 'exception')",
    )
    .bind(owner_id)
    .bind(item_id)
    .bind(next_status)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    if updated.rows_affected() == 0 {
        return Ok(());
    }
    if next_status == "resolved" {
        release_item_locks(tx, owner_id, item_id, now).await?;
    }
    let seed = audit_seed.ok_or_else(|| {
        ReconciliationError::Audit("H8 receipt audit context is required".to_string())
    })?;
    let mut audit = seed.clone();
    audit.occurred_at = now;
    audit.owner_id = owner_id;
    audit.action = "advance_reconciliation_h8_receipt".to_string();
    audit.module = "M-RC".to_string();
    audit.resource_type = "reconciliation_item".to_string();
    audit.resource_id = item_id.to_string();
    audit.diff = Some(AuditDiff::compute(
        json!({}),
        json!({
            "h8_status": h8_status,
            "status": next_status,
        }),
    ));
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| ReconciliationError::Audit(format!("{error:?}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../../migrations")]
    async fn repeated_adjustment_completion_does_not_duplicate_progress_audit(pool: PgPool) {
        let owner_id = Uuid::parse_str("41000000-0000-0000-0000-000000000001").unwrap();
        let user_id = Uuid::parse_str("41000000-0000-0000-0000-000000000002").unwrap();
        let item_id = Uuid::parse_str("41000000-0000-0000-0000-000000000003").unwrap();
        let adjustment_id = Uuid::parse_str("41000000-0000-0000-0000-000000000004").unwrap();
        sqlx::raw_sql(
            "INSERT INTO auth_owners (id, owner_code, owner_name)
             VALUES ('41000000-0000-0000-0000-000000000001','RC-PROGRESS','RC progression');
             INSERT INTO auth_users (id, username, display_name, password_hash)
             VALUES ('41000000-0000-0000-0000-000000000002','rc-progress','RC progression','x');
             INSERT INTO warehouses
               (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status)
             VALUES
               ('41000000-0000-0000-0000-000000000005',
                '41000000-0000-0000-0000-000000000001',
                'RC-WH','RC warehouse','main','active');
             INSERT INTO warehouse_zones
               (id, owner_id, warehouse_id, zone_code, zone_name,
                temperature_zone, quality_color, status)
             VALUES
               ('41000000-0000-0000-0000-000000000006',
                '41000000-0000-0000-0000-000000000001',
                '41000000-0000-0000-0000-000000000005',
                'RC-Z','RC zone','normal_10_30','qualified_green','active');
             INSERT INTO warehouse_locations
               (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no,
                layer_no, max_volume_cm3, used_volume_cm3, max_sku_count,
                location_type, status)
             VALUES
               ('41000000-0000-0000-0000-000000000007',
                '41000000-0000-0000-0000-000000000001',
                '41000000-0000-0000-0000-000000000005',
                '41000000-0000-0000-0000-000000000006',
                'RC-L',1,1,1,1000,0,10,'storage','available');
             INSERT INTO products
               (id, owner_id, product_code, product_name, specification,
                storage_condition, special_drug_category, status)
             VALUES
               ('41000000-0000-0000-0000-000000000008',
                '41000000-0000-0000-0000-000000000001',
                'RC-P','RC product','1 box','normal_10_30','normal','active');
             INSERT INTO inventory_batches
               (id, owner_id, product_code, batch_no, production_date, expiry_date,
                qty_on_hand, qty_frozen, status, location_id, location_code)
             VALUES
               ('41000000-0000-0000-0000-000000000009',
                '41000000-0000-0000-0000-000000000001',
                'RC-P','B1','2026-01-01','2028-01-01',10,0,'quarantined',
                '41000000-0000-0000-0000-000000000007','RC-L');
             INSERT INTO reconciliation_runs
               (id, owner_id, window_key, request_hash, snapshot_at, created_by)
             VALUES
               ('41000000-0000-0000-0000-000000000010',
                '41000000-0000-0000-0000-000000000001',
                'progress','hash',now(),
                '41000000-0000-0000-0000-000000000002');
             INSERT INTO reconciliation_items
               (id, owner_id, run_id, product_code, batch_no, wms_qty, erp_qty,
                difference_qty, difference_type, resolution_status, disposition, resolved_by)
             VALUES
               ('41000000-0000-0000-0000-000000000003',
                '41000000-0000-0000-0000-000000000001',
                '41000000-0000-0000-0000-000000000010',
                'RC-P','B1',10,8,2,'wms_more','adjustment_pending','erp_truth',
                '41000000-0000-0000-0000-000000000002');
             INSERT INTO stock_adjustment_orders
               (id, owner_id, warehouse_id, order_no, adjustment_type, batch_id,
                product_code, batch_no, quantity, reason_code, source, status, created_by)
             VALUES
               ('41000000-0000-0000-0000-000000000004',
                '41000000-0000-0000-0000-000000000001',
                '41000000-0000-0000-0000-000000000005',
                'RC-SA','surplus',
                '41000000-0000-0000-0000-000000000009',
                'RC-P','B1',2,'other','manual','completed',
                '41000000-0000-0000-0000-000000000002');
             INSERT INTO reconciliation_item_adjustments
               (item_id, owner_id, inventory_batch_id, quantity, adjustment_order_id, created_at)
             VALUES
               ('41000000-0000-0000-0000-000000000003',
                '41000000-0000-0000-0000-000000000001',
                '41000000-0000-0000-0000-000000000009',
                2,'41000000-0000-0000-0000-000000000004',now());
             INSERT INTO reconciliation_item_locks
               (item_id, inventory_batch_id, owner_id, previous_status, locked_at)
             VALUES
               ('41000000-0000-0000-0000-000000000003',
                '41000000-0000-0000-0000-000000000009',
                '41000000-0000-0000-0000-000000000001','qualified',now());",
        )
        .execute(&pool)
        .await
        .unwrap();
        let ctx = AuthContext {
            user_id,
            owner_id,
            actor_name: "rc-progress-test".to_string(),
            permissions: Vec::new(),
            jti: "rc-progress-test".to_string(),
            warehouse_scope: None,
        };
        let now = Utc::now();
        let mut tx = pool.begin().await.unwrap();
        advance_from_stock_adjustment_in_tx(&mut tx, &ctx, adjustment_id, "completed", now)
            .await
            .unwrap();
        advance_from_stock_adjustment_in_tx(&mut tx, &ctx, adjustment_id, "completed", now)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
               FROM audit_event
              WHERE owner_id=$1
                AND module='M-RC'
                AND action='advance_reconciliation_adjustment'
                AND resource_id=$2",
        )
        .bind(owner_id)
        .bind(item_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit_count, 1);
    }
}
