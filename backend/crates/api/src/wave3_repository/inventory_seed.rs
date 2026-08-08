use super::*;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct ErpInventorySeedSnapshot {
    pub snapshot_id: String,
    pub warehouse_id: Uuid,
    pub push_type: i32,
    pub push_time: DateTime<Utc>,
    pub payload_digest: String,
    pub items: Vec<ErpInventorySeedItem>,
}

#[derive(Clone, Debug)]
pub struct ErpInventorySeedItem {
    pub row_no: i32,
    pub product_code: String,
    pub batch_no: String,
    pub expiry_date: Option<NaiveDate>,
    pub location_code: Option<String>,
    pub goods_status: Option<String>,
    pub quantity: wms_domain::Quantity,
}

impl PgWave3Repository {
    pub async fn stage_erp_inventory_snapshot(
        &self,
        ctx: &AuthContext,
        snapshot: ErpInventorySeedSnapshot,
        now: DateTime<Utc>,
    ) -> Result<IdempotentMutation<Uuid>, Wave3RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let existing: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id,payload_digest FROM erp_inventory_snapshot_staging WHERE owner_id=$1 AND snapshot_id=$2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(&snapshot.snapshot_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some((id, digest)) = existing {
            tx.commit().await.map_err(map_db_error)?;
            if digest == snapshot.payload_digest {
                return Ok(IdempotentMutation {
                    value: id,
                    replayed: true,
                });
            }
            return Err(Wave3RepositoryError::IdempotencyConflict);
        }
        let id = Uuid::new_v4();
        let quarantined_items = snapshot
            .items
            .iter()
            .filter(|item| {
                item.location_code
                    .as_deref()
                    .map_or(true, |value| value.trim().is_empty())
                    || item
                        .goods_status
                        .as_deref()
                        .map_or(true, |value| value.trim().is_empty())
            })
            .count();
        let status = if snapshot.push_type == 1 {
            "pending_approval"
        } else {
            "reconciliation_only"
        };
        sqlx::query(
            "INSERT INTO erp_inventory_snapshot_staging (id,owner_id,warehouse_id,snapshot_id,push_type,push_time,payload_digest,status,summary,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(snapshot.warehouse_id)
        .bind(&snapshot.snapshot_id)
        .bind(snapshot.push_type)
        .bind(snapshot.push_time)
        .bind(&snapshot.payload_digest)
        .bind(status)
        .bind(json!({"total_items": snapshot.items.len(), "quarantined_items": quarantined_items}))
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        for item in snapshot.items {
            sqlx::query(
                "INSERT INTO erp_inventory_snapshot_staging_items (id,snapshot_staging_id,owner_id,row_no,product_code,batch_no,expiry_date,location_code,goods_status,quantity,quarantined,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(ctx.owner_id)
            .bind(item.row_no)
            .bind(item.product_code.trim())
            .bind(item.batch_no.trim())
            .bind(item.expiry_date)
            .bind(item.location_code.as_deref().map(str::trim).filter(|value| !value.is_empty()))
            .bind(item.goods_status.as_deref().map(str::trim).filter(|value| !value.is_empty()))
            .bind(item.quantity)
            .bind(item.location_code.as_deref().map_or(true, |value| value.trim().is_empty())
                || item.goods_status.as_deref().map_or(true, |value| value.trim().is_empty()))
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "stage_erp_inventory_snapshot",
            "H8",
            "erp_inventory_snapshot",
            id.to_string(),
            Some(AuditDiff::compute(
                serde_json::Value::Null,
                json!({"snapshot_id": snapshot.snapshot_id, "push_type": snapshot.push_type, "status": status}),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: id,
            replayed: false,
        })
    }
}
