impl PgWave3Repository {
    pub async fn list_receiving_dashboard(
        &self,
        ctx: &AuthContext,
        query: &ReceivingDashboardQuery,
    ) -> Result<Vec<ReceivingDashboardRow>, Wave3RepositoryError> {
        let rows: Vec<(String, i64, wms_domain::Quantity, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
            r#"
            SELECT orders.status,
                   COUNT(DISTINCT orders.id)::BIGINT,
                   COALESCE(SUM(lines.expected_qty), 0),
                   MAX(orders.created_at)
              FROM receiving_orders orders
              LEFT JOIN receiving_order_lines lines
                ON lines.receiving_order_id = orders.id
               AND lines.owner_id = orders.owner_id
               AND ($3::TEXT IS NULL OR lines.product_code = $3)
             WHERE orders.owner_id = $1
               AND ($2::UUID IS NULL OR orders.supplier_id = $2)
               AND ($3::TEXT IS NULL OR EXISTS (
                    SELECT 1 FROM receiving_order_lines product_lines
                     WHERE product_lines.receiving_order_id = orders.id
                       AND product_lines.owner_id = orders.owner_id
                       AND product_lines.product_code = $3
               ))
               AND ($4::TIMESTAMPTZ IS NULL OR orders.expected_arrival_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR orders.expected_arrival_at <= $5)
             GROUP BY orders.status
             ORDER BY orders.status
            "#,
        )
        .bind(ctx.owner_id)
        .bind(query.supplier_id)
        .bind(query.product_code.as_deref())
        .bind(query.from)
        .bind(query.to)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows
            .into_iter()
            .map(|(status, order_count, expected_qty, created_at)| ReceivingDashboardRow {
                created_at,
                abnormal: matches!(status.as_str(), "closed_rejected" | "exception"),
                status,
                order_count,
                expected_qty,
            })
            .collect())
    }

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
            SELECT id, line_no, product_id, product_code, expected_qty, batch_no,
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
