impl PgWave3Repository {
async fn begin(&self) -> Result<Transaction<'_, Postgres>, Wave3RepositoryError> {
        self.pool.begin().await.map_err(map_db_error)
    }

    async fn load_receiving_order_lines(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<Vec<ReceivingOrderLine>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, ReceivingOrderLineRow>(
            r#"
            SELECT line_no, product_id, product_code, expected_qty, batch_no,
                   production_date, expiry_date
              FROM receiving_order_lines
             WHERE receiving_order_id = $1 AND owner_id = $2
             ORDER BY line_no
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(map_receiving_order_line).collect())
    }
}
