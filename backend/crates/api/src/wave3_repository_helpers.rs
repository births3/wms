use serde::de::DeserializeOwned;

async fn lock_receiving_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<ReceivingOrderRow, Wave3RepositoryError> {
    sqlx::query_as::<_, ReceivingOrderRow>(
        r#"
        SELECT id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
               external_ref, status, expected_arrival_at, created_at, updated_at
          FROM receiving_orders
         WHERE id = $1 AND owner_id = $2
         FOR UPDATE
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::NotFound)
}

async fn load_receiving_order_lines_in_tx(
    tx: &mut Transaction<'_, Postgres>,
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
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(map_receiving_order_line).collect())
}

async fn ensure_owned_reference(
    tx: &mut Transaction<'_, Postgres>,
    table: &'static str,
    owner_id: Uuid,
    id: Uuid,
) -> Result<(), Wave3RepositoryError> {
    let query = match table {
        "suppliers" => "SELECT EXISTS(SELECT 1 FROM suppliers WHERE owner_id = $1 AND id = $2 AND status = 'active')",
        "warehouses" => "SELECT EXISTS(SELECT 1 FROM warehouses WHERE owner_id = $1 AND id = $2 AND status = 'active')",
        "products" => "SELECT EXISTS(SELECT 1 FROM products WHERE owner_id = $1 AND id = $2 AND status = 'active')",
        _ => {
            return Err(Wave3RepositoryError::Serialize(
                "invalid reference table".to_string(),
            ))
        }
    };
    let exists: bool = sqlx::query_scalar(query)
        .bind(owner_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave3RepositoryError::NotFound)
    }
}

async fn ensure_cold_chain_device_active(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    device_code: &str,
) -> Result<(), Wave3RepositoryError> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM cold_chain_devices
             WHERE owner_id = $1 AND device_code = $2 AND status = 'active'
        )
        "#,
    )
    .bind(owner_id)
    .bind(device_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave3RepositoryError::NotFound)
    }
}

