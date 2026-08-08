#[derive(Clone, Debug)]
pub struct ErpMasterApplyOutcome {
    pub id: Uuid,
    pub ignored_old_version: bool,
}

#[derive(Clone, Debug)]
pub struct ErpPartnerSnapshot {
    pub entity_id: i64,
    pub source_version: i64,
    pub code: Option<String>,
    pub name: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub payload: Value,
}

impl PgMasterDataReadRepository {
    pub async fn apply_erp_product_snapshot(
        &self,
        ctx: &AuthContext,
        entity_id: i64,
        source_version: i64,
        op_type: &str,
        request: Option<CreateProductRequest>,
        mapping_traces: Vec<ProductMappingTraceInput>,
        status: &str,
        now: DateTime<Utc>,
    ) -> Result<ErpMasterApplyOutcome, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let existing: Option<(Uuid, Option<i64>)> = sqlx::query_as(
            "SELECT id, erp_source_version FROM products WHERE owner_id=$1 AND erp_goods_id=$2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(entity_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some((id, Some(version))) = existing {
            if version >= source_version {
                tx.commit().await.map_err(map_db_error)?;
                return Ok(ErpMasterApplyOutcome {
                    id,
                    ignored_old_version: true,
                });
            }
        }
        if op_type == "D" {
            let Some((id, _)) = existing else {
                tx.commit().await.map_err(map_db_error)?;
                return Ok(ErpMasterApplyOutcome {
                    id: Uuid::nil(),
                    ignored_old_version: true,
                });
            };
            sqlx::query(
                "UPDATE products SET status='disabled', erp_source_version=$3, updated_at=$4, version=version+1 WHERE owner_id=$1 AND id=$2",
            )
            .bind(ctx.owner_id)
            .bind(id)
            .bind(source_version)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            append_erp_master_audit(&mut tx, ctx, "product", id, entity_id, source_version, op_type, now).await?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok(ErpMasterApplyOutcome { id, ignored_old_version: false });
        }

