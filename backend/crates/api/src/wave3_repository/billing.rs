use super::*;

impl PgWave3Repository {
    pub async fn create_billing_account(
        &self,
        ctx: &AuthContext,
        req: CreateBillingAccountRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingAccount, Wave3RepositoryError> {
        self.create_billing_account_in_tx(ctx, req, now, None).await
    }

    pub async fn create_billing_account_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateBillingAccountRequest,
        now: DateTime<Utc>,
        audit: AuditWriteRequest,
    ) -> Result<BillingAccount, Wave3RepositoryError> {
        self.create_billing_account_in_tx(ctx, req, now, Some(audit))
            .await
    }

    async fn create_billing_account_in_tx(
        &self,
        ctx: &AuthContext,
        req: CreateBillingAccountRequest,
        now: DateTime<Utc>,
        audit: Option<AuditWriteRequest>,
    ) -> Result<BillingAccount, Wave3RepositoryError> {
        let account = BillingAccount {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            account_code: req.account_code,
            account_name: req.account_name,
            status: "active".to_string(),
            created_at: now,
        };
        let mut tx = self.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO billing_accounts (
                id, owner_id, account_code, account_name, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            "#,
        )
        .bind(account.id)
        .bind(account.owner_id)
        .bind(&account.account_code)
        .bind(&account.account_name)
        .bind(&account.status)
        .bind(account.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some(mut audit) = audit {
            audit.resource_id = account.id.to_string();
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(account)
    }

    pub async fn create_billing_contract(
        &self,
        ctx: &AuthContext,
        req: CreateBillingContractRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingContract, Wave3RepositoryError> {
        let mut tx = self.begin().await?;
        let contract = create_billing_contract_in_tx(&mut tx, ctx, &req, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(contract)
    }

    pub async fn create_billing_contract_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateBillingContractRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        mut audit: AuditWriteRequest,
    ) -> Result<IdempotentMutation<BillingContract>, Wave3RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let contract = create_billing_contract_in_tx(&mut tx, ctx, &req, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/billing/contracts",
            "billing_contract",
            contract.id.to_string(),
            &contract,
            now,
        )
        .await?;
        audit.resource_id = contract.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: contract,
            replayed: false,
        })
    }

    pub async fn create_billing_rule(
        &self,
        ctx: &AuthContext,
        req: CreateBillingRuleRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingRule, Wave3RepositoryError> {
        let mut tx = self.begin().await?;
        let rule = Self::create_billing_rule_in_tx(&mut tx, ctx, &req, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(rule)
    }

    pub async fn create_billing_rule_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateBillingRuleRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        mut audit: AuditWriteRequest,
    ) -> Result<IdempotentMutation<BillingRule>, Wave3RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let rule = Self::create_billing_rule_in_tx(&mut tx, ctx, &req, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/billing/rules",
            "billing_rule",
            rule.id.to_string(),
            &rule,
            now,
        )
        .await?;
        audit.resource_id = rule.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: rule,
            replayed: false,
        })
    }

    async fn create_billing_rule_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        req: &CreateBillingRuleRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingRule, Wave3RepositoryError> {
        validate_billing_rule_request(req).map_err(map_rule_validation_error)?;
        let effective_from = parse_date(&req.effective_from)?;
        let effective_to = parse_date(&req.effective_to)?;
        if effective_to < effective_from {
            return Err(Wave3RepositoryError::InvalidEffectiveWindow);
        }

        let contract_window: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT valid_from, valid_to FROM billing_contracts WHERE id = $1 AND owner_id = $2 FOR UPDATE",
        )
        .bind(req.contract_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let Some((contract_from, contract_to)) = contract_window else {
            return Err(Wave3RepositoryError::NotFound);
        };
        if effective_from < contract_from || effective_to > contract_to {
            return Err(Wave3RepositoryError::InvalidEffectiveWindow);
        }
        let overlap: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM billing_rules
                 WHERE owner_id = $1
                   AND contract_id = $2
                   AND charge_item = $3
                   AND unit = $4
                   AND billing_cycle = $5
                   AND effective_from <= $7
                   AND effective_to >= $6
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.contract_id)
        .bind(&req.charge_item)
        .bind(&req.unit)
        .bind(&req.billing_cycle)
        .bind(effective_from)
        .bind(effective_to)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if overlap {
            return Err(Wave3RepositoryError::BillingRuleConflict);
        }

        let rule = BillingRule {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            contract_id: req.contract_id,
            charge_item: req.charge_item.clone(),
            unit: req.unit.clone(),
            unit_price_cents: req.unit_price_cents,
            billing_cycle: req.billing_cycle.clone(),
            effective_from: effective_from.to_string(),
            effective_to: effective_to.to_string(),
            created_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO billing_rules (
                id, owner_id, contract_id, charge_item, unit, unit_price_cents,
                billing_cycle, effective_from, effective_to, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(rule.id)
        .bind(rule.owner_id)
        .bind(rule.contract_id)
        .bind(&rule.charge_item)
        .bind(&rule.unit)
        .bind(rule.unit_price_cents)
        .bind(&rule.billing_cycle)
        .bind(effective_from)
        .bind(effective_to)
        .bind(rule.created_at)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        Ok(rule)
    }
}

fn map_rule_validation_error(error: BillingRuleValidationError) -> Wave3RepositoryError {
    match error {
        BillingRuleValidationError::InvalidChargeItem
        | BillingRuleValidationError::InvalidUnit
        | BillingRuleValidationError::InvalidBillingCycle => {
            Wave3RepositoryError::InvalidBillingRuleField
        }
        BillingRuleValidationError::InvalidRate => Wave3RepositoryError::InvalidRate,
        BillingRuleValidationError::InvalidEffectiveWindow => {
            Wave3RepositoryError::InvalidEffectiveWindow
        }
    }
}

async fn create_billing_contract_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    req: &CreateBillingContractRequest,
    now: DateTime<Utc>,
) -> Result<BillingContract, Wave3RepositoryError> {
    let valid_from = parse_date(&req.valid_from)?;
    let valid_to = parse_date(&req.valid_to)?;
    if valid_to < valid_from {
        return Err(Wave3RepositoryError::InvalidEffectiveWindow);
    }
    let row = sqlx::query_as::<_, BillingContractRow>(
        r#"
        INSERT INTO billing_contracts (
            id, owner_id, account_id, contract_no, valid_from, valid_to,
            status, created_at, updated_at
        )
        SELECT $1, $2, $3, $4, $5, $6, 'active', $7, $7
          FROM billing_accounts
         WHERE id = $3 AND owner_id = $2
        RETURNING id, owner_id, account_id, contract_no, valid_from, valid_to,
                  status, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(req.account_id)
    .bind(&req.contract_no)
    .bind(valid_from)
    .bind(valid_to)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::NotFound)?;
    Ok(BillingContract {
        id: row.id,
        owner_id: row.owner_id,
        account_id: row.account_id,
        contract_no: row.contract_no,
        valid_from: row.valid_from.to_string(),
        valid_to: row.valid_to.to_string(),
        status: row.status,
        created_at: row.created_at,
    })
}
