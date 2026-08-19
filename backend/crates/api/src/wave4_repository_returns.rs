pub const PURCHASE_RETURN_STATUS_PENDING_APPROVAL: &str = "pending_approval";
pub const PURCHASE_RETURN_STATUS_APPROVED: &str = "approved";
pub const PURCHASE_RETURN_STATUS_PICKING: &str = "picking";
pub const PURCHASE_RETURN_STATUS_REVIEWED: &str = "reviewed";
pub const PURCHASE_RETURN_STATUS_SHIPPED: &str = "shipped";
pub const PURCHASE_RETURN_STATUS_CANCELLED: &str = "cancelled";

const PURCHASE_RETURN_SELECT_COLUMNS: &str = r#"
    id, owner_id, warehouse_id, return_no, document_type, source_purchase_order_no,
    supplier_id, supplier_name, reason, approval_source, status, product_code, qty,
    reject_reason, shipped_at, shipped_by, shipped_by_name, created_at, updated_at
"#;

#[derive(FromRow)]
struct PurchaseReturnOrderRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_id: Uuid,
    return_no: String,
    document_type: String,
    source_purchase_order_no: String,
    supplier_id: Option<Uuid>,
    supplier_name: String,
    reason: String,
    approval_source: String,
    status: String,
    product_code: String,
    qty: wms_domain::Quantity,
    reject_reason: Option<String>,
    shipped_at: Option<DateTime<Utc>>,
    shipped_by: Option<Uuid>,
    shipped_by_name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// 状态转换附加语义：驳回记录必填原因；发货交接记录交接时间与操作人。
enum PurchaseReturnTransitionExtra {
    None,
    Reject { reason: String },
    Ship,
}

impl PgWave4Repository {
    /// 创建采购退货出库单：初始状态 `pending_approval`，`return_no` 货主内唯一（冲突 409）。
    pub async fn create_purchase_return(
        &self,
        ctx: &AuthContext,
        req: CreatePurchaseReturnRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PurchaseReturnOrder>, Wave4RepositoryError> {
        let mut req = req;
        for (field, value) in [
            ("return_no", &mut req.return_no),
            (
                "source_purchase_order_no",
                &mut req.source_purchase_order_no,
            ),
            ("supplier_name", &mut req.supplier_name),
            ("reason", &mut req.reason),
            ("product_code", &mut req.product_code),
        ] {
            *value = value.trim().to_string();
            if value.is_empty() {
                return Err(Wave4RepositoryError::MissingRequiredField(field));
            }
        }
        if req.qty <= wms_domain::Quantity::ZERO {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let return_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO purchase_return_orders (
                id, owner_id, warehouse_id, return_no, document_type, source_purchase_order_no,
                supplier_id, supplier_name, reason, approval_source, status, product_code, qty,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14)
            "#,
        )
        .bind(return_id)
        .bind(ctx.owner_id)
        .bind(req.warehouse_id)
        .bind(&req.return_no)
        .bind(wms_domain::PURCHASE_RETURN_DOCUMENT_TYPE)
        .bind(&req.source_purchase_order_no)
        .bind(req.supplier_id)
        .bind(&req.supplier_name)
        .bind(&req.reason)
        .bind(wms_domain::PURCHASE_RETURN_APPROVAL_SOURCE)
        .bind(PURCHASE_RETURN_STATUS_PENDING_APPROVAL)
        .bind(&req.product_code)
        .bind(req.qty)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;

        let created = load_purchase_return(&mut tx, ctx.owner_id, return_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/purchase-returns",
            "purchase_return_order",
            created.id.to_string(),
            &created,
            now,
        )
        .await?;
        append_purchase_return_audit(
            &mut tx,
            ctx,
            audit,
            "create_purchase_return",
            created.id,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: created,
            replayed: false,
        })
    }