        let req = request.ok_or(MasterDataError::InvalidProductFields)?;
        validate_create_product_fields(&req)?;
        if !matches!(status, "active" | "pending_mapping") {
            return Err(MasterDataError::InvalidProductFields);
        }
        if status == "active" {
            validate_product_packaging_levels(&req.packaging_levels)?;
        } else if !req.packaging_levels.is_empty() {
            return Err(MasterDataError::InvalidProductPackaging);
        }
        validate_product_mapping_traces(&mapping_traces)?;
        let volume_cm3 = normalize_product_volume(
            req.length_mm,
            req.width_mm,
            req.height_mm,
            req.volume_cm3,
            req.weight_g,
        )?;
        let attrs = product_attrs_with_default_source(req.attrs.clone(), "erp_interface");
        let storage_condition = string_attr(&attrs, "storage_condition");
        if status == "active" {
            validate_product_storage_condition(&attrs)?;
        }
        let special_drug_category = req
            .special_drug_category_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if status == "active" && special_drug_category.is_none() {
            return Err(MasterDataError::InvalidSpecialDrugCategory);
        }
        if let Some(category) = special_drug_category {
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
        let source = string_attr(&attrs, "source").unwrap_or_else(|| "erp_interface".to_string());
        let id = existing.map_or_else(Uuid::new_v4, |(id, _)| id);
        let query = if existing.is_some() {
            r#"
            UPDATE products SET
                erp_goods_id=$23, product_code=$3, product_name=$4, specification=$5, dosage_form=$6,
                storage_condition=$7, special_drug_category=$8, approval_no=$9,
                manufacturer=$10, udi_code=$11, electronic_regulatory_code=$12,
                length_mm=$13, width_mm=$14, height_mm=$15, volume_cm3=$16,
                weight_g=$17, source=$18, attrs=$19, status=$20,
                erp_source_version=$21, updated_at=$22, version=version+1
            WHERE owner_id=$1 AND id=$2 RETURNING id
            "#
        } else {
            r#"
            INSERT INTO products (
                owner_id,id,erp_goods_id,erp_source_version,product_code,product_name,
                specification,dosage_form,storage_condition,special_drug_category,
                approval_no,manufacturer,udi_code,electronic_regulatory_code,
                length_mm,width_mm,height_mm,volume_cm3,weight_g,source,attrs,status,
                created_at,updated_at
            ) VALUES ($1,$2,$23,$21,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$22,$22)
            RETURNING id
            "#
        };
        sqlx::query_scalar::<_, Uuid>(query)
            .bind(ctx.owner_id)
            .bind(id)
            .bind(req.product_code.trim())
            .bind(req.product_name.trim())
            .bind(req.spec.trim())
            .bind(&req.dosage_form)
            .bind(storage_condition.as_deref())
            .bind(special_drug_category)
            .bind(&req.approval_no)
            .bind(&req.manufacturer)
            .bind(req.udi_code.as_deref().map(str::trim))
            .bind(&req.electronic_regulatory_code)
            .bind(req.length_mm)
            .bind(req.width_mm)
            .bind(req.height_mm)
            .bind(volume_cm3)
            .bind(req.weight_g)
            .bind(&source)
            .bind(&attrs)
            .bind(status)
            .bind(source_version)
            .bind(now)
            .bind(entity_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| map_catalog_write_error(error, req.product_code.trim()))?;
        sqlx::query("DELETE FROM product_packaging_levels WHERE owner_id=$1 AND product_id=$2")
            .bind(ctx.owner_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        insert_product_packaging_levels(&mut tx, ctx.owner_id, id, &req.packaging_levels, now)
            .await?;
        insert_product_mapping_traces(&mut tx, ctx.owner_id, id, &mapping_traces, now)
            .await?;
        append_erp_master_audit(&mut tx, ctx, "product", id, entity_id, source_version, op_type, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(ErpMasterApplyOutcome { id, ignored_old_version: false })
    }

    pub async fn apply_erp_customer_snapshot(
        &self,
        ctx: &AuthContext,
        op_type: &str,
        snapshot: ErpPartnerSnapshot,
        now: DateTime<Utc>,
    ) -> Result<ErpMasterApplyOutcome, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let existing: Option<(Uuid, Option<i64>)> = sqlx::query_as(
            "SELECT id, erp_source_version FROM customers WHERE owner_id=$1 AND erp_client_id=$2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(snapshot.entity_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some((id, Some(version))) = existing {
            if version >= snapshot.source_version {
                tx.commit().await.map_err(map_db_error)?;
                return Ok(ErpMasterApplyOutcome {
                    id,
                    ignored_old_version: true,
                });
            }
        }
        let id = existing.map_or_else(Uuid::new_v4, |(id, _)| id);
        if op_type == "D" {
            if existing.is_some() {
                sqlx::query("UPDATE customers SET status='disabled', erp_source_version=$3, updated_at=$4, version=version+1 WHERE owner_id=$1 AND id=$2")
                    .bind(ctx.owner_id).bind(id).bind(snapshot.source_version).bind(now)
                    .execute(&mut *tx).await.map_err(map_db_error)?;
                append_erp_master_audit(&mut tx, ctx, "customer", id, snapshot.entity_id, snapshot.source_version, op_type, now).await?;
            }
            tx.commit().await.map_err(map_db_error)?;
            return Ok(ErpMasterApplyOutcome { id, ignored_old_version: existing.is_none() });
        }
        let code = snapshot.code.as_deref().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| MasterDataError::DuplicateCode("customer_code".into()))?;
        let name = snapshot.name.as_deref().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| MasterDataError::DuplicateCode("customer_name".into()))?;
        if existing.is_some() {
            sqlx::query("UPDATE customers SET customer_code=$3, customer_name=$4, contact_name=$5, contact_phone=$6, erp_payload=$7, status='active', erp_source_version=$8, updated_at=$9, version=version+1 WHERE owner_id=$1 AND id=$2")
                .bind(ctx.owner_id).bind(id).bind(code).bind(name).bind(&snapshot.contact_name).bind(&snapshot.contact_phone).bind(&snapshot.payload).bind(snapshot.source_version).bind(now)
                .execute(&mut *tx).await.map_err(|error| map_catalog_write_error(error, code))?;
        } else {
            sqlx::query("INSERT INTO customers (id,owner_id,erp_client_id,erp_source_version,customer_code,customer_name,customer_type,contact_name,contact_phone,erp_payload,source,status,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,'customer',$7,$8,$9,'erp_interface','active',$10,$10)")
                .bind(id).bind(ctx.owner_id).bind(snapshot.entity_id).bind(snapshot.source_version).bind(code).bind(name).bind(&snapshot.contact_name).bind(&snapshot.contact_phone).bind(&snapshot.payload).bind(now)
                .execute(&mut *tx).await.map_err(|error| map_catalog_write_error(error, code))?;
        }
        append_erp_master_audit(&mut tx, ctx, "customer", id, snapshot.entity_id, snapshot.source_version, op_type, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(ErpMasterApplyOutcome { id, ignored_old_version: false })
    }

