impl MasterDataAppState {
    async fn get_customer_profile(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
    ) -> Result<CustomerProfile, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.get_customer_profile(ctx, customer_id).await?);
        }
        Err(MasterDataHandlerError::PostgresReadNotImplemented)
    }

    async fn upsert_customer_profile(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
        req: UpsertCustomerProfileRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<CustomerProfile, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .upsert_customer_profile(ctx, customer_id, req, now, idempotency_key)
                .await?);
        }
        Err(MasterDataHandlerError::PostgresWriteNotImplemented)
    }
}

async fn get_customer_profile_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(customer_id): Path<Uuid>,
) -> Result<Json<CustomerProfile>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_READ_PERMISSION)?;
    Ok(Json(state.get_customer_profile(&ctx, customer_id).await?))
}

async fn upsert_customer_profile_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(customer_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpsertCustomerProfileRequest>,
) -> Result<Json<CustomerProfile>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .upsert_customer_profile(
                &ctx,
                customer_id,
                req,
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}
