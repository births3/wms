use super::*;

#[derive(FromRow)]
struct PutawayTaskLine {
    line_no: i32,
    product_id: Option<Uuid>,
    product_code: String,
    batch_no: Option<String>,
    planned_qty: i64,
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
               COALESCE(SUM(inspection.accepted_qty), 0)::BIGINT AS planned_qty
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