    pub async fn apply_erp_supplier_snapshot(
        &self,
        ctx: &AuthContext,
        op_type: &str,
        snapshot: ErpPartnerSnapshot,
        now: DateTime<Utc>,
    ) -> Result<ErpMasterApplyOutcome, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let existing: Option<(Uuid, Option<i64>)> = sqlx::query_as(
            "SELECT id, erp_source_version FROM suppliers WHERE owner_id=$1 AND erp_supplier_id=$2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(snapshot.entity_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some((id, Some(version))) = existing {
            if version >= snapshot.source_version {
                tx.commit().await.map_err(map_db_error)?;
                return Ok(ErpMasterApplyOutcome {
                    id,
                    ignored_old_version: true,
                });
            }
        }
        let id = existing.map_or_else(Uuid::new_v4, |(id, _)| id);
        if op_type == "D" {
            if existing.is_some() {
                sqlx::query("UPDATE suppliers SET status='disabled', erp_source_version=$3, updated_at=$4, version=version+1 WHERE owner_id=$1 AND id=$2")
                    .bind(ctx.owner_id).bind(id).bind(snapshot.source_version).bind(now)
                    .execute(&mut *tx).await.map_err(map_db_error)?;
                append_erp_master_audit(&mut tx, ctx, "supplier", id, snapshot.entity_id, snapshot.source_version, op_type, now).await?;
            }
            tx.commit().await.map_err(map_db_error)?;
            return Ok(ErpMasterApplyOutcome { id, ignored_old_version: existing.is_none() });
        }
        let code = snapshot.code.as_deref().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| MasterDataError::DuplicateCode("supplier_code".into()))?;
        let name = snapshot.name.as_deref().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| MasterDataError::DuplicateCode("supplier_name".into()))?;
        if existing.is_some() {
            sqlx::query("UPDATE suppliers SET supplier_code=$3, supplier_name=$4, uscc=$3, contact_name=$5, contact_phone=$6, erp_payload=$7, status='active', erp_source_version=$8, updated_at=$9, version=version+1 WHERE owner_id=$1 AND id=$2")
                .bind(ctx.owner_id).bind(id).bind(code).bind(name).bind(&snapshot.contact_name).bind(&snapshot.contact_phone).bind(&snapshot.payload).bind(snapshot.source_version).bind(now)
                .execute(&mut *tx).await.map_err(|error| map_catalog_write_error(error, code))?;
        } else {
            sqlx::query("INSERT INTO suppliers (id,owner_id,erp_supplier_id,erp_source_version,supplier_code,supplier_name,uscc,contact_name,contact_phone,erp_payload,source,status,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$5,$7,$8,$9,'erp_interface','active',$10,$10)")
                .bind(id).bind(ctx.owner_id).bind(snapshot.entity_id).bind(snapshot.source_version).bind(code).bind(name).bind(&snapshot.contact_name).bind(&snapshot.contact_phone).bind(&snapshot.payload).bind(now)
                .execute(&mut *tx).await.map_err(|error| map_catalog_write_error(error, code))?;
        }
        append_erp_master_audit(&mut tx, ctx, "supplier", id, snapshot.entity_id, snapshot.source_version, op_type, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(ErpMasterApplyOutcome { id, ignored_old_version: false })
    }
}

async fn append_erp_master_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    resource_type: &str,
    id: Uuid,
    entity_id: i64,
    source_version: i64,
    op_type: &str,
    now: DateTime<Utc>,
) -> Result<(), MasterDataError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        "apply_erp_master_snapshot",
        "H8",
        resource_type,
        id.to_string(),
        Some(AuditDiff::compute(
            Value::Null,
            json!({"erp_entity_id": entity_id, "source_version": source_version, "op_type": op_type}),
        )),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| MasterDataError::Audit(format!("{error:?}")))?;
    Ok(())
}
