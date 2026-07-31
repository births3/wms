// @governance: skip-page-size - 拆分文件仍包含同一事务族，当前优先保持商品、包装与映射溯源的原子边界。
use crate::idempotency;

impl From<crate::idempotency::IdempotencyError> for MasterDataError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(error) => {
                Self::Database(error.to_string())
            }
            crate::idempotency::IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}

impl PgMasterDataReadRepository {
    pub async fn batch_create_customers(
        &self,
        ctx: &AuthContext,
        requests: Vec<CreateCustomerRequest>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Vec<Customer>, MasterDataError> {
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/customers/batch-sync",
            "request": &requests,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Vec<Customer>>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/master-data/customers/batch-sync",
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }

        let mut customers = Vec::with_capacity(requests.len());
        for req in requests {
            let id = Uuid::new_v4();
            let source = req.source.unwrap_or_else(|| "api_import".to_string());
            let row = sqlx::query_as::<_, CustomerRow>(
                r#"
                INSERT INTO customers (
                    id, owner_id, customer_code, customer_name, customer_type,
                    license_no, source, status, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, 'customer', $5, $6, 'active', $7, $7)
                RETURNING id, owner_id, customer_code, customer_name, license_no, source,
                          status, created_at, updated_at
                "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.customer_code)
            .bind(&req.customer_name)
            .bind(&req.license_no)
            .bind(&source)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| map_catalog_write_error(error, &req.customer_code))?;
            let customer = Customer::from(row);
            append_master_data_audit(
                &mut tx,
                ctx,
                "batch_create_customer",
                "customer",
                customer.id,
                &customer,
                now,
            )
            .await?;
            customers.push(customer);
        }
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &customers,
            now,
            "POST",
            "/api/v1/master-data/customers/batch-sync",
            "customer_batch",
            idempotency_key,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(customers)
    }

    pub async fn batch_create_suppliers(
        &self,
        ctx: &AuthContext,
        requests: Vec<CreateSupplierRequest>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Vec<Supplier>, MasterDataError> {
        let requests = requests
            .into_iter()
            .map(|mut request| {
                request.license_no = normalize_supplier_uscc(request.license_no)?;
                Ok(request)
            })
            .collect::<Result<Vec<_>, MasterDataError>>()?;
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/suppliers/batch-sync",
            "request": &requests,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Vec<Supplier>>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/master-data/suppliers/batch-sync",
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }

        let mut suppliers = Vec::with_capacity(requests.len());
        for req in requests {
            let id = Uuid::new_v4();
            let source = req.source.unwrap_or_else(|| "api_import".to_string());
            let uscc = req
                .license_no
                .clone()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| req.supplier_code.clone());
            let row = sqlx::query_as::<_, SupplierRow>(
                r#"
                INSERT INTO suppliers (
                    id, owner_id, supplier_code, supplier_name, uscc, contact_name,
                    source, status, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', $8, $8)
                RETURNING id, owner_id, supplier_code, supplier_name, uscc, contact_name,
                          source, status, created_at, updated_at
                "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.supplier_code)
            .bind(&req.supplier_name)
            .bind(&uscc)
            .bind(&req.contact_name)
            .bind(&source)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| map_catalog_write_error(error, &req.supplier_code))?;
            let supplier = Supplier::from(row);
            append_master_data_audit(
                &mut tx,
                ctx,
                "batch_create_supplier",
                "supplier",
                supplier.id,
                &supplier,
                now,
            )
            .await?;
            suppliers.push(supplier);
        }
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &suppliers,
            now,
            "POST",
            "/api/v1/master-data/suppliers/batch-sync",
            "supplier_batch",
            idempotency_key,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(suppliers)
    }
}

