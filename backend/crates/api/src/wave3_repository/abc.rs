use super::*;
use wms_domain::{
    InventoryAbcClassification, InventoryAbcListResponse, InventoryAbcQuery,
    OverrideInventoryAbcRequest, PageMeta, RecomputeInventoryAbcRequest,
};

#[derive(FromRow)]
struct AbcRow {
    id: Uuid,
    owner_id: Uuid,
    product_code: String,
    abc_class: String,
    score: f64,
    outbound_qty: i64,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    source: String,
    override_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub async fn list_abc_classifications(
        &self,
        ctx: &AuthContext,
        query: &InventoryAbcQuery,
    ) -> Result<InventoryAbcListResponse, Wave3RepositoryError> {
        let abc_class = query
            .abc_class
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let product = query
            .product_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let rows = sqlx::query_as::<_, AbcRow>(
            r#"
            SELECT id, owner_id, product_code, abc_class, score::float8 AS score, outbound_qty,
                   period_start, period_end, source, override_reason, created_at, updated_at
              FROM inventory_abc_classifications
             WHERE owner_id = $1
               AND ($2::text IS NULL OR abc_class = $2)
               AND ($3::text IS NULL OR product_code ILIKE '%' || $3 || '%')
             ORDER BY abc_class ASC, score DESC, product_code ASC
             LIMIT 500
            "#,
        )
        .bind(ctx.owner_id)
        .bind(abc_class)
        .bind(product)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let data: Vec<_> = rows.into_iter().map(map_abc).collect();
        let count = data.len() as u32;
        Ok(InventoryAbcListResponse {
            data,
            page: PageMeta {
                next_cursor: None,
                count,
            },
        })
    }

    pub async fn recompute_abc_classifications(
        &self,
        ctx: &AuthContext,
        req: RecomputeInventoryAbcRequest,
        now: DateTime<Utc>,
    ) -> Result<InventoryAbcListResponse, Wave3RepositoryError> {
        let days = req.period_days.unwrap_or(30).clamp(7, 366);
        let period_end = now.date_naive();
        let period_start = period_end - chrono::Duration::days(days);
        let stats: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT product_code, COALESCE(SUM(ABS(qty_delta)), 0)::BIGINT
              FROM inventory_movements m
              JOIN inventory_batches b ON b.id = m.batch_id AND b.owner_id = m.owner_id
             WHERE m.owner_id = $1
               AND m.movement_type IN ('outbound_ship', 'stock_loss')
               AND m.occurred_at::date BETWEEN $2 AND $3
             GROUP BY product_code
            "#,
        )
        .bind(ctx.owner_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        let mut ranked = stats;
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let total: i64 = ranked.iter().map(|(_, qty)| *qty).sum();
        let mut cumulative = 0i64;
        let mut tx = self.begin().await?;
        sqlx::query(
            r#"
            DELETE FROM inventory_abc_classifications
             WHERE owner_id = $1 AND period_end = $2 AND source = 'system'
            "#,
        )
        .bind(ctx.owner_id)
        .bind(period_end)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        for (product_code, outbound_qty) in ranked {
            cumulative += outbound_qty;
            let ratio = if total > 0 {
                cumulative as f64 / total as f64
            } else {
                1.0
            };
            let abc_class = if ratio <= 0.8 {
                "A"
            } else if ratio <= 0.95 {
                "B"
            } else {
                "C"
            };
            let score = outbound_qty as f64;
            sqlx::query(
                r#"
                INSERT INTO inventory_abc_classifications (
                    id, owner_id, product_code, abc_class, score, outbound_qty,
                    period_start, period_end, source, created_at, updated_at
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'system',$9,$9)
                ON CONFLICT (owner_id, product_code, period_end) DO UPDATE
                  SET abc_class = EXCLUDED.abc_class,
                      score = EXCLUDED.score,
                      outbound_qty = EXCLUDED.outbound_qty,
                      period_start = EXCLUDED.period_start,
                      source = CASE
                        WHEN inventory_abc_classifications.source = 'manual' THEN 'manual'
                        ELSE 'system'
                      END,
                      updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&product_code)
            .bind(abc_class)
            .bind(score)
            .bind(outbound_qty)
            .bind(period_start)
            .bind(period_end)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let mut audit_event = crate::audit::AuditWriteRequest::from_auth_context(
            ctx,
            "recompute_inventory_abc",
            "M3",
            "inventory_abc_classification",
            period_end.to_string(),
            None,
        );
        audit_event.occurred_at = now;
        crate::audit::append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        self.list_abc_classifications(ctx, &InventoryAbcQuery::default())
            .await
    }

    pub async fn override_abc_classification(
        &self,
        ctx: &AuthContext,
        req: OverrideInventoryAbcRequest,
        now: DateTime<Utc>,
    ) -> Result<InventoryAbcClassification, Wave3RepositoryError> {
        let class = req.abc_class.trim().to_uppercase();
        if !matches!(class.as_str(), "A" | "B" | "C") {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        if req.reason.trim().is_empty() {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        let period_end = now.date_naive();
        let period_start = period_end - chrono::Duration::days(30);
        let mut tx = self.begin().await?;
        let row = sqlx::query_as::<_, AbcRow>(
            r#"
            INSERT INTO inventory_abc_classifications (
                id, owner_id, product_code, abc_class, score, outbound_qty,
                period_start, period_end, source, override_reason, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,0,0,$5,$6,'manual',$7,$8,$8)
            ON CONFLICT (owner_id, product_code, period_end) DO UPDATE
              SET abc_class = EXCLUDED.abc_class,
                  source = 'manual',
                  override_reason = EXCLUDED.override_reason,
                  updated_at = EXCLUDED.updated_at
            RETURNING id, owner_id, product_code, abc_class, score::float8 AS score, outbound_qty,
                      period_start, period_end, source, override_reason, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(req.product_code.trim())
        .bind(&class)
        .bind(period_start)
        .bind(period_end)
        .bind(req.reason.trim())
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value = map_abc(row);
        let mut audit_event = crate::audit::AuditWriteRequest::from_auth_context(
            ctx,
            "override_inventory_abc",
            "M3",
            "inventory_abc_classification",
            value.id.to_string(),
            None,
        );
        audit_event.occurred_at = now;
        crate::audit::append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }
}

fn map_abc(row: AbcRow) -> InventoryAbcClassification {
    InventoryAbcClassification {
        id: row.id,
        owner_id: row.owner_id,
        product_code: row.product_code,
        abc_class: row.abc_class,
        score: row.score,
        outbound_qty: row.outbound_qty,
        period_start: row.period_start,
        period_end: row.period_end,
        source: row.source,
        override_reason: row.override_reason,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
