#[derive(FromRow)]
struct OutboundShipmentRow {
    id: Uuid,
    delivery_provider_type: String,
    vehicle_no: Option<String>,
    plate_no: String,
    driver_user_id: Option<Uuid>,
    driver_name: Option<String>,
    courier_name: Option<String>,
    courier_phone: Option<String>,
    signature_attachment_id: Option<Uuid>,
    cold_chain: bool,
    loading_temperature_celsius: Option<f64>,
    cold_chain_packages: serde_json::Value,
    package_count: i32,
    handover_by: Uuid,
    shipped_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

const OUTBOUND_SHIPMENT_SELECT: &str = r#"
    SELECT id, delivery_provider_type, vehicle_no, plate_no, driver_user_id,
           driver_name, courier_name, courier_phone, signature_attachment_id,
           cold_chain, loading_temperature_celsius, cold_chain_packages,
           package_count, handover_by, shipped_at, created_at
      FROM outbound_shipments
     WHERE owner_id = $1 AND outbound_order_id = $2
"#;

async fn load_outbound_shipment(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<Option<OutboundShipment>, Wave4RepositoryError> {
    sqlx::query_as::<_, OutboundShipmentRow>(OUTBOUND_SHIPMENT_SELECT)
        .bind(owner_id)
        .bind(order_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .map(map_outbound_shipment)
        .transpose()
}

async fn load_outbound_shipment_from_pool(
    pool: &PgPool,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<Option<OutboundShipment>, Wave4RepositoryError> {
    sqlx::query_as::<_, OutboundShipmentRow>(OUTBOUND_SHIPMENT_SELECT)
        .bind(owner_id)
        .bind(order_id)
        .fetch_optional(pool)
        .await
        .map_err(map_db_error)?
        .map(map_outbound_shipment)
        .transpose()
}

async fn outbound_order_requires_cold_chain(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<bool, Wave4RepositoryError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM outbound_order_lines line
              JOIN products product
                ON product.owner_id = line.owner_id
               AND product.product_code = line.product_code
             WHERE line.owner_id = $1
               AND line.outbound_order_id = $2
               AND product.storage_condition IN ('cold', 'frozen')
        )
        "#,
    )
    .bind(owner_id)
    .bind(order_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn outbound_driver_name(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &ShipOutboundOrderRequest,
) -> Result<Option<String>, Wave4RepositoryError> {
    let Some(driver_user_id) = request.driver_user_id else {
        return Ok(None);
    };
    sqlx::query_scalar(
        r#"
        SELECT user_account.display_name
          FROM auth_users user_account
          JOIN auth_user_owner_bindings binding
            ON binding.user_id = user_account.id
           AND binding.owner_id = $1
           AND binding.is_active = TRUE
         WHERE user_account.id = $2
           AND user_account.status = 'active'
        "#,
    )
    .bind(owner_id)
    .bind(driver_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::InvalidDriver)
    .map(Some)
}

async fn ensure_handover_signature_attachment(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    attachment_id: Option<Uuid>,
) -> Result<(), Wave4RepositoryError> {
    let Some(attachment_id) = attachment_id else {
        return Ok(());
    };
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM attachments
             WHERE id = $1
               AND owner_id = $2
               AND module = 'M4'
               AND entity_type = 'outbound_handover_signature'
               AND entity_id = $3
               AND content_type IN ('image/jpeg', 'image/png')
        )
        "#,
    )
    .bind(attachment_id)
    .bind(owner_id)
    .bind(order_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if valid {
        Ok(())
    } else {
        Err(Wave4RepositoryError::InvalidSignatureAttachment)
    }
}

fn map_outbound_shipment(
    row: OutboundShipmentRow,
) -> Result<OutboundShipment, Wave4RepositoryError> {
    Ok(OutboundShipment {
        id: row.id,
        delivery_provider_type: row.delivery_provider_type,
        vehicle_no: row.vehicle_no,
        plate_no: row.plate_no,
        driver_user_id: row.driver_user_id,
        driver_name: row.driver_name,
        courier_name: row.courier_name,
        courier_phone: row.courier_phone,
        signature_attachment_id: row.signature_attachment_id,
        cold_chain: row.cold_chain,
        loading_temperature_celsius: row.loading_temperature_celsius,
        cold_chain_packages: serde_json::from_value::<Vec<OutboundColdChainPackage>>(
            row.cold_chain_packages,
        )
        .map_err(|error| Wave4RepositoryError::Serialize(error.to_string()))?,
        package_count: u32::try_from(row.package_count)
            .map_err(|_| Wave4RepositoryError::InvalidQuantity)?,
        handover_by: row.handover_by,
        shipped_at: row.shipped_at,
        created_at: row.created_at,
    })
}