async fn load_temperature_reading(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    device_code: &str,
    captured_at: DateTime<Utc>,
) -> Result<Option<TemperatureReading>, Wave3RepositoryError> {
    let row = sqlx::query_as::<_, TemperatureReadingRow>(
        r#"
        SELECT id, owner_id, device_code, temperature_celsius, humidity_percent,
               captured_at, external_report_url, out_of_range
          FROM temperature_readings
         WHERE owner_id = $1 AND device_code = $2 AND captured_at = $3
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(device_code)
    .bind(captured_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.map(map_temperature_reading))
}

async fn load_temperature_excursion(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    external_event_id: &str,
) -> Result<Option<TemperatureExcursionEvent>, Wave3RepositoryError> {
    let row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
        r#"
        SELECT id, owner_id, external_event_id, device_code, location_code,
               started_at, ended_at, min_temperature_celsius,
               max_temperature_celsius, affected_batch_ids, status, created_at
          FROM temperature_excursion_events
         WHERE owner_id = $1 AND external_event_id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(external_event_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.map(map_temperature_excursion))
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, Wave3RepositoryError> {
    idempotency::replay_hash_only(tx, owner_id, idempotency_key, request_hash, now)
        .await
        .map_err(Into::into)
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Wave3RepositoryError> {
    idempotency::lock_key(tx, "wave3", owner_id, idempotency_key)
        .await
        .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), Wave3RepositoryError> {
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        &resource_id,
        response,
        now,
    )
    .await
    .map_err(Into::into)
}

fn request_hash(value: &serde_json::Value) -> Result<String, Wave3RepositoryError> {
    idempotency::request_hash(value).map_err(Into::into)
}

async fn insert_receiving_order_lines(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    receiving_order_id: Uuid,
    lines: &[ReceivingOrderLine],
) -> Result<(), Wave3RepositoryError> {
    for line in lines {
        sqlx::query(
            r#"
            INSERT INTO receiving_order_lines (
                id, receiving_order_id, owner_id, line_no, product_id,
                product_code, expected_qty, batch_no, production_date, expiry_date
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(receiving_order_id)
        .bind(owner_id)
        .bind(i32::try_from(line.line_no).map_err(|_| Wave3RepositoryError::InvalidQuantity)?)
        .bind(line.product_id)
        .bind(&line.product_code)
        .bind(line.expected_qty)
        .bind(&line.batch_no)
        .bind(parse_optional_date(line.production_date.as_deref())?)
        .bind(parse_optional_date(line.expiry_date.as_deref())?)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

async fn insert_receiving_order_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    mut req: CreateReceivingOrderRequest,
    now: DateTime<Utc>,
) -> Result<ReceivingOrder, Wave3RepositoryError> {
    let id = Uuid::new_v4();
    let receipt_no = if req.receipt_no.trim().is_empty() {
        crate::document_numbering::PgDocumentNumberingService::new()
            .generate_in_tx(
                tx,
                ctx,
                crate::document_numbering::GenerateDocumentNumberRequest {
                    document_type: req.document_type.clone(),
                    idempotency_key: format!("m2-asn-create:{id}"),
                    source_module: "M2".to_string(),
                    source_document_id: Some(id),
                },
                now,
            )
            .await
            .map_err(|error| Wave3RepositoryError::DocumentNumbering(format!("{error:?}")))?
            .value
            .generated_no
    } else {
        req.receipt_no.clone()
    };
    for line in &mut req.lines {
        if line.product_id.is_none() {
            line.product_id = sqlx::query_scalar(
                "SELECT id FROM products WHERE owner_id = $1 AND product_code = $2 AND status = 'active'",
            )
            .bind(ctx.owner_id)
            .bind(&line.product_code)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
        }
    }
    sqlx::query(
        r#"
        INSERT INTO receiving_orders (
            id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
            external_ref, status, expected_arrival_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'draft', $8, $9, $9)
        "#,
    )
    .bind(id)
    .bind(ctx.owner_id)
    .bind(&receipt_no)
    .bind(&req.document_type)
    .bind(req.supplier_id)
    .bind(req.warehouse_id)
    .bind(&req.external_ref)
    .bind(req.expected_arrival_at)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    insert_receiving_order_lines(&mut *tx, ctx.owner_id, id, &req.lines).await?;
    Ok(ReceivingOrder {
        id,
        owner_id: ctx.owner_id,
        receipt_no,
        document_type: req.document_type,
        supplier_id: req.supplier_id,
        warehouse_id: req.warehouse_id,
        external_ref: req.external_ref,
        status: "draft".to_string(),
        expected_arrival_at: req.expected_arrival_at,
        lines: req.lines,
        created_at: now,
        updated_at: now,
    })
}

fn validate_document_type(value: &str) -> Result<(), Wave3RepositoryError> {
    match value {
        RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND | RECEIVING_DOCUMENT_TYPE_SALES_RETURN => Ok(()),
        _ => Err(Wave3RepositoryError::InvalidDocumentType),
    }
}

fn map_request_validation_error(
    error: ReceivingOrderRequestValidationError,
) -> Wave3RepositoryError {
    match error {
        ReceivingOrderRequestValidationError::MissingSupplier => {
            Wave3RepositoryError::MissingSupplier
        }
        ReceivingOrderRequestValidationError::MissingExpectedArrival => {
            Wave3RepositoryError::MissingExpectedArrival
        }
        ReceivingOrderRequestValidationError::InvalidExpectedArrival => {
            Wave3RepositoryError::InvalidExpectedArrival
        }
        ReceivingOrderRequestValidationError::MissingProduct => {
            Wave3RepositoryError::MissingProduct
        }
        ReceivingOrderRequestValidationError::MultipleProducts => {
            Wave3RepositoryError::MultipleProducts
        }
    }
}

fn validate_receiving_order_lines(
    document_type: &str,
    lines: &[ReceivingOrderLine],
) -> Result<(), Wave3RepositoryError> {
    if lines.is_empty() {
        return Err(Wave3RepositoryError::InvalidQuantity);
    }

    for line in lines {
        if line.line_no == 0 || line.expected_qty <= wms_domain::Quantity::ZERO {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let has_batch = line
            .batch_no
            .as_deref()
            .is_some_and(|batch_no| !batch_no.trim().is_empty());
        match document_type {
            RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND
                if line.batch_no.is_some()
                    || line.production_date.is_some()
                    || line.expiry_date.is_some() =>
            {
                return Err(Wave3RepositoryError::InvalidBatchPolicy);
            }
            RECEIVING_DOCUMENT_TYPE_SALES_RETURN if !has_batch => {
                return Err(Wave3RepositoryError::InvalidBatchPolicy);
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_date(value: &str) -> Result<NaiveDate, Wave3RepositoryError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| Wave3RepositoryError::InvalidDate(value.to_string()))
}

fn map_db_error(error: sqlx::Error) -> Wave3RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave3RepositoryError::DuplicateCode;
        }
    }
    Wave3RepositoryError::Database(error.to_string())
}

fn map_receipt_insert_error(error: sqlx::Error) -> Wave3RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave3RepositoryError::DuplicateReceipt;
        }
    }
    map_db_error(error)
}

fn validate_receiving_gsp_fields(
    req: &ReceiveReceivingOrderRequest,
) -> Result<(), Wave3RepositoryError> {
    let details = req
        .details
        .as_ref()
        .ok_or_else(|| Wave3RepositoryError::MissingRequiredField("details".to_string()))?;
    let required = [
        ("vehicle_no", details.vehicle_no.as_deref()),
        ("origin", details.origin.as_deref()),
        ("transport_mode", details.transport_mode.as_deref()),
        ("carrier", details.carrier.as_deref()),
        ("contact_name", details.contact_name.as_deref()),
        ("contact_phone", details.contact_phone.as_deref()),
        ("contact_id_no", details.contact_id_no.as_deref()),
        ("seal_checked", details.seal_checked.as_deref()),
        ("filing_checked", details.filing_checked.as_deref()),
    ];
    for (field, value) in required {
        if value
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .is_none()
        {
            return Err(Wave3RepositoryError::MissingRequiredField(
                field.to_string(),
            ));
        }
    }
    if details.departure_at.is_none() {
        return Err(Wave3RepositoryError::MissingRequiredField(
            "departure_at".to_string(),
        ));
    }
    if details.arrival_at.is_none() {
        return Err(Wave3RepositoryError::MissingRequiredField(
            "arrival_at".to_string(),
        ));
    }
    if details.storage_at.is_none() {
        return Err(Wave3RepositoryError::MissingRequiredField(
            "storage_at".to_string(),
        ));
    }
    Ok(())
}

async fn order_requires_cold_chain(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    receiving_order_id: Uuid,
) -> Result<bool, Wave3RepositoryError> {
    let cold: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM receiving_order_lines line
              JOIN products product
                ON product.owner_id = line.owner_id
               AND product.product_code = line.product_code
             WHERE line.receiving_order_id = $1
               AND line.owner_id = $2
               AND (
                    product.storage_condition ILIKE '%cold%'
                    OR product.storage_condition ILIKE '%cool%'
                    OR product.storage_condition ILIKE '%冻%'
                    OR product.storage_condition ILIKE '%冷%'
                    OR product.storage_condition IN ('cold', 'cool', 'frozen', 'refrigerated')
               )
        )
        "#,
    )
    .bind(receiving_order_id)
    .bind(owner_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(cold)
}

/// 按商品储存条件汇总到货温度允许区间（最严格取交集的近似：冷冻最严）。
async fn receiving_temperature_band(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    receiving_order_id: Uuid,
) -> Result<(f64, f64), Wave3RepositoryError> {
    let conditions: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT COALESCE(product.storage_condition, 'normal')
          FROM receiving_order_lines line
          JOIN products product
            ON product.owner_id = line.owner_id
           AND product.product_code = line.product_code
         WHERE line.receiving_order_id = $1
           AND line.owner_id = $2
        "#,
    )
    .bind(receiving_order_id)
    .bind(owner_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut min_t = f64::NEG_INFINITY;
    let mut max_t = f64::INFINITY;
    for condition in conditions {
        let lower = condition.to_ascii_lowercase();
        let (lo, hi) = if lower.contains("frozen") || lower.contains('冻') {
            (-25.0, -15.0)
        } else if lower.contains("cool") {
            (8.0, 15.0)
        } else if lower.contains("cold") || lower.contains("refrigerat") || lower.contains('冷') {
            (2.0, 8.0)
        } else {
            continue;
        };
        min_t = min_t.max(lo);
        max_t = max_t.min(hi);
    }
    if !min_t.is_finite() || !max_t.is_finite() || min_t > max_t {
        // 默认冷藏带
        return Ok((2.0, 8.0));
    }
    Ok((min_t, max_t))
}

fn validate_inspection_quality_checks(
    req: &InspectReceivingOrderRequest,
) -> Result<serde_json::Value, Wave3RepositoryError> {
    let required = [
        ("appearance", req.appearance_check.as_deref()),
        ("package", req.package_check.as_deref()),
        ("instruction", req.instruction_check.as_deref()),
        ("label", req.label_check.as_deref()),
    ];
    let mut checks = serde_json::Map::new();
    for (field, value) in required {
        let trimmed = value
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| Wave3RepositoryError::MissingRequiredField(field.to_string()))?;
        checks.insert(
            field.to_string(),
            serde_json::Value::String(trimmed.to_string()),
        );
    }
    Ok(serde_json::Value::Object(checks))
}

