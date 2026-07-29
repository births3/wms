impl PgWave3Repository {
    pub async fn inspect_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: InspectReceivingOrderRequest,
        today: NaiveDate,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingInspectionRecord>, Wave3RepositoryError> {
        if req.accepted_qty < 0 || req.rejected_qty < 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let inspected_qty = req
            .accepted_qty
            .checked_add(req.rejected_qty)
            .filter(|qty| *qty > 0)
            .ok_or(Wave3RepositoryError::InvalidQuantity)?;
        if req.batch_no.trim().is_empty() {
            return Err(Wave3RepositoryError::InvalidBatchPolicy);
        }
        let quality_checks = validate_inspection_quality_checks(&req)?;
        let sampling_qty = req.sampling_qty.unwrap_or(0);
        if sampling_qty <= 0 {
            return Err(Wave3RepositoryError::MissingRequiredField(
                "sampling_qty".to_string(),
            ));
        }
        let mut unique_trace_codes = req.trace_codes.clone();
        unique_trace_codes.sort_unstable();
        unique_trace_codes.dedup();
        if unique_trace_codes.len() != req.trace_codes.len() {
            return Err(Wave3RepositoryError::DuplicateTraceCode);
        }
        let production_date = parse_date(&req.production_date)?;
        let expiry_date = parse_date(&req.expiry_date)?;
        if expiry_date < today {
            return Err(Wave3RepositoryError::BatchExpired);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
        }))?;

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

        resolve_quality_color(&mut tx, ctx.owner_id, &req.quality_status, now).await?;

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "inspecting" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "inspecting".to_string(),
                actual: order.status,
            });
        }

        let received_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(actual_qty), 0)::BIGINT FROM receiving_order_receipts WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let total_previous_inspected_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(accepted_qty + rejected_qty), 0)::BIGINT FROM receiving_inspections WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if total_previous_inspected_qty
            .checked_add(inspected_qty)
            .is_none_or(|qty| qty > received_qty)
        {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        let line = sqlx::query_as::<_, ReceivingOrderLineRow>(
            r#"
            SELECT id, line_no, product_id, product_code, expected_qty, batch_no,
                   production_date, expiry_date
              FROM receiving_order_lines
             WHERE receiving_order_id = $1
               AND owner_id = $2
               AND (
                    ($3 = 'purchase_inbound' AND batch_no IS NULL)
                    OR ($3 = 'sales_return' AND batch_no = $4)
               )
             ORDER BY line_no
             LIMIT 1
             FOR UPDATE
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&order.document_type)
        .bind(&req.batch_no)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let product_id = line.product_id.ok_or(Wave3RepositoryError::MissingProduct)?;
        match validate_drug_inspection_for_acceptance(
            &mut tx,
            ctx,
            id,
            &order.receipt_no,
            product_id,
            &req.batch_no,
            idempotency_key,
            now,
        )
        .await?
        {
            DrugInspectionAcceptanceDecision::Continue => {}
            DrugInspectionAcceptanceDecision::MissingBlocked => {
                let validation_audit = AuditWriteRequest::from_auth_context(
                    ctx,
                    "di.acceptance.missing_blocked",
                    "M-DI",
                    "receiving_order",
                    id.to_string(),
                    Some(AuditDiff::compute(
                        serde_json::json!({}),
                        serde_json::json!({
                            "batch_no": req.batch_no,
                            "result": "missing_blocked"
                        }),
                    )),
                );
                append_event_in_tx(&mut tx, &validation_audit)
                    .await
                    .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
                tx.commit().await.map_err(map_db_error)?;
                return Err(Wave3RepositoryError::DrugInspectionMissingBlocked);
            }
            DrugInspectionAcceptanceDecision::UnqualifiedBlocked(report_version_id) => {
                enqueue_drug_inspection_unqualified_liaison(
                    &mut tx,
                    ctx,
                    id,
                    &order.receipt_no,
                    product_id,
                    &req.batch_no,
                    report_version_id,
                    now,
                )
                .await?;
                let validation_audit = AuditWriteRequest::from_auth_context(
                    ctx,
                    "di.acceptance.unqualified_blocked",
                    "M-DI",
                    "receiving_order",
                    id.to_string(),
                    Some(AuditDiff::compute(
                        serde_json::json!({}),
                        serde_json::json!({
                            "batch_no": req.batch_no,
                            "report_version_id": report_version_id,
                            "result": "unqualified_blocked"
                        }),
                    )),
                );
                append_event_in_tx(&mut tx, &validation_audit)
                    .await
                    .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
                tx.commit().await.map_err(map_db_error)?;
                return Err(Wave3RepositoryError::DrugInspectionUnqualifiedBlocked);
            }
        }
        let previous_inspected_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(accepted_qty + rejected_qty), 0)::BIGINT FROM receiving_inspections WHERE receiving_order_id = $1 AND owner_id = $2 AND batch_no = $3",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.batch_no)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if previous_inspected_qty
            .checked_add(inspected_qty)
            .is_none_or(|qty| qty > line.expected_qty)
        {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }
        if !req.trace_codes.is_empty() {
            let trace_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM receiving_inspections WHERE receiving_order_id = $1 AND owner_id = $2 AND trace_codes && $3)",
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.trace_codes)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if trace_exists {
                return Err(Wave3RepositoryError::DuplicateTraceCode);
            }
        }
        if let Some(approval_no) = req
            .approval_no
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let product_approval: Option<String> = sqlx::query_scalar(
                r#"
                SELECT NULLIF(TRIM(product.approval_no), '')
                  FROM products product
                 WHERE product.owner_id = $1 AND product.product_code = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(&line.product_code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .flatten();
            if let Some(expected) = product_approval {
                if expected != approval_no {
                    return Err(Wave3RepositoryError::MissingRequiredField(
                        "approval_no_mismatch".to_string(),
                    ));
                }
            }
        }

        let inspection = ReceivingInspectionRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no.clone(),
            accepted_qty: req.accepted_qty,
            rejected_qty: req.rejected_qty,
            quality_status: req.quality_status.clone(),
            quality_checks: Some(quality_checks.clone()),
            sampling_qty: Some(sampling_qty),
            approval_no: req
                .approval_no
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_inspections (
                id, receiving_order_id, owner_id, batch_no, accepted_qty,
                rejected_qty, production_date, expiry_date, quality_status,
                trace_codes, quality_checks, sampling_qty, approval_no, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(inspection.id)
        .bind(inspection.receiving_order_id)
        .bind(inspection.owner_id)
        .bind(&inspection.batch_no)
        .bind(inspection.accepted_qty)
        .bind(inspection.rejected_qty)
        .bind(production_date)
        .bind(expiry_date)
        .bind(&inspection.quality_status)
        .bind(&req.trace_codes)
        .bind(sqlx::types::Json(quality_checks.clone()))
        .bind(sampling_qty)
        .bind(&inspection.approval_no)
        .bind(inspection.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        if req.quality_status.eq_ignore_ascii_case("unqualified")
            || req.quality_status.eq_ignore_ascii_case("不合格")
        {
            enqueue_unqualified_quality_liaison(
                &mut tx,
                ctx,
                order.id,
                &order.receipt_no,
                &inspection,
                now,
            )
            .await?;
        }

        let updated_line = sqlx::query(
            r#"
            UPDATE receiving_order_lines
               SET batch_no = $3, production_date = $4, expiry_date = $5
             WHERE id = $1 AND receiving_order_id = $2 AND owner_id = $6
            "#,
        )
        .bind(line.id)
        .bind(id)
        .bind(&req.batch_no)
        .bind(production_date)
        .bind(expiry_date)
        .bind(ctx.owner_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if updated_line.rows_affected() != 1 {
            return Err(Wave3RepositoryError::NotFound);
        }

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/inspect",
            "receiving_inspection",
            inspection.id.to_string(),
            &inspection,
            now,
        )
        .await?;
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: inspection,
            replayed: false,
        })
    }

    pub async fn sign_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: SignInspectionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InspectionSignatureRecord>, Wave3RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
        }))?;

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

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        // 取最近一条未完成双签的第一签字记录；已完整双签则拒绝。
        let existing_signature = sqlx::query_as::<_, InspectionSignatureRow>(
            r#"
            SELECT id, receiving_order_id, owner_id, first_signer_id,
                   second_signer_id, strategy_rule_id, approval_record_id, signed_at
              FROM receiving_inspection_signatures
             WHERE receiving_order_id = $1 AND owner_id = $2
             ORDER BY signed_at DESC, id DESC
             LIMIT 1
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;

        // 第二人独立签字：订单处于待第二人，当前用户必须是第二签字人。
        // append-only：追加一条完整双签记录，禁止 UPDATE 第一条签名。
        if order.status == "awaiting_second_sign" {
            let Some(existing) = existing_signature else {
                return Err(Wave3RepositoryError::InvalidStatus {
                    expected: "awaiting_second_sign with first signature".to_string(),
                    actual: order.status,
                });
            };
            if existing.second_signer_id.is_some() {
                return Err(Wave3RepositoryError::InvalidStatus {
                    expected: "awaiting_second_sign".to_string(),
                    actual: "already_fully_signed".to_string(),
                });
            }
            // 第二步只认当前认证主体为第二人；请求体 first_signer_id 忽略。
            let second_signer_id = req.second_signer_id.unwrap_or(ctx.user_id);
            if second_signer_id != ctx.user_id {
                return Err(Wave3RepositoryError::UnauthorizedSigner);
            }
            if second_signer_id == existing.first_signer_id {
                return Err(Wave3RepositoryError::SameSigner);
            }
            ensure_receiving_clerk_signer(&mut tx, ctx.owner_id, second_signer_id).await?;

            let complete_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO receiving_inspection_signatures (
                    id, receiving_order_id, owner_id, dual_required,
                    first_signer_id, second_signer_id, strategy_rule_id,
                    approval_record_id, signed_at
                ) VALUES ($1, $2, $3, TRUE, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(complete_id)
            .bind(id)
            .bind(ctx.owner_id)
            .bind(existing.first_signer_id)
            .bind(second_signer_id)
            .bind(existing.strategy_rule_id)
            .bind(existing.approval_record_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

            sqlx::query(
                "UPDATE receiving_orders SET status = 'putaway', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            integrations::create_putaway_tasks_for_receiving_order(&mut tx, ctx, &order, now)
                .await?;

            let signature = InspectionSignatureRecord {
                id: complete_id,
                receiving_order_id: id,
                owner_id: ctx.owner_id,
                first_signer_id: existing.first_signer_id,
                second_signer_id: Some(second_signer_id),
                strategy_rule_id: existing.strategy_rule_id,
                approval_record_id: existing.approval_record_id,
                signed_at: now,
            };
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &request_hash,
                "POST",
                "/api/v1/inbound/receiving-orders/{id}/sign",
                "receiving_inspection_signature",
                signature.id.to_string(),
                &signature,
                now,
            )
            .await?;
            if let Some(mut audit) = audit {
                audit.diff = Some(AuditDiff::compute(
                    serde_json::json!({ "status": "awaiting_second_sign" }),
                    serde_json::json!({
                        "status": "putaway",
                        "second_signer_id": second_signer_id,
                    }),
                ));
                append_event_in_tx(&mut tx, &audit)
                    .await
                    .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
            }
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value: signature,
                replayed: false,
            });
        }

        if order.status != "inspecting" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "inspecting".to_string(),
                actual: order.status,
            });
        }
        // 第一签字人必须是当前认证用户，禁止代签。
        if req.first_signer_id != ctx.user_id {
            return Err(Wave3RepositoryError::UnauthorizedSigner);
        }
        ensure_receiving_clerk_signer(&mut tx, ctx.owner_id, req.first_signer_id).await?;

        let product_codes: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT product_code FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2 ORDER BY product_code",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let strategy = crate::dual_person_policy::resolve_for_product_codes_in_tx(
            &mut tx,
            ctx.owner_id,
            order.warehouse_id,
            &product_codes,
            "入库",
            "验收",
        )
        .await
        .map_err(|error| Wave3RepositoryError::Database(format!("M-VR 双人策略解析失败: {error:?}")))?;
        let dual_required = strategy.policy != wms_domain::DualPersonPolicy::Single;
        // 双人策略禁止一次请求提交两名签字人。
        if dual_required && req.second_signer_id.is_some() {
            return Err(Wave3RepositoryError::UnauthorizedSigner);
        }
        let approval_record_id = if strategy.policy
            == wms_domain::DualPersonPolicy::DualScanWithApproval
        {
            crate::dual_person_policy::approved_dual_person_record_in_tx(
                &mut tx,
                ctx.owner_id,
                &id.to_string(),
            )
            .await
            .map_err(|error| {
                Wave3RepositoryError::Database(format!("M-VR 审批记录查询失败: {error:?}"))
            })?
            .ok_or(Wave3RepositoryError::DualPersonApprovalRequired)?
            .into()
        } else {
            None
        };

        let received_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(actual_qty), 0)::BIGINT FROM receiving_order_receipts WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let inspected_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(accepted_qty + rejected_qty), 0)::BIGINT FROM receiving_inspections WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if received_qty <= 0 || inspected_qty != received_qty {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }
        let incomplete_lines: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::BIGINT
              FROM receiving_order_lines AS line
             WHERE line.receiving_order_id = $1
               AND line.owner_id = $2
               AND NOT EXISTS (
                   SELECT 1
                     FROM receiving_inspections AS inspection
                    WHERE inspection.receiving_order_id = line.receiving_order_id
                      AND inspection.owner_id = line.owner_id
                      AND inspection.batch_no = line.batch_no
               )
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if incomplete_lines > 0 {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        let signature = InspectionSignatureRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            first_signer_id: req.first_signer_id,
            second_signer_id: None,
            strategy_rule_id: strategy.source_rule_id,
            approval_record_id,
            signed_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_inspection_signatures (
                id, receiving_order_id, owner_id, dual_required,
                first_signer_id, second_signer_id, strategy_rule_id,
                approval_record_id, signed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(signature.id)
        .bind(signature.receiving_order_id)
        .bind(signature.owner_id)
        .bind(dual_required)
        .bind(signature.first_signer_id)
        .bind(signature.second_signer_id)
        .bind(signature.strategy_rule_id)
        .bind(signature.approval_record_id)
        .bind(signature.signed_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let next_status = if dual_required {
            "awaiting_second_sign"
        } else {
            "putaway"
        };
        sqlx::query(
            "UPDATE receiving_orders SET status = $3, updated_at = $4, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(next_status)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        if next_status == "putaway" {
            integrations::create_putaway_tasks_for_receiving_order(&mut tx, ctx, &order, now)
                .await?;
        }

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/sign",
            "receiving_inspection_signature",
            signature.id.to_string(),
            &signature,
            now,
        )
        .await?;
        if let Some(mut audit) = audit {
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({}),
                serde_json::json!({
                    "first_signer_id": signature.first_signer_id,
                    "second_signer_id": signature.second_signer_id,
                    "status": next_status,
                    "strategy_rule_id": signature.strategy_rule_id,
                    "approval_record_id": signature.approval_record_id,
                }),
            ));
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: signature,
            replayed: false,
        })
    }
}
