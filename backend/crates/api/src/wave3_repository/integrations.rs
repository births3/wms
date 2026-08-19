use super::*;

impl PgWave3Repository {
    pub async fn attach_erp_receiving_identity(
        &self,
        owner_id: Uuid,
        order_id: Uuid,
        erp_bill_id: i64,
        erp_bill_code: &str,
        revision: i32,
        line_no: i32,
        partner_type: Option<&str>,
        partner_id: Option<Uuid>,
        partner_code: Option<&str>,
        correlation_id: &str,
    ) -> Result<(), Wave3RepositoryError> {
        sqlx::query(
            r#"
            UPDATE receiving_orders
               SET erp_bill_id=$3, erp_bill_code=$4, erp_revision=$5, erp_line_no=$6,
                   partner_type=$7, partner_id=$8, partner_code=$9,
                   erp_correlation_id=$10
             WHERE owner_id=$1 AND id=$2
            "#,
        )
        .bind(owner_id)
        .bind(order_id)
        .bind(erp_bill_id)
        .bind(erp_bill_code)
        .bind(revision)
        .bind(line_no)
        .bind(partner_type)
        .bind(partner_id)
        .bind(partner_code)
        .bind(correlation_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }
}

#[derive(FromRow)]
struct PutawayTaskLine {
    line_no: i32,
    product_id: Option<Uuid>,
    product_code: String,
    batch_no: Option<String>,
    planned_qty: wms_domain::Quantity,
}

pub(super) async fn create_putaway_tasks_for_receiving_order(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    order: &ReceivingOrderRow,
    now: DateTime<Utc>,
) -> Result<(), Wave3RepositoryError> {
    let task_lines = sqlx::query_as::<_, PutawayTaskLine>(
        r#"
        SELECT line.line_no,
               line.product_id,
               line.product_code,
               line.batch_no,
               COALESCE(SUM(inspection.accepted_qty), 0) AS planned_qty
          FROM receiving_order_lines line
          JOIN receiving_inspections inspection
            ON inspection.owner_id = line.owner_id
           AND inspection.receiving_order_id = line.receiving_order_id
           AND inspection.batch_no = line.batch_no
         WHERE line.owner_id = $1
           AND line.receiving_order_id = $2
         GROUP BY line.line_no, line.product_id, line.product_code, line.batch_no
        HAVING COALESCE(SUM(inspection.accepted_qty), 0) > 0
         ORDER BY line.line_no
        "#,
    )
    .bind(ctx.owner_id)
    .bind(order.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    for line in task_lines {
        crate::task_engine::create_task_in_tx(
            tx,
            ctx,
            &wms_domain::CreateWarehouseTaskRequest {
                task_type_code: wms_domain::TASK_TYPE_PUTAWAY.to_string(),
                source_module: "M2".to_string(),
                source_doc_type: "receiving_order".to_string(),
                source_doc_id: Some(order.id),
                source_doc_no: order.receipt_no.clone(),
                source_line_no: Some(line.line_no),
                source_task_key: format!("M2:putaway:{}:{}", order.id, line.line_no),
                warehouse_id: order.warehouse_id,
                task_group_code: crate::task_engine::default_task_group_code(order.warehouse_id),
                product_id: line.product_id,
                product_code: line.product_code,
                batch_id: None,
                batch_no: line.batch_no,
                planned_qty: line.planned_qty,
                source_location_id: None,
                source_location_code: None,
                target_location_id: None,
                target_location_code: None,
                priority: None,
                urgent_order: false,
                predecessor_task_id: None,
            },
            now,
        )
        .await
        .map_err(|error| {
            Wave3RepositoryError::Database(format!("M-TE 上架任务创建失败: {error:?}"))
        })?;
    }
    Ok(())
}
