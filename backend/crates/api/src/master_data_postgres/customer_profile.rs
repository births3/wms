#[derive(sqlx::FromRow)]
struct CustomerProfileRow {
    customer_id: Uuid,
    owner_id: Uuid,
    customer_type: String,
    contact_name: Option<String>,
    contact_phone: Option<String>,
    business_scope: Vec<String>,
    qualification_certificates: Value,
    chain_name: Option<String>,
    updated_at: DateTime<Utc>,
}

impl PgMasterDataReadRepository {
    pub async fn get_customer_profile(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
    ) -> Result<CustomerProfile, MasterDataError> {
        let row = sqlx::query_as::<_, CustomerProfileRow>(
            "SELECT id AS customer_id, owner_id, customer_type, contact_name, contact_phone, business_scope, qualification_certificates, chain_name, updated_at FROM customers WHERE owner_id=$1 AND id=$2",
        )
        .bind(ctx.owner_id)
        .bind(customer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)?;
        customer_profile_from_row(row)
    }

    pub async fn upsert_customer_profile(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
        req: UpsertCustomerProfileRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<CustomerProfile, MasterDataError> {
        validate_customer_profile(&req)?;
        let request_hash = request_hash(&json!({
            "path": format!("/api/v1/master-data/customers/{customer_id}/profile"),
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<CustomerProfile>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/master-data/customers/{customer_id}/profile"),
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = load_customer_profile_before(&mut tx, ctx.owner_id, customer_id).await?;
        let qualifications = serde_json::to_value(&req.qualification_certificates)
            .map_err(|error| MasterDataError::Serialize(error.to_string()))?;
        let row = sqlx::query_as::<_, CustomerProfileRow>(
            "UPDATE customers SET customer_type=$3, contact_name=$4, contact_phone=$5, business_scope=$6, qualification_certificates=$7, chain_name=$8, updated_at=$9, version=version+1 WHERE owner_id=$1 AND id=$2 RETURNING id AS customer_id, owner_id, customer_type, contact_name, contact_phone, business_scope, qualification_certificates, chain_name, updated_at",
        )
        .bind(ctx.owner_id)
        .bind(customer_id)
        .bind(&req.customer_type)
        .bind(req.contact_name.trim())
        .bind(req.contact_phone.trim())
        .bind(&req.business_scope)
        .bind(qualifications)
        .bind(req.chain_name.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)?;
        let profile = customer_profile_from_row(row)?;
        append_master_data_update_audit(
            &mut tx,
            ctx,
            "update_customer_profile",
            "customer_profile",
            customer_id,
            before,
            &profile,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &profile,
            now,
            "PATCH",
            &format!("/api/v1/master-data/customers/{customer_id}/profile"),
            "customer_profile",
            &customer_id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(profile)
    }
}

async fn load_customer_profile_before(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    customer_id: Uuid,
) -> Result<Value, MasterDataError> {
    sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM (SELECT id AS customer_id, owner_id, customer_type, contact_name, contact_phone, business_scope, qualification_certificates, chain_name, updated_at FROM customers WHERE owner_id=$1 AND id=$2) t",
    )
    .bind(owner_id)
    .bind(customer_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(MasterDataError::NotFound)
}

fn customer_profile_from_row(row: CustomerProfileRow) -> Result<CustomerProfile, MasterDataError> {
    let qualification_certificates = serde_json::from_value(row.qualification_certificates)
        .map_err(|error| MasterDataError::Serialize(error.to_string()))?;
    Ok(CustomerProfile {
        customer_id: row.customer_id,
        owner_id: row.owner_id,
        customer_type: row.customer_type,
        contact_name: row.contact_name,
        contact_phone: row.contact_phone,
        business_scope: row.business_scope,
        qualification_certificates,
        chain_name: row.chain_name,
        updated_at: row.updated_at,
    })
}

fn validate_customer_profile(req: &UpsertCustomerProfileRequest) -> Result<(), MasterDataError> {
    if !matches!(req.customer_type.as_str(), "customer" | "store")
        || req.contact_name.trim().is_empty()
        || req.contact_phone.trim().is_empty()
        || (req.customer_type == "store" && req.business_scope.is_empty())
        || req
            .qualification_certificates
            .iter()
            .any(|item| item.certificate_type.trim().is_empty() || item.certificate_no.trim().is_empty())
    {
        return Err(MasterDataError::InvalidCustomerProfile);
    }
    Ok(())
}