fn location_drafts(
    owner_id: Uuid,
    req: &BatchCreateLocationsRequest,
    now: DateTime<Utc>,
) -> Result<Vec<Location>, MasterDataError> {
    let area_code = req.area_code.trim().to_uppercase();
    let valid_range = req.row_start >= 1
        && req.row_start <= req.row_end
        && req.row_end <= 99
        && req.column_start >= 1
        && req.column_start <= req.column_end
        && req.column_end <= 99
        && req.layer_start >= 1
        && req.layer_start <= req.layer_end
        && req.layer_end <= 99
        && req.max_volume_cm3 >= 0
        && req.max_sku_count > 0
        && !req.location_type.trim().is_empty()
        && area_code.len() == 3
        && area_code.chars().all(|item| item.is_ascii_alphanumeric());
    if !valid_range {
        return Err(MasterDataError::InvalidLocationBatchRange);
    }
    let total_count = (req.row_end - req.row_start + 1)
        * (req.column_end - req.column_start + 1)
        * (req.layer_end - req.layer_start + 1);
    if total_count > LOCATION_BATCH_MAX_COUNT {
        return Err(MasterDataError::InvalidLocationBatchRange);
    }

    let mut locations = Vec::with_capacity(total_count as usize);
    for row_no in req.row_start..=req.row_end {
        for column_no in req.column_start..=req.column_end {
            for layer_no in req.layer_start..=req.layer_end {
                locations.push(Location {
                    id: Uuid::new_v4(),
                    owner_id,
                    warehouse_id: req.warehouse_id,
                    zone_id: req.zone_id,
                    location_code: format!("{area_code}-{row_no:02}-{column_no:02}-{layer_no:02}"),
                    row_no,
                    column_no,
                    layer_no,
                    max_volume_cm3: req.max_volume_cm3,
                    used_volume_cm3: 0,
                    max_sku_count: req.max_sku_count,
                    location_type: req.location_type.clone(),
                    bound_owner_id: req.bound_owner_id,
                    status: "available".to_string(),
                    created_at: now,
                    updated_at: now,
                });
            }
        }
    }
    Ok(locations)
}

async fn ensure_enabled_dictionary_item(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    dict_code: &str,
    item_code: &str,
    now: DateTime<Utc>,
) -> Result<(), MasterDataError> {
    let exists: bool = sqlx::query_scalar(
        r#"
        WITH scoped_items AS (
            SELECT
                item.enabled,
                ROW_NUMBER() OVER (
                    PARTITION BY item.item_code
                    ORDER BY
                        CASE WHEN item.owner_id = $2 THEN 1 ELSE 0 END DESC,
                        item.updated_at DESC
                ) AS scope_rank
              FROM system_dictionary_items item
              JOIN system_dictionary_categories category
                ON category.dict_code = item.dict_code
               AND category.enabled = TRUE
             WHERE item.dict_code = $1
               AND item.item_code = $3
               AND (item.owner_id IS NULL OR item.owner_id = $2)
               AND (item.effective_from IS NULL OR item.effective_from <= $4)
               AND (item.effective_to IS NULL OR item.effective_to > $4)
        )
        SELECT EXISTS (
            SELECT 1
              FROM scoped_items
             WHERE scope_rank = 1
               AND enabled = TRUE
        )
        "#,
    )
    .bind(dict_code)
    .bind(owner_id)
    .bind(item_code)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(MasterDataError::InvalidLocationBatchRange)
    }
}

async fn ensure_warehouse_zone_in_owner(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
) -> Result<(), MasterDataError> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM warehouses warehouse
              JOIN warehouse_zones zone
                ON zone.owner_id = warehouse.owner_id
               AND zone.warehouse_id = warehouse.id
             WHERE warehouse.owner_id = $1
               AND warehouse.id = $2
               AND zone.id = $3
        )
        "#,
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(MasterDataError::NotFound)
    }
}

async fn ensure_bound_owner_exists(
    tx: &mut Transaction<'_, Postgres>,
    bound_owner_id: Option<Uuid>,
) -> Result<(), MasterDataError> {
    let Some(bound_owner_id) = bound_owner_id else {
        return Ok(());
    };
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM auth_owners WHERE id = $1)")
            .bind(bound_owner_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(MasterDataError::InvalidLocationOwner)
    }
}

