async fn publish_customer_portal_order_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    order: &OutboundOrder,
    shipped_at: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let customer: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT customer_code, customer_name, updated_at
         FROM customers
         WHERE owner_id = $1 AND id = $2",
    )
    .bind(order.owner_id)
    .bind(order.customer_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((customer_code, customer_name, customer_updated_at)) = customer else {
        return Ok(());
    };
    let address: Option<(String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT detail_address, updated_at
         FROM customer_addresses
         WHERE owner_id = $1 AND customer_id = $2 AND id = $3",
    )
    .bind(order.owner_id)
    .bind(order.customer_id)
    .bind(order.delivery_address_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((address_name, address_updated_at)) = address else {
        return Ok(());
    };
    let lines: Vec<(Uuid, Uuid, String, String, String, i64)> = sqlx::query_as(
        r#"
        SELECT line.id,
               COALESCE(product.id, '00000000-0000-0000-0000-000000000000'::uuid),
               line.product_code,
               COALESCE(product.product_name, line.product_code),
               line.batch_no,
               line.shipped_qty
          FROM outbound_order_lines line
     LEFT JOIN products product
            ON product.owner_id = line.owner_id
           AND product.product_code = line.product_code
         WHERE line.owner_id = $1 AND line.outbound_order_id = $2
         ORDER BY line.line_no
        "#,
    )
    .bind(order.owner_id)
    .bind(order.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let payload = serde_json::json!({
        "projection_event_type": "customer_order.snapshot",
        "customer": {
            "id": order.customer_id,
            "customer_code": customer_code,
            "customer_name": customer_name,
            "updated_at": customer_updated_at
        },
        "address": {
            "id": order.delivery_address_id,
            "customer_id": order.customer_id,
            "address_code": order.delivery_address_id.to_string(),
            "address_name": address_name,
            "address_snapshot": order.delivery_address_snapshot,
            "updated_at": address_updated_at
        },
        "order": {
            "id": order.id,
            "customer_id": order.customer_id,
            "order_no": order.wms_order_no,
            "status": "shipped",
            "delivery_address_id": order.delivery_address_id,
            "address_snapshot": order.delivery_address_snapshot,
            "shipped_at": shipped_at,
            "signed_at": null,
            "updated_at": order.updated_at,
            "lines": lines.into_iter().map(
                |(id, product_id, product_code, product_name, batch_no, quantity)| {
                    serde_json::json!({
                        "id": id,
                        "product_id": product_id,
                        "product_code": product_code,
                        "product_name": product_name,
                        "batch_no": batch_no,
                        "quantity": quantity
                    })
                },
            ).collect::<Vec<_>>()
        }
    });
    publish_event_in_tx(
        tx,
        order.owner_id,
        &format!("portal-outbound-shipped:{}", order.id),
        "portal.customer_order.snapshot",
        "M4",
        "outbound_order",
        &order.id.to_string(),
        payload,
        order.updated_at,
    )
    .await
    .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))?;
    Ok(())
}
