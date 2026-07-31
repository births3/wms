#[derive(sqlx::FromRow)]
struct CustomerAddressRow {
    id: Uuid,
    owner_id: Uuid,
    customer_id: Uuid,
    province: String,
    city: String,
    district: String,
    detail_address: String,
    contact_name: String,
    contact_phone: String,
    is_default: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgMasterDataReadRepository {
    pub async fn list_customer_addresses(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
    ) -> Result<Vec<CustomerAddress>, MasterDataError> {
        ensure_customer_exists(&self.pool, ctx.owner_id, customer_id).await?;
        let rows = sqlx::query_as::<_, CustomerAddressRow>(
            "SELECT id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone, is_default, created_at, updated_at FROM customer_addresses WHERE owner_id=$1 AND customer_id=$2 ORDER BY is_default DESC, updated_at DESC, id",
        )
        .bind(ctx.owner_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(CustomerAddress::from).collect())
    }

    pub async fn create_customer_address(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
        req: CreateCustomerAddressRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<CustomerAddress, MasterDataError> {
        validate_customer_address(&req)?;
        let request_hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/customers/{customer_id}/addresses"),
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        ensure_customer_exists_tx(&mut tx, ctx.owner_id, customer_id).await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<CustomerAddress>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &format!("/api/v1/master-data/customers/{customer_id}/addresses"),
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let previous_default_ids =
            unset_default_customer_address(&mut tx, ctx.owner_id, customer_id, req.is_default, now)
                .await?;
        let row = sqlx::query_as::<_, CustomerAddressRow>(
            "INSERT INTO customer_addresses (id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone, is_default, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$11) RETURNING id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone, is_default, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(customer_id)
        .bind(&req.province)
        .bind(&req.city)
        .bind(&req.district)
        .bind(&req.detail_address)
        .bind(&req.contact_name)
        .bind(&req.contact_phone)
        .bind(req.is_default)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let address = CustomerAddress::from(row);
        audit_customer_address_default_resets(&mut tx, ctx, previous_default_ids, now).await?;
        append_master_data_audit(
            &mut tx,
            ctx,
            "create_customer_address",
            "customer_address",
            address.id,
            &address,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &address,
            now,
            "POST",
            &format!("/api/v1/master-data/customers/{customer_id}/addresses"),
            "customer_address",
            &address.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(address)
    }

    pub async fn update_customer_address(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
        address_id: Uuid,
        req: UpdateCustomerAddressRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<CustomerAddress, MasterDataError> {
        let request_hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/customers/{customer_id}/addresses/{address_id}"),
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        ensure_customer_exists_tx(&mut tx, ctx.owner_id, customer_id).await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<CustomerAddress>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!(
                "/api/v1/master-data/customers/{customer_id}/addresses/{address_id}"
            ),
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = load_customer_address(&mut tx, ctx.owner_id, customer_id, address_id).await?;
        let next = CustomerAddress {
            id: before.id,
            owner_id: before.owner_id,
            customer_id: before.customer_id,
            province: req.province.unwrap_or_else(|| before.province.clone()),
            city: req.city.unwrap_or_else(|| before.city.clone()),
            district: req.district.unwrap_or_else(|| before.district.clone()),
            detail_address: req
                .detail_address
                .unwrap_or_else(|| before.detail_address.clone()),
            contact_name: req
                .contact_name
                .unwrap_or_else(|| before.contact_name.clone()),
            contact_phone: req
                .contact_phone
                .unwrap_or_else(|| before.contact_phone.clone()),
            is_default: req.is_default.unwrap_or(before.is_default),
            created_at: before.created_at,
            updated_at: now,
        };
        validate_customer_address(&CreateCustomerAddressRequest {
            province: next.province.clone(),
            city: next.city.clone(),
            district: next.district.clone(),
            detail_address: next.detail_address.clone(),
            contact_name: next.contact_name.clone(),
            contact_phone: next.contact_phone.clone(),
            is_default: next.is_default,
        })?;
        let previous_default_ids =
            unset_default_customer_address(&mut tx, ctx.owner_id, customer_id, next.is_default, now)
                .await?;
        let row = sqlx::query_as::<_, CustomerAddressRow>(
            "UPDATE customer_addresses SET province=$4, city=$5, district=$6, detail_address=$7, contact_name=$8, contact_phone=$9, is_default=$10, updated_at=$11, version=version+1 WHERE owner_id=$1 AND customer_id=$2 AND id=$3 RETURNING id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone, is_default, created_at, updated_at",
        )
        .bind(ctx.owner_id)
        .bind(customer_id)
        .bind(address_id)
        .bind(&next.province)
        .bind(&next.city)
        .bind(&next.district)
        .bind(&next.detail_address)
        .bind(&next.contact_name)
        .bind(&next.contact_phone)
        .bind(next.is_default)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)?;
        let address = CustomerAddress::from(row);
        audit_customer_address_default_resets(
            &mut tx,
            ctx,
            previous_default_ids
                .into_iter()
                .filter(|id| *id != address.id)
                .collect(),
            now,
        )
        .await?;
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_customer_address",
            "customer_address",
            address.id,
            serde_json::to_value(before)
                .map_err(|error| MasterDataError::Serialize(error.to_string()))?,
            &address,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &address,
            now,
            "PATCH",
            &format!("/api/v1/master-data/customers/{customer_id}/addresses/{address_id}"),
            "customer_address",
            &address.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(address)
    }
}

async fn ensure_customer_exists(
    pool: &PgPool,
    owner_id: Uuid,
    customer_id: Uuid,
) -> Result<(), MasterDataError> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM customers WHERE owner_id=$1 AND id=$2)")
        .bind(owner_id)
        .bind(customer_id)
        .fetch_one(pool)
        .await
        .map_err(map_db_error)?
        .then_some(())
        .ok_or(MasterDataError::NotFound)
}

async fn ensure_customer_exists_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    customer_id: Uuid,
) -> Result<(), MasterDataError> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM customers WHERE owner_id=$1 AND id=$2 FOR UPDATE")
        .bind(owner_id)
        .bind(customer_id)
        .fetch_one(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|error| {
            if matches!(error, sqlx::Error::RowNotFound) {
                MasterDataError::NotFound
            } else {
                map_db_error(error)
            }
        })
}