async fn existing_location_code(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    location_codes: &[String],
) -> Result<Option<String>, MasterDataError> {
    sqlx::query_scalar(
        r#"
        SELECT location_code
          FROM warehouse_locations
         WHERE owner_id = $1
           AND location_code = ANY($2)
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(location_codes)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, MasterDataError> {
    idempotency::replay(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        now,
    )
    .await
    .map_err(Into::into)
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), MasterDataError> {
    idempotency::lock_key(tx, "master-data", owner_id, idempotency_key)
        .await
        .map_err(Into::into)
}

async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    response: &T,
    now: DateTime<Utc>,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: &str,
) -> Result<(), MasterDataError> {
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        resource_id,
        response,
        now,
    )
    .await
    .map_err(Into::into)
}

async fn append_master_data_audit<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), MasterDataError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| MasterDataError::Serialize(error.to_string()))?;
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M1",
        resource_type,
        resource_id.to_string(),
        Some(AuditDiff::compute(json!({}), response_body)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| MasterDataError::Audit(format!("{error:?}")))
}

async fn append_master_data_update_audit<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    before: Value,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), MasterDataError> {
    let after = serde_json::to_value(response)
        .map_err(|error| MasterDataError::Serialize(error.to_string()))?;
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M1",
        resource_type,
        resource_id.to_string(),
        Some(AuditDiff::compute(before, after)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| MasterDataError::Audit(format!("{error:?}")))
}

async fn load_master_data_before(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
    resource_type: &str,
) -> Result<Value, MasterDataError> {
    let sql = match resource_type {
        "product" => r#"
            SELECT to_jsonb(t) || jsonb_build_object(
                'packaging_levels',
                COALESCE((
                    SELECT jsonb_agg(
                        to_jsonb(level_row)
                        - 'owner_id' - 'product_id' - 'created_at' - 'updated_at'
                        ORDER BY level_row.sort_order
                    )
                      FROM product_packaging_levels level_row
                     WHERE level_row.owner_id = $1 AND level_row.product_id = $2
                ), '[]'::jsonb),
                'mapping_traces',
                COALESCE((
                    SELECT jsonb_agg(
                        to_jsonb(trace_row) - 'owner_id' - 'product_id'
                        ORDER BY trace_row.created_at, trace_row.id
                    )
                      FROM product_mapping_traces trace_row
                     WHERE trace_row.owner_id = $1 AND trace_row.product_id = $2
                ), '[]'::jsonb)
            )
              FROM (
                    SELECT id, owner_id, product_code, product_name, specification,
                           dosage_form, storage_condition, special_drug_category,
                           approval_no, manufacturer, udi_code,
                           electronic_regulatory_code, length_mm, width_mm, height_mm,
                           volume_cm3, weight_g, source, attrs, status, created_at, updated_at
                      FROM products
                     WHERE owner_id = $1 AND id = $2
              ) t
        "#,
        "supplier" => "SELECT to_jsonb(t) FROM (SELECT id, owner_id, supplier_code, supplier_name, uscc, contact_name, source, status, created_at, updated_at FROM suppliers WHERE owner_id=$1 AND id=$2) t",
        "customer" => "SELECT to_jsonb(t) FROM (SELECT id, owner_id, customer_code, customer_name, license_no, source, status, created_at, updated_at FROM customers WHERE owner_id=$1 AND id=$2) t",
        "warehouse" => "SELECT to_jsonb(t) FROM (SELECT id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at FROM warehouses WHERE owner_id=$1 AND id=$2) t",
        "location" => "SELECT to_jsonb(t) FROM (SELECT id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, bound_owner_id, status, created_at, updated_at FROM warehouse_locations WHERE owner_id=$1 AND id=$2) t",
        "warehouse_zone" => "SELECT to_jsonb(t) FROM (SELECT id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status, created_at, updated_at FROM warehouse_zones WHERE owner_id=$1 AND id=$2) t",
        _ => return Err(MasterDataError::Serialize("unsupported audit resource".into())),
    };
    sqlx::query_scalar(sql)
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)
}

