impl MasterDataAppState {
    async fn list_customer_addresses(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
    ) -> Result<Vec<CustomerAddress>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_customer_addresses(ctx, customer_id).await?);
        }
        Err(MasterDataHandlerError::PostgresReadNotImplemented)
    }

    async fn create_customer_address(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
        req: CreateCustomerAddressRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<CustomerAddress, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .create_customer_address(ctx, customer_id, req, now, idempotency_key)
                .await?);
        }
        Err(MasterDataHandlerError::PostgresWriteNotImplemented)
    }

    async fn update_customer_address(
        &self,
        ctx: &AuthContext,
        customer_id: Uuid,
        address_id: Uuid,
        req: UpdateCustomerAddressRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<CustomerAddress, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .update_customer_address(
                    ctx,
                    customer_id,
                    address_id,
                    req,
                    now,
                    idempotency_key,
                )
                .await?);
        }
        Err(MasterDataHandlerError::PostgresWriteNotImplemented)
    }
}

async fn list_customer_addresses_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(customer_id): Path<Uuid>,
) -> Result<Json<CustomerAddressListResponse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_READ_PERMISSION)?;
    let data = state.list_customer_addresses(&ctx, customer_id).await?;
    Ok(Json(CustomerAddressListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_customer_address_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(customer_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<CreateCustomerAddressRequest>,
) -> Result<Json<CustomerAddress>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .create_customer_address(
                &ctx,
                customer_id,
                req,
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}

async fn update_customer_address_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path((customer_id, address_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(req): Json<UpdateCustomerAddressRequest>,
) -> Result<Json<CustomerAddress>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_customer_address(
                &ctx,
                customer_id,
                address_id,
                req,
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}