async fn unset_default_customer_address(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    customer_id: Uuid,
    next_is_default: bool,
    now: DateTime<Utc>,
) -> Result<Vec<Uuid>, MasterDataError> {
    if !next_is_default {
        return Ok(Vec::new());
    }
    let previous_default_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM customer_addresses WHERE owner_id=$1 AND customer_id=$2 AND is_default=TRUE FOR UPDATE",
    )
    .bind(owner_id)
    .bind(customer_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query("UPDATE customer_addresses SET is_default=FALSE, updated_at=$3, version=version+1 WHERE owner_id=$1 AND customer_id=$2 AND is_default=TRUE")
        .bind(owner_id)
        .bind(customer_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(previous_default_ids)
}

async fn audit_customer_address_default_resets(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    address_ids: Vec<Uuid>,
    now: DateTime<Utc>,
) -> Result<(), MasterDataError> {
    for address_id in address_ids {
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "unset_customer_address_default",
            "M1",
            "customer_address",
            address_id.to_string(),
            Some(AuditDiff::compute(
                json!({"is_default": true}),
                json!({"is_default": false}),
            )),
        );
        audit.occurred_at = now;
        append_event_in_tx(tx, &audit)
            .await
            .map(|_| ())
            .map_err(|error| MasterDataError::Audit(format!("{error:?}")))?;
    }
    Ok(())
}

async fn load_customer_address(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
) -> Result<CustomerAddress, MasterDataError> {
    sqlx::query_as::<_, CustomerAddressRow>(
        "SELECT id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone, is_default, created_at, updated_at FROM customer_addresses WHERE owner_id=$1 AND customer_id=$2 AND id=$3",
    )
    .bind(owner_id)
    .bind(customer_id)
    .bind(address_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .map(CustomerAddress::from)
    .ok_or(MasterDataError::NotFound)
}

fn validate_customer_address(req: &CreateCustomerAddressRequest) -> Result<(), MasterDataError> {
    let fields = [
        &req.province,
        &req.city,
        &req.district,
        &req.detail_address,
        &req.contact_name,
        &req.contact_phone,
    ];
    if fields.iter().any(|value| value.trim().is_empty()) {
        return Err(MasterDataError::InvalidCustomerAddress);
    }
    Ok(())
}

impl From<CustomerAddressRow> for CustomerAddress {
    fn from(row: CustomerAddressRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            customer_id: row.customer_id,
            province: row.province,
            city: row.city,
            district: row.district,
            detail_address: row.detail_address,
            contact_name: row.contact_name,
            contact_phone: row.contact_phone,
            is_default: row.is_default,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