async fn disable_warehouse_children(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    warehouse_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), MasterDataError> {
    let has_stock: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM inventory_batches batch JOIN warehouse_locations location ON location.owner_id = batch.owner_id AND location.id = batch.location_id WHERE batch.owner_id = $1 AND location.warehouse_id = $2 AND batch.qty_on_hand > 0)",
    )
    .bind(ctx.owner_id)
    .bind(warehouse_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if has_stock {
        return Err(MasterDataError::LocationHasStock);
    }

    let zone_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM warehouse_zones WHERE owner_id = $1 AND warehouse_id = $2 AND status <> 'disabled' ORDER BY id",
    )
    .bind(ctx.owner_id)
    .bind(warehouse_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for zone_id in zone_ids {
        let before = load_master_data_before(tx, ctx.owner_id, zone_id, "warehouse_zone").await?;
        sqlx::query(
            "UPDATE warehouse_zones SET status = 'disabled', updated_at = $3, version = version + 1 WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(zone_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let after = load_master_data_before(tx, ctx.owner_id, zone_id, "warehouse_zone").await?;
        append_master_data_update_audit(
            tx,
            ctx,
            "cascade_disable_warehouse",
            "warehouse_zone",
            zone_id,
            before,
            &after,
            now,
        )
        .await?;
    }

    let location_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM warehouse_locations WHERE owner_id = $1 AND warehouse_id = $2 AND status <> 'disabled' ORDER BY id",
    )
    .bind(ctx.owner_id)
    .bind(warehouse_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for location_id in location_ids {
        let before = load_master_data_before(tx, ctx.owner_id, location_id, "location").await?;
        sqlx::query(
            "UPDATE warehouse_locations SET status = 'disabled', updated_at = $3, version = version + 1 WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(location_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let after = load_master_data_before(tx, ctx.owner_id, location_id, "location").await?;
        append_master_data_update_audit(
            tx,
            ctx,
            "cascade_disable_warehouse",
            "location",
            location_id,
            before,
            &after,
            now,
        )
        .await?;
    }
    Ok(())
}

fn request_hash(value: &Value) -> Result<String, MasterDataError> {
    idempotency::request_hash(value).map_err(Into::into)
}

impl From<ProductRow> for Product {
    fn from(row: ProductRow) -> Self {
        let mut attrs = row.attrs;
        if let Some(object) = attrs.as_object_mut() {
            object.insert(
                "storage_condition".to_string(),
                json!(row.storage_condition),
            );
            object.insert("source".to_string(), json!(row.source));
        }
        Self {
            id: row.id,
            owner_id: row.owner_id,
            product_code: row.product_code,
            product_name: row.product_name,
            approval_no: row.approval_no,
            spec: row.specification,
            dosage_form: row.dosage_form,
            manufacturer: row.manufacturer,
            udi_code: row.udi_code,
            electronic_regulatory_code: row.electronic_regulatory_code,
            length_mm: row.length_mm,
            width_mm: row.width_mm,
            height_mm: row.height_mm,
            volume_cm3: row.volume_cm3,
            weight_g: row.weight_g,
            packaging_levels: Vec::new(),
            mapping_traces: Vec::new(),
            special_drug_category_code: row.special_drug_category,
            status: row.status,
            attrs,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ProductPackagingLevelRow> for ProductPackagingLevel {
    fn from(row: ProductPackagingLevelRow) -> Self {
        Self {
            id: row.id,
            unit_code: row.unit_code,
            unit_name: row.unit_name,
            ratio_to_base: row.ratio_to_base,
            is_base: row.is_base,
            is_default: row.is_default,
            sort_order: row.sort_order,
        }
    }
}

impl From<ProductMappingTraceRow> for ProductMappingTrace {
    fn from(row: ProductMappingTraceRow) -> Self {
        Self {
            id: row.id,
            field_name: row.field_name,
            rule_id: row.rule_id,
            source_system: row.source_system,
            source_value: row.source_value,
            target_value: row.target_value,
            created_at: row.created_at,
        }
    }
}

async fn insert_product_packaging_levels(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    product_id: Uuid,
    levels: &[ProductPackagingLevelInput],
    now: DateTime<Utc>,
) -> Result<Vec<ProductPackagingLevel>, MasterDataError> {
    let mut inserted = Vec::with_capacity(levels.len());
    for level in levels {
        let row = sqlx::query_as::<_, ProductPackagingLevelRow>(
            r#"
            INSERT INTO product_packaging_levels (
                id, owner_id, product_id, unit_code, unit_name, ratio_to_base,
                is_base, is_default, sort_order, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
            RETURNING id, product_id, unit_code, unit_name, ratio_to_base,
                      is_base, is_default, sort_order
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(product_id)
        .bind(level.unit_code.trim())
        .bind(level.unit_name.trim())
        .bind(level.ratio_to_base)
        .bind(level.is_base)
        .bind(level.is_default)
        .bind(level.sort_order)
        .bind(now)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        inserted.push(ProductPackagingLevel::from(row));
    }
    inserted.sort_by_key(|level| level.sort_order);
    Ok(inserted)
}

async fn load_product_packaging_levels(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
) -> Result<Vec<ProductPackagingLevel>, MasterDataError> {
    sqlx::query_as::<_, ProductPackagingLevelRow>(
        r#"
        SELECT id, product_id, unit_code, unit_name, ratio_to_base,
               is_base, is_default, sort_order
          FROM product_packaging_levels
         WHERE owner_id = $1 AND product_id = $2
         ORDER BY sort_order
        "#,
    )
    .bind(owner_id)
    .bind(product_id)
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(ProductPackagingLevel::from).collect())
    .map_err(map_db_error)
}

async fn load_product_packaging_levels_by_owner(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<HashMap<Uuid, Vec<ProductPackagingLevel>>, MasterDataError> {
    let rows = sqlx::query_as::<_, ProductPackagingLevelRow>(
        r#"
        SELECT id, product_id, unit_code, unit_name, ratio_to_base,
               is_base, is_default, sort_order
          FROM product_packaging_levels
         WHERE owner_id = $1
         ORDER BY product_id, sort_order
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let mut grouped = HashMap::new();
    for row in rows {
        grouped
            .entry(row.product_id)
            .or_insert_with(Vec::new)
            .push(ProductPackagingLevel::from(row));
    }
    Ok(grouped)
}

async fn insert_product_mapping_traces(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    product_id: Uuid,
    traces: &[ProductMappingTraceInput],
    now: DateTime<Utc>,
) -> Result<Vec<ProductMappingTrace>, MasterDataError> {
    let mut inserted = Vec::with_capacity(traces.len());
    for trace in traces {
        let row = sqlx::query_as::<_, ProductMappingTraceRow>(
            r#"
            INSERT INTO product_mapping_traces (
                id, owner_id, product_id, field_name, rule_id,
                source_system, source_value, target_value, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, product_id, field_name, rule_id, source_system,
                      source_value, target_value, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(product_id)
        .bind(trace.field_name.trim())
        .bind(trace.rule_id)
        .bind(trace.source_system.trim())
        .bind(trace.source_value.trim())
        .bind(trace.target_value.as_deref().map(str::trim))
        .bind(now)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        inserted.push(ProductMappingTrace::from(row));
    }
    Ok(inserted)
}

async fn load_product_mapping_traces(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
) -> Result<Vec<ProductMappingTrace>, MasterDataError> {
    sqlx::query_as::<_, ProductMappingTraceRow>(
        r#"
        SELECT id, product_id, field_name, rule_id, source_system,
               source_value, target_value, created_at
          FROM product_mapping_traces
         WHERE owner_id = $1 AND product_id = $2
         ORDER BY created_at, id
        "#,
    )
    .bind(owner_id)
    .bind(product_id)
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(ProductMappingTrace::from).collect())
    .map_err(map_db_error)
}

async fn load_product_mapping_traces_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    product_id: Uuid,
) -> Result<Vec<ProductMappingTrace>, MasterDataError> {
    sqlx::query_as::<_, ProductMappingTraceRow>(
        r#"
        SELECT id, product_id, field_name, rule_id, source_system,
               source_value, target_value, created_at
          FROM product_mapping_traces
         WHERE owner_id = $1 AND product_id = $2
         ORDER BY created_at, id
        "#,
    )
    .bind(owner_id)
    .bind(product_id)
    .fetch_all(&mut **tx)
    .await
    .map(|rows| rows.into_iter().map(ProductMappingTrace::from).collect())
    .map_err(map_db_error)
}

async fn load_product_mapping_traces_by_owner(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<HashMap<Uuid, Vec<ProductMappingTrace>>, MasterDataError> {
    let rows = sqlx::query_as::<_, ProductMappingTraceRow>(
        r#"
        SELECT id, product_id, field_name, rule_id, source_system,
               source_value, target_value, created_at
          FROM product_mapping_traces
         WHERE owner_id = $1
         ORDER BY product_id, created_at, id
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let mut grouped = HashMap::new();
    for row in rows {
        grouped
            .entry(row.product_id)
            .or_insert_with(Vec::new)
            .push(ProductMappingTrace::from(row));
    }
    Ok(grouped)
}

impl From<SupplierRow> for Supplier {
    fn from(row: SupplierRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            supplier_code: row.supplier_code,
            supplier_name: row.supplier_name,
            license_no: Some(row.uscc),
            contact_name: row.contact_name,
            source: row.source,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<CustomerRow> for Customer {
    fn from(row: CustomerRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            customer_code: row.customer_code,
            customer_name: row.customer_name,
            license_no: row.license_no,
            source: row.source,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<WarehouseRow> for Warehouse {
    fn from(row: WarehouseRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            warehouse_code: row.warehouse_code,
            warehouse_name: row.warehouse_name,
            warehouse_type: row.warehouse_type,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<LocationRow> for Location {
    fn from(row: LocationRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            warehouse_id: row.warehouse_id,
            zone_id: row.zone_id,
            location_code: row.location_code,
            row_no: row.row_no,
            column_no: row.column_no,
            layer_no: row.layer_no,
            max_volume_cm3: row.max_volume_cm3,
            used_volume_cm3: row.used_volume_cm3,
            max_sku_count: row.max_sku_count,
            location_type: row.location_type,
            bound_owner_id: row.bound_owner_id,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn map_db_error(error: sqlx::Error) -> MasterDataError {
    MasterDataError::Database(error.to_string())
}

fn string_attr(attrs: &Value, key: &str) -> Option<String> {
    attrs
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn non_empty_or(value: Option<String>, default_value: &str) -> String {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_value.to_string())
}

fn map_location_write_error(error: sqlx::Error) -> MasterDataError {
    if let sqlx::Error::Database(db_error) = &error {
        if db_error.code().as_deref() == Some("23505") {
            return MasterDataError::DuplicateLocationCode("warehouse_location".to_string());
        }
    }
    map_db_error(error)
}

fn map_catalog_write_error(error: sqlx::Error, code: &str) -> MasterDataError {
    if let sqlx::Error::Database(db_error) = &error {
        if db_error.code().as_deref() == Some("23505") {
            if db_error.constraint() == Some("products_owner_udi_uidx") {
                return MasterDataError::DuplicateProductUdi;
            }
            return MasterDataError::DuplicateCode(code.to_string());
        }
    }
    map_db_error(error)
}
