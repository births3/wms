use super::*;

impl PgWave3Repository {
    pub async fn get_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        let row = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            SELECT id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                   external_ref, status, expected_arrival_at, created_at, updated_at
              FROM receiving_orders
             WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let lines = self.load_receiving_order_lines(ctx.owner_id, id).await?;
        Ok(map_receiving_order(row, lines))
    }

    pub async fn release_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        let updated = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            UPDATE receiving_orders
               SET status = 'released', updated_at = $3, version = version + 1
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                      external_ref, status, expected_arrival_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let lines = self.load_receiving_order_lines(ctx.owner_id, id).await?;
        Ok(map_receiving_order(updated, lines))
    }
}