async fn enqueue_unqualified_quality_liaison(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    receiving_order_id: Uuid,
    receipt_no: &str,
    inspection: &ReceivingInspectionRecord,
    now: DateTime<Utc>,
) -> Result<(), Wave3RepositoryError> {
    let content = format!(
        "入库验收不合格：ASN {} 批号 {} 拒收数量 {}，已触发质量联系单/通知。",
        receipt_no, inspection.batch_no, inspection.rejected_qty
    );
    let dedupe_key = format!(
        "m2-inspect-unqualified:{}:{}:{}",
        receiving_order_id, inspection.batch_no, inspection.id
    );
    sqlx::query(
        r#"
        INSERT INTO h4_notification_records (
            id, owner_id, config_id, event_type, dedupe_key, recipient, channel,
            content, content_summary, status, retry_count, failure_reason, sent_at,
            created_at, updated_at
        ) VALUES (
            $1, $2, NULL, 'm2.inspect.unqualified', $3, 'warehouse_manager', 'wechat',
            $4, $5, 'retrying', 0, 'awaiting_wechat_delivery', NULL, $6, $6
        )
        ON CONFLICT (owner_id, event_type, recipient, dedupe_key) DO UPDATE
           SET content = EXCLUDED.content,
               content_summary = EXCLUDED.content_summary,
               updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(&dedupe_key)
    .bind(&content)
    .bind(&content)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    // 若货主已配置 inbound_unqualified 质量联系单类型，则直接建单（审批链路沿用 M-QL）。
    let type_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM quality_liaison_types
             WHERE owner_id = $1 AND type_code = 'inbound_unqualified' AND enabled
        )
        "#,
    )
    .bind(ctx.owner_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if type_exists {
        let liaison_id = Uuid::new_v4();
        let liaison_no = format!("QL-M2-{}", &liaison_id.to_string()[..8]);
        let payload = serde_json::json!({
            "receiving_order_id": receiving_order_id,
            "receipt_no": receipt_no,
            "batch_no": inspection.batch_no,
            "inspection_id": inspection.id,
            "rejected_qty": inspection.rejected_qty,
            "action": "create_stock_loss_pending",
        });
        sqlx::query(
            r#"
            INSERT INTO quality_liaison_orders (
                id, owner_id, liaison_no, type_code, related_document_type, related_document_no,
                problem_description, disposition_suggestion, trigger_source, business_payload,
                status, created_by, created_at, updated_at
            ) VALUES (
                $1, $2, $3, 'inbound_unqualified', 'receiving_order', $4,
                $5, '报损或采退', 'm2.inspect', $6,
                'pending_approval', $7, $8, $8
            )
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(liaison_id)
        .bind(ctx.owner_id)
        .bind(&liaison_no)
        .bind(receipt_no)
        .bind(&content)
        .bind(payload)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

async fn ensure_receiving_clerk_signer(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    user_id: Uuid,
) -> Result<(), Wave3RepositoryError> {
    let authorized: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM auth_user_owner_bindings binding
              JOIN auth_users user_row
                ON user_row.id = binding.user_id
              JOIN auth_user_roles user_role
                ON user_role.user_id = binding.user_id
               AND user_role.owner_id = binding.owner_id
              JOIN auth_roles role
                ON role.id = user_role.role_id
               AND role.owner_id = binding.owner_id
              JOIN auth_role_permissions role_permission
                ON role_permission.role_id = role.id
              JOIN auth_permissions permission
                ON permission.id = role_permission.permission_id
               AND permission.permission_code = 'm2.write'
             WHERE binding.user_id = $1
               AND binding.owner_id = $2
               AND binding.is_active
               AND user_row.status = 'active'
               AND role.role_code = 'receiving_clerk'
        )
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !authorized {
        return Err(Wave3RepositoryError::UnauthorizedSigner);
    }
    Ok(())
}