    pub async fn list_purchase_returns(
        &self,
        ctx: &AuthContext,
        status: Option<&str>,
        q: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<PurchaseReturnOrder>, Wave4RepositoryError> {
        let status = non_empty_filter(status);
        let q = non_empty_filter(q);
        let limit = i64::from(limit.unwrap_or(50).clamp(1, 200));
        let rows = sqlx::query_as::<_, PurchaseReturnOrderRow>(&format!(
            r#"
            SELECT {PURCHASE_RETURN_SELECT_COLUMNS}
              FROM purchase_return_orders
             WHERE owner_id = $1
               AND ($2::TEXT IS NULL OR status = $2)
               AND (
                    $3::TEXT IS NULL
                    OR return_no ILIKE '%' || $3 || '%'
                    OR source_purchase_order_no ILIKE '%' || $3 || '%'
                    OR supplier_name ILIKE '%' || $3 || '%'
               )
             ORDER BY updated_at DESC, return_no ASC
             LIMIT $4
            "#,
        ))
        .bind(ctx.owner_id)
        .bind(status)
        .bind(q)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(map_purchase_return).collect())
    }

    pub async fn get_purchase_return(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<PurchaseReturnOrder, Wave4RepositoryError> {
        let row = sqlx::query_as::<_, PurchaseReturnOrderRow>(&format!(
            r#"
            SELECT {PURCHASE_RETURN_SELECT_COLUMNS}
              FROM purchase_return_orders
             WHERE owner_id = $1 AND id = $2
            "#,
        ))
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        Ok(map_purchase_return(row))
    }

    /// 审批通过：`pending_approval` → `approved`。
    pub async fn approve_purchase_return(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PurchaseReturnOrder>, Wave4RepositoryError> {
        self.transition_purchase_return(
            ctx,
            id,
            "approve_purchase_return",
            "/api/v1/outbound/purchase-returns/{id}/approve",
            PURCHASE_RETURN_STATUS_PENDING_APPROVAL,
            PURCHASE_RETURN_STATUS_APPROVED,
            now,
            idempotency_key,
            audit,
            PurchaseReturnTransitionExtra::None,
        )
        .await
    }

    /// 审批驳回：`pending_approval` → `cancelled`，驳回原因必填。
    pub async fn reject_purchase_return(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: RejectPurchaseReturnRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PurchaseReturnOrder>, Wave4RepositoryError> {
        let reason = req.reason.trim().to_string();
        if reason.is_empty() {
            return Err(Wave4RepositoryError::MissingRejectReason);
        }
        self.transition_purchase_return(
            ctx,
            id,
            "reject_purchase_return",
            "/api/v1/outbound/purchase-returns/{id}/reject",
            PURCHASE_RETURN_STATUS_PENDING_APPROVAL,
            PURCHASE_RETURN_STATUS_CANCELLED,
            now,
            idempotency_key,
            audit,
            PurchaseReturnTransitionExtra::Reject { reason },
        )
        .await
    }

    /// 退货拣货：`approved` → `picking`。
    pub async fn pick_purchase_return(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PurchaseReturnOrder>, Wave4RepositoryError> {
        self.transition_purchase_return(
            ctx,
            id,
            "pick_purchase_return",
            "/api/v1/outbound/purchase-returns/{id}/pick",
            PURCHASE_RETURN_STATUS_APPROVED,
            PURCHASE_RETURN_STATUS_PICKING,
            now,
            idempotency_key,
            audit,
            PurchaseReturnTransitionExtra::None,
        )
        .await
    }

    /// 退货复核：`picking` → `reviewed`。
    pub async fn review_purchase_return(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PurchaseReturnOrder>, Wave4RepositoryError> {
        self.transition_purchase_return(
            ctx,
            id,
            "review_purchase_return",
            "/api/v1/outbound/purchase-returns/{id}/review",
            PURCHASE_RETURN_STATUS_PICKING,
            PURCHASE_RETURN_STATUS_REVIEWED,
            now,
            idempotency_key,
            audit,
            PurchaseReturnTransitionExtra::None,
        )
        .await
    }

    /// 出库交接：`reviewed` → `shipped`，记录交接时间与操作人。
    pub async fn ship_purchase_return(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PurchaseReturnOrder>, Wave4RepositoryError> {
        self.transition_purchase_return(
            ctx,
            id,
            "ship_purchase_return",
            "/api/v1/outbound/purchase-returns/{id}/ship",
            PURCHASE_RETURN_STATUS_REVIEWED,
            PURCHASE_RETURN_STATUS_SHIPPED,
            now,
            idempotency_key,
            audit,
            PurchaseReturnTransitionExtra::Ship,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn transition_purchase_return(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        action: &str,
        path: &str,
        expected: &str,
        next: &str,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
        extra: PurchaseReturnTransitionExtra,
    ) -> Result<IdempotentMutation<PurchaseReturnOrder>, Wave4RepositoryError> {
        let mut hash_payload = serde_json::json!({
            "purchase_return_id": id,
            "action": action,
        });
        if let PurchaseReturnTransitionExtra::Reject { reason } = &extra {
            hash_payload["reject_reason"] = serde_json::json!(reason);
        }
        let request_hash = request_hash(&hash_payload)?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let current = lock_purchase_return(&mut tx, ctx.owner_id, id).await?;
        if current.status != expected {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: expected.to_string(),
                actual: current.status,
            });
        }

        match &extra {
            PurchaseReturnTransitionExtra::None => {
                sqlx::query(
                    r#"
                    UPDATE purchase_return_orders
                       SET status = $3, updated_at = $4, version = version + 1
                     WHERE owner_id = $1 AND id = $2
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(id)
                .bind(next)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
            }
            PurchaseReturnTransitionExtra::Reject { reason } => {
                sqlx::query(
                    r#"
                    UPDATE purchase_return_orders
                       SET status = $3, reject_reason = $4, updated_at = $5, version = version + 1
                     WHERE owner_id = $1 AND id = $2
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(id)
                .bind(next)
                .bind(reason)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
            }
            PurchaseReturnTransitionExtra::Ship => {
                sqlx::query(
                    r#"
                    UPDATE purchase_return_orders
                       SET status = $3, shipped_at = $4, shipped_by = $5, shipped_by_name = $6,
                           updated_at = $4, version = version + 1
                     WHERE owner_id = $1 AND id = $2
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(id)
                .bind(next)
                .bind(now)
                .bind(ctx.user_id)
                .bind(&ctx.actor_name)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
            }
        }

        let updated = load_purchase_return(&mut tx, ctx.owner_id, id).await?;
        let audit = audit.map(|mut audit| {
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({ "status": &current.status }),
                serde_json::json!({
                    "status": &updated.status,
                    "reject_reason": &updated.reject_reason,
                    "shipped_at": &updated.shipped_at,
                    "shipped_by": &updated.shipped_by,
                }),
            ));
            audit
        });
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "purchase_return_order",
            updated.id.to_string(),
            &updated,
            now,
        )
        .await?;
        append_purchase_return_audit(&mut tx, ctx, audit, action, updated.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: updated,
            replayed: false,
        })
    }
}

async fn lock_purchase_return(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<PurchaseReturnOrder, Wave4RepositoryError> {
    let row = sqlx::query_as::<_, PurchaseReturnOrderRow>(&format!(
        r#"
        SELECT {PURCHASE_RETURN_SELECT_COLUMNS}
          FROM purchase_return_orders
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    ))
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)?;
    Ok(map_purchase_return(row))
}

async fn load_purchase_return(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<PurchaseReturnOrder, Wave4RepositoryError> {
    let row = sqlx::query_as::<_, PurchaseReturnOrderRow>(&format!(
        r#"
        SELECT {PURCHASE_RETURN_SELECT_COLUMNS}
          FROM purchase_return_orders
         WHERE owner_id = $1 AND id = $2
        "#,
    ))
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)?;
    Ok(map_purchase_return(row))
}

async fn append_purchase_return_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    audit: Option<AuditWriteRequest>,
    action: &str,
    resource_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let mut audit = audit.unwrap_or_else(|| {
        AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "M4",
            "purchase_return_order",
            resource_id.to_string(),
            None,
        )
    });
    audit.action = action.to_string();
    audit.module = "M4".to_string();
    audit.resource_type = "purchase_return_order".to_string();
    audit.resource_id = resource_id.to_string();
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))
}

fn map_purchase_return(row: PurchaseReturnOrderRow) -> PurchaseReturnOrder {
    PurchaseReturnOrder {
        id: row.id,
        owner_id: row.owner_id,
        warehouse_id: row.warehouse_id,
        return_no: row.return_no,
        document_type: row.document_type,
        source_purchase_order_no: row.source_purchase_order_no,
        supplier_id: row.supplier_id,
        supplier_name: row.supplier_name,
        reason: row.reason,
        approval_source: row.approval_source,
        status: row.status,
        product_code: row.product_code,
        qty: row.qty,
        reject_reason: row.reject_reason,
        shipped_at: row.shipped_at,
        shipped_by: row.shipped_by,
        shipped_by_name: row.shipped_by_name,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
