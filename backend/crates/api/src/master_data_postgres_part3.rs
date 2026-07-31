// @governance: skip-page-size - M1 仓储拆分文件共享幂等事务边界，当前仅收敛存储实现，不拆业务行为链。
impl PgMasterDataReadRepository {
    pub async fn get_product(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<Product, MasterDataError> {
        let row = sqlx::query_as::<_, ProductRow>(
            "SELECT id, owner_id, product_code, product_name, specification, dosage_form, storage_condition, special_drug_category, approval_no, manufacturer, udi_code, electronic_regulatory_code, length_mm, width_mm, height_mm, volume_cm3, weight_g, source, attrs, status, created_at, updated_at FROM products WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)?;
        let mut product = Product::from(row);
        product.packaging_levels =
            load_product_packaging_levels(&self.pool, ctx.owner_id, product.id).await?;
        product.mapping_traces =
            load_product_mapping_traces(&self.pool, ctx.owner_id, product.id).await?;
        Ok(product)
    }

    pub async fn update_product(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateProductRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Product, MasterDataError> {
        self.update_product_with_mapping_traces(ctx, id, req, Vec::new(), now, idempotency_key)
            .await
    }

    pub async fn update_product_with_mapping_traces(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateProductRequest,
        mapping_traces: Vec<ProductMappingTraceInput>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Product, MasterDataError> {
        validate_update_product_fields(&req)?;
        validate_product_mapping_traces(&mapping_traces)?;
        if let Some(attrs) = req.attrs.as_ref() {
            if attrs.get("storage_condition").is_some() {
                validate_product_storage_condition(attrs)?;
            }
        }
        if let Some(levels) = req.packaging_levels.as_ref() {
            validate_product_packaging_levels(levels)?;
        }
        let physical = normalize_product_physical_patch(&req)?;
        let packaging_levels = req.packaging_levels.clone();
        let request_hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/products/{id}"),
            "request": &req,
            "mapping_traces": &mapping_traces,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Product>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/master-data/products/{id}"),
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let (current_category, current_status): (String, String) = sqlx::query_as(
            "SELECT special_drug_category, status FROM products WHERE owner_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)?;
        validate_product_update_transition(Some(&current_category), &current_status, &req)?;
        let before = load_master_data_before(&mut tx, ctx.owner_id, id, "product").await?;
        let storage_condition = req
            .attrs
            .as_ref()
            .and_then(|attrs| string_attr(attrs, "storage_condition"));
        let source = req
            .attrs
            .as_ref()
            .and_then(|attrs| string_attr(attrs, "source"));
        if let Some(category) = req.special_drug_category_code.as_deref() {
            ensure_enabled_dictionary_item(
                &mut tx,
                ctx.owner_id,
                SPECIAL_DRUG_CATEGORY_DICT,
                category,
                now,
            )
            .await
            .map_err(|_| MasterDataError::InvalidSpecialDrugCategory)?;
        }
        let approval_no_present = req.approval_no.is_some();
        let approval_no = req.approval_no.clone().flatten();
        let dosage_form_present = req.dosage_form.is_some();
        let dosage_form = req.dosage_form.clone().flatten();
        let manufacturer_present = req.manufacturer.is_some();
        let manufacturer = req.manufacturer.clone().flatten();
        let udi_code_present = req.udi_code.is_some();
        let udi_code = req
            .udi_code
            .clone()
            .flatten()
            .map(|value| value.trim().to_string());
        let electronic_regulatory_code_present = req.electronic_regulatory_code.is_some();
        let electronic_regulatory_code = req.electronic_regulatory_code.clone().flatten();
        let row = sqlx::query_as::<_, ProductRow>(
            r#"UPDATE products SET
                product_name = COALESCE($3, product_name),
                approval_no = CASE WHEN $4 THEN $5 ELSE approval_no END,
                specification = COALESCE($6, specification),
                dosage_form = CASE WHEN $7 THEN $8 ELSE dosage_form END,
                manufacturer = CASE WHEN $9 THEN $10 ELSE manufacturer END,
                special_drug_category = COALESCE($11, special_drug_category),
                status = COALESCE($12, status),
                storage_condition = COALESCE($13, storage_condition),
                source = COALESCE($14, source),
                attrs = CASE WHEN $15 IS NULL THEN attrs ELSE attrs || $15 END,
                udi_code = CASE WHEN $16 THEN $17 ELSE udi_code END,
                electronic_regulatory_code = CASE WHEN $18 THEN $19 ELSE electronic_regulatory_code END,
                length_mm = CASE WHEN $20 THEN $21 ELSE length_mm END,
                width_mm = CASE WHEN $22 THEN $23 ELSE width_mm END,
                height_mm = CASE WHEN $24 THEN $25 ELSE height_mm END,
                volume_cm3 = CASE WHEN $26 THEN $27 ELSE volume_cm3 END,
                weight_g = CASE WHEN $28 THEN $29 ELSE weight_g END,
                updated_at = $30,
                version = version + 1
              WHERE owner_id = $1 AND id = $2
              RETURNING id, owner_id, product_code, product_name, specification,
                        dosage_form, storage_condition, special_drug_category, approval_no,
                        manufacturer, udi_code, electronic_regulatory_code, length_mm,
                        width_mm, height_mm, volume_cm3, weight_g, source, attrs, status,
                        created_at, updated_at"#,
        )
        .bind(ctx.owner_id).bind(id).bind(req.product_name)
        .bind(approval_no_present).bind(approval_no).bind(req.spec)
        .bind(dosage_form_present).bind(dosage_form)
        .bind(manufacturer_present).bind(manufacturer)
        .bind(req.special_drug_category_code).bind(req.status)
        .bind(storage_condition).bind(source).bind(req.attrs)
        .bind(udi_code_present).bind(udi_code)
        .bind(electronic_regulatory_code_present).bind(electronic_regulatory_code)
        .bind(physical.length_mm.is_some()).bind(physical.length_mm.flatten())
        .bind(physical.width_mm.is_some()).bind(physical.width_mm.flatten())
        .bind(physical.height_mm.is_some()).bind(physical.height_mm.flatten())
        .bind(physical.volume_cm3.is_some()).bind(physical.volume_cm3.flatten())
        .bind(physical.weight_g.is_some()).bind(physical.weight_g.flatten())
        .bind(now)
        .fetch_optional(&mut *tx).await.map_err(|error| map_catalog_write_error(error, "product"))?
        .ok_or(MasterDataError::NotFound)?;
        let mut value = Product::from(row);
        if let Some(levels) = packaging_levels {
            sqlx::query(
                "DELETE FROM product_packaging_levels WHERE owner_id = $1 AND product_id = $2",
            )
            .bind(ctx.owner_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            value.packaging_levels =
                insert_product_packaging_levels(&mut tx, ctx.owner_id, id, &levels, now).await?;
        } else {
            value.packaging_levels =
                load_product_packaging_levels(&self.pool, ctx.owner_id, id).await?;
        }
        if !mapping_traces.is_empty() {
            insert_product_mapping_traces(&mut tx, ctx.owner_id, id, &mapping_traces, now).await?;
        }
        value.mapping_traces = load_product_mapping_traces_in_tx(&mut tx, ctx.owner_id, id).await?;
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_product",
            "product",
            id,
            before,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &value,
            now,
            "PATCH",
            &format!("/api/v1/master-data/products/{id}"),
            "product",
            &id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn update_supplier(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateSupplierRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Supplier, MasterDataError> {
        let mut req = req;
        req.license_no = normalize_supplier_uscc(req.license_no)?;
        let request_hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/suppliers/{id}"),
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Supplier>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/master-data/suppliers/{id}"),
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = load_master_data_before(&mut tx, ctx.owner_id, id, "supplier").await?;
        let row = sqlx::query_as::<_, SupplierRow>(
            r#"UPDATE suppliers SET supplier_name = COALESCE($3, supplier_name), uscc = COALESCE($4, uscc), contact_name = COALESCE($5, contact_name), status = COALESCE($6, status), updated_at = $7, version = version + 1 WHERE owner_id = $1 AND id = $2 RETURNING id, owner_id, supplier_code, supplier_name, uscc, contact_name, source, status, created_at, updated_at"#,
        ).bind(ctx.owner_id).bind(id).bind(req.supplier_name).bind(req.license_no).bind(req.contact_name).bind(req.status).bind(now)
        .fetch_optional(&mut *tx).await.map_err(map_db_error)?.ok_or(MasterDataError::NotFound)?;
        let value = Supplier::from(row);
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_supplier",
            "supplier",
            id,
            before,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &value,
            now,
            "PATCH",
            &format!("/api/v1/master-data/suppliers/{id}"),
            "supplier",
            &id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn update_customer(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateCustomerRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Customer, MasterDataError> {
        let request_hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/customers/{id}"),
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Customer>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/master-data/customers/{id}"),
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = load_master_data_before(&mut tx, ctx.owner_id, id, "customer").await?;
        let row = sqlx::query_as::<_, CustomerRow>(
            r#"UPDATE customers SET customer_name = COALESCE($3, customer_name), license_no = COALESCE($4, license_no), status = COALESCE($5, status), updated_at = $6, version = version + 1 WHERE owner_id = $1 AND id = $2 RETURNING id, owner_id, customer_code, customer_name, license_no, source, status, created_at, updated_at"#,
        ).bind(ctx.owner_id).bind(id).bind(req.customer_name).bind(req.license_no).bind(req.status).bind(now)
        .fetch_optional(&mut *tx).await.map_err(map_db_error)?.ok_or(MasterDataError::NotFound)?;
        let value = Customer::from(row);
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_customer",
            "customer",
            id,
            before,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &value,
            now,
            "PATCH",
            &format!("/api/v1/master-data/customers/{id}"),
            "customer",
            &id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn create_warehouse(
        &self,
        ctx: &AuthContext,
        req: CreateWarehouseRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Warehouse, MasterDataError> {
        validate_warehouse_type(&req.warehouse_type)?;
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/warehouses",
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Warehouse>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/master-data/warehouses",
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let row = sqlx::query_as::<_, WarehouseRow>(
            r#"INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, 'active', $6, $6) RETURNING id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at"#,
        ).bind(Uuid::new_v4()).bind(ctx.owner_id).bind(&req.warehouse_code).bind(&req.warehouse_name).bind(&req.warehouse_type).bind(now)
        .fetch_one(&mut *tx).await.map_err(|error| map_catalog_write_error(error, &req.warehouse_code))?;
        let value = Warehouse::from(row);
        append_master_data_audit(
            &mut tx,
            ctx,
            "create_warehouse",
            "warehouse",
            value.id,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &value,
            now,
            "POST",
            "/api/v1/master-data/warehouses",
            "warehouse",
            &value.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn update_warehouse(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateWarehouseRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Warehouse, MasterDataError> {
        let request_hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/warehouses/{id}"),
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Warehouse>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/master-data/warehouses/{id}"),
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = load_master_data_before(&mut tx, ctx.owner_id, id, "warehouse").await?;
        let disable = req.status.as_deref() == Some("disabled");
        if let Some(warehouse_type) = req.warehouse_type.as_deref() {
            validate_warehouse_type(warehouse_type)?;
        }
        let row = sqlx::query_as::<_, WarehouseRow>(
            r#"UPDATE warehouses SET warehouse_name = COALESCE($3, warehouse_name), warehouse_type = COALESCE($4, warehouse_type), status = COALESCE($5, status), updated_at = $6, version = version + 1 WHERE owner_id = $1 AND id = $2 RETURNING id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at"#,
        ).bind(ctx.owner_id).bind(id).bind(req.warehouse_name).bind(req.warehouse_type).bind(req.status).bind(now)
        .fetch_optional(&mut *tx).await.map_err(map_db_error)?.ok_or(MasterDataError::NotFound)?;
        let value = Warehouse::from(row);
        if disable {
            disable_warehouse_children(&mut tx, ctx, id, now).await?;
        }
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_warehouse",
            "warehouse",
            id,
            before,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &value,
            now,
            "PATCH",
            &format!("/api/v1/master-data/warehouses/{id}"),
            "warehouse",
            &id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn create_location(
        &self,
        ctx: &AuthContext,
        req: CreateLocationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Location, MasterDataError> {
        validate_location_code(&req.location_code, req.row_no, req.column_no, req.layer_no)?;
        validate_location_capacity(req.max_volume_cm3, 0)?;
        let hash = request_hash(&json!({
            "path": "/api/v1/master-data/locations",
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency::<Location>(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &hash,
                "POST",
                "/api/v1/master-data/locations",
                now,
            )
                .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        ensure_warehouse_zone_in_owner(&mut tx, ctx.owner_id, req.warehouse_id, req.zone_id)
            .await?;
        ensure_bound_owner_exists(&mut tx, req.bound_owner_id).await?;
        ensure_enabled_dictionary_item(
            &mut tx,
            ctx.owner_id,
            LOCATION_TYPE_DICT,
            &req.location_type,
            now,
        )
        .await?;
        let row = sqlx::query_as::<_, LocationRow>(
            r#"INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, bound_owner_id, status, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,0,$10,$11,$12,'available',$13,$13) RETURNING id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, bound_owner_id, status, created_at, updated_at"#,
        ).bind(Uuid::new_v4()).bind(ctx.owner_id).bind(req.warehouse_id).bind(req.zone_id).bind(&req.location_code)
        .bind(req.row_no).bind(req.column_no).bind(req.layer_no).bind(req.max_volume_cm3).bind(req.max_sku_count).bind(&req.location_type).bind(req.bound_owner_id).bind(now)
        .fetch_one(&mut *tx).await.map_err(|error| map_catalog_write_error(error, &req.location_code))?;
        let value = Location::from(row);
        append_master_data_audit(
            &mut tx,
            ctx,
            "create_location",
            "location",
            value.id,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            &value,
            now,
            "POST",
            "/api/v1/master-data/locations",
            "location",
            &value.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn update_location(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateLocationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Location, MasterDataError> {
        let hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/locations/{id}"),
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency::<Location>(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &hash,
                "PATCH",
                &format!("/api/v1/master-data/locations/{id}"),
                now,
            )
                .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = load_master_data_before(&mut tx, ctx.owner_id, id, "location").await?;
        let current = sqlx::query_as::<_, (String, i32, i32, i32, i64, i64)>(
            "SELECT location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3 FROM warehouse_locations WHERE owner_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)?;
        validate_location_code(
            req.location_code.as_deref().unwrap_or(&current.0),
            req.row_no.unwrap_or(current.1),
            req.column_no.unwrap_or(current.2),
            req.layer_no.unwrap_or(current.3),
        )?;
        validate_location_capacity(
            req.max_volume_cm3.unwrap_or(current.4),
            req.used_volume_cm3.unwrap_or(current.5),
        )?;
        if req.status.as_deref() == Some("disabled") {
            let has_stock: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM inventory_batches WHERE owner_id = $1 AND location_id = $2 AND qty_on_hand > 0)",
            )
            .bind(ctx.owner_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if has_stock {
                return Err(MasterDataError::LocationHasStock);
            }
        }
        if let Some(zone_id) = req.zone_id {
            let warehouse_id = sqlx::query_scalar(
                "SELECT warehouse_id FROM warehouse_locations WHERE owner_id = $1 AND id = $2",
            )
            .bind(ctx.owner_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(MasterDataError::NotFound)?;
            ensure_warehouse_zone_in_owner(&mut tx, ctx.owner_id, warehouse_id, zone_id).await?;
        }
        ensure_bound_owner_exists(&mut tx, req.bound_owner_id).await?;
        if let Some(location_type) = req.location_type.as_deref() {
            ensure_enabled_dictionary_item(
                &mut tx,
                ctx.owner_id,
                LOCATION_TYPE_DICT,
                location_type,
                now,
            )
            .await?;
        }
        let row = sqlx::query_as::<_, LocationRow>(
            r#"UPDATE warehouse_locations SET zone_id=COALESCE($3,zone_id), location_code=COALESCE($4,location_code), row_no=COALESCE($5,row_no), column_no=COALESCE($6,column_no), layer_no=COALESCE($7,layer_no), max_volume_cm3=COALESCE($8,max_volume_cm3), used_volume_cm3=COALESCE($9,used_volume_cm3), max_sku_count=COALESCE($10,max_sku_count), location_type=COALESCE($11,location_type), bound_owner_id=COALESCE($12,bound_owner_id), status=COALESCE($13,status), updated_at=$14, version=version+1 WHERE owner_id=$1 AND id=$2 RETURNING id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, bound_owner_id, status, created_at, updated_at"#,
        ).bind(ctx.owner_id).bind(id).bind(req.zone_id).bind(req.location_code).bind(req.row_no).bind(req.column_no).bind(req.layer_no).bind(req.max_volume_cm3).bind(req.used_volume_cm3).bind(req.max_sku_count).bind(req.location_type).bind(req.bound_owner_id).bind(req.status).bind(now)
        .fetch_optional(&mut *tx).await.map_err(map_db_error)?.ok_or(MasterDataError::NotFound)?;
        let value = Location::from(row);
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_location",
            "location",
            id,
            before,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            &value,
            now,
            "PATCH",
            &format!("/api/v1/master-data/locations/{id}"),
            "location",
            &id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn list_warehouse_zones(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<WarehouseZone>, MasterDataError> {
        let rows = sqlx::query_as::<_, WarehouseZoneRow>(
            "SELECT id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status, created_at, updated_at FROM warehouse_zones WHERE owner_id = $1 ORDER BY updated_at DESC, zone_code",
        ).bind(ctx.owner_id).fetch_all(&self.pool).await.map_err(map_db_error)?;
        Ok(rows.into_iter().map(WarehouseZone::from).collect())
    }

    pub async fn create_warehouse_zone(
        &self,
        ctx: &AuthContext,
        req: CreateWarehouseZoneRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<WarehouseZone, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let hash = request_hash(&json!({
            "path": "/api/v1/master-data/warehouse-zones",
            "request": &req,
        }))?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &hash,
                "POST",
                "/api/v1/master-data/warehouse-zones",
                now,
            )
            .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let warehouse_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM warehouses WHERE owner_id=$1 AND id=$2)",
        )
        .bind(ctx.owner_id)
        .bind(req.warehouse_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !warehouse_exists {
            return Err(MasterDataError::NotFound);
        }
        ensure_enabled_dictionary_item(
            &mut tx,
            ctx.owner_id,
            TEMPERATURE_ZONE_DICT,
            &req.temperature_zone,
            now,
        )
        .await?;
        ensure_enabled_dictionary_item(
            &mut tx,
            ctx.owner_id,
            QUALITY_COLOR_DICT,
            &req.quality_color,
            now,
        )
        .await?;
        let row = sqlx::query_as::<_, WarehouseZoneRow>(
            r#"INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,'active',$8,$8) RETURNING id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status, created_at, updated_at"#,
        ).bind(Uuid::new_v4()).bind(ctx.owner_id).bind(req.warehouse_id).bind(&req.zone_code).bind(&req.zone_name).bind(&req.temperature_zone).bind(&req.quality_color).bind(now)
        .fetch_one(&mut *tx).await.map_err(|error| map_catalog_write_error(error, &req.zone_code))?;
        let value = WarehouseZone::from(row);
        append_master_data_audit(
            &mut tx,
            ctx,
            "create_warehouse_zone",
            "warehouse_zone",
            value.id,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            &value,
            now,
            "POST",
            "/api/v1/master-data/warehouse-zones",
            "warehouse_zone",
            &value.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn update_warehouse_zone(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateWarehouseZoneRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<WarehouseZone, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/warehouse-zones/{id}"),
            "request": &req,
        }))?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &hash,
                "PATCH",
                &format!("/api/v1/master-data/warehouse-zones/{id}"),
                now,
            )
            .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = load_master_data_before(&mut tx, ctx.owner_id, id, "warehouse_zone").await?;
        if let Some(value) = req.temperature_zone.as_deref() {
            ensure_enabled_dictionary_item(
                &mut tx,
                ctx.owner_id,
                TEMPERATURE_ZONE_DICT,
                value,
                now,
            )
            .await?;
        }
        if let Some(value) = req.quality_color.as_deref() {
            ensure_enabled_dictionary_item(&mut tx, ctx.owner_id, QUALITY_COLOR_DICT, value, now)
                .await?;
        }
        let row = sqlx::query_as::<_, WarehouseZoneRow>(
            r#"UPDATE warehouse_zones SET zone_name=COALESCE($3,zone_name), temperature_zone=COALESCE($4,temperature_zone), quality_color=COALESCE($5,quality_color), status=COALESCE($6,status), updated_at=$7, version=version+1 WHERE owner_id=$1 AND id=$2 RETURNING id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status, created_at, updated_at"#,
        ).bind(ctx.owner_id).bind(id).bind(req.zone_name).bind(req.temperature_zone).bind(req.quality_color).bind(req.status).bind(now)
        .fetch_optional(&mut *tx).await.map_err(map_db_error)?.ok_or(MasterDataError::NotFound)?;
        let value = WarehouseZone::from(row);
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_warehouse_zone",
            "warehouse_zone",
            id,
            before,
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            &value,
            now,
            "PATCH",
            &format!("/api/v1/master-data/warehouse-zones/{id}"),
            "warehouse_zone",
            &id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }
}

impl From<WarehouseZoneRow> for WarehouseZone {
    fn from(row: WarehouseZoneRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            warehouse_id: row.warehouse_id,
            zone_code: row.zone_code,
            zone_name: row.zone_name,
            temperature_zone: row.temperature_zone,
            quality_color: row.quality_color,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
