use super::*;

impl PgWave4Repository {
    pub async fn attach_erp_outbound_identity(
        &self,
        owner_id: Uuid,
        order_id: Uuid,
        erp_bill_id: i64,
        erp_bill_code: &str,
        revision: i32,
        order_type: i32,
        send_mode: Option<i32>,
        correlation_id: &str,
    ) -> Result<(), Wave4RepositoryError> {
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET erp_bill_id=$3, erp_bill_code=$4, erp_revision=$5,
                   erp_order_type=$6, send_mode=$7, erp_correlation_id=$8
             WHERE owner_id=$1 AND id=$2
            "#,
        )
        .bind(owner_id)
        .bind(order_id)
        .bind(erp_bill_id)
        .bind(erp_bill_code)
        .bind(revision)
        .bind(order_type)
        .bind(send_mode)
        .bind(correlation_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }

    pub async fn cancel_erp_outbound_order(
        &self,
        ctx: &AuthContext,
        erp_bill_code: &str,
        revision: i32,
        command_id: &str,
        correlation_id: &str,
        memo: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<IdempotentMutation<Uuid>, Wave4RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        sqlx::query(
            "INSERT INTO erp_order_cancel_commands (owner_id,command_id,erp_bill_code,revision,order_type,correlation_id,memo,created_at) VALUES ($1,$2,$3,$4,2,$5,$6,$7) ON CONFLICT (owner_id,command_id) DO NOTHING",
        )
        .bind(ctx.owner_id)
        .bind(command_id)
        .bind(erp_bill_code)
        .bind(revision)
        .bind(correlation_id)
        .bind(memo)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let order: Option<(Uuid, String, Uuid)> = sqlx::query_as(
            "SELECT id,status,warehouse_id FROM outbound_orders WHERE owner_id=$1 AND erp_bill_code=$2 AND erp_revision=$3 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(erp_bill_code)
        .bind(revision)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some((id, status, warehouse_id)) = order else {
            // 订单尚未挂接 ERP 身份：保留 pending 取消命令（cancel-on-arrival
            // 语义，见 h8_inbound_order_cancel_postgres 的 waiting_cancel 用例），
            // 由 worker 重试同一 command_id 时 resolve；NotFound 映射为 425 可重试。
            tx.commit().await.map_err(map_db_error)?;
            return Err(Wave4RepositoryError::NotFound);
        };
        // 幂等重放检查须在订单 FOR UPDATE 锁之后执行：并发同 command_id
        // 的两个请求被订单锁串行化，后到者在此处命中已写入的 outbox 行并
        // 返回 replayed，而不是在 outbox 唯一索引上撞出重复键错误。
        if let Some(id) = sqlx::query_scalar(
            "SELECT outbound_order_id FROM shipment_confirm_erp_feedback_outbox WHERE owner_id=$1 AND command_id=$2",
        )
        .bind(ctx.owner_id)
        .bind(command_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation { value: id, replayed: true });
        }
        let cancellable = matches!(
            status.as_str(),
            "pending_validation"
                | "validation_exception"
                | "confirmed"
                | "void_requested"
                | "cancelled"
        );
        if cancellable && status != "cancelled" {
            sqlx::query("UPDATE outbound_orders SET status='cancelled',updated_at=$3,version=version+1 WHERE owner_id=$1 AND id=$2")
                .bind(ctx.owner_id).bind(id).bind(now).execute(&mut *tx).await.map_err(map_db_error)?;
        }
        let (feedback_type, result_code, result_message) = if cancellable {
            (100, None, None)
        } else {
            (
                9,
                Some("ORDER_ALREADY_IN_WAVE"),
                Some("订单已进入波次，拒绝 ERP 取消"),
            )
        };
        let outbox_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO shipment_confirm_erp_feedback_outbox (id,owner_id,outbound_order_id,command_id,event_type,payload,status,attempt_count,next_attempt_at,created_at,updated_at) VALUES ($1,$2,$3,$4,'order_status',$5,'pending',0,$6,$6,$6)",
        )
        .bind(outbox_id)
        .bind(ctx.owner_id)
        .bind(id)
        .bind(command_id)
        .bind(serde_json::json!({
            "warehouse_id": warehouse_id,
            "erp_bill_code": erp_bill_code,
            "revision": revision,
            "order_type": 2,
            "feedback_type": feedback_type,
            "command_id": command_id,
            "result_code": result_code,
            "result_message": result_message,
            "correlation_id": correlation_id,
            "feedback_time": now,
        }))
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query("UPDATE erp_order_cancel_commands SET status=$3,resolved_at=$4 WHERE owner_id=$1 AND command_id=$2")
            .bind(ctx.owner_id)
            .bind(command_id)
            .bind(if cancellable { "completed" } else { "rejected" })
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "erp_order_cancel",
            "H8",
            "outbound_order",
            id.to_string(),
            Some(AuditDiff::compute(
                serde_json::json!({"status": status}),
                serde_json::json!({"status": if cancellable { "cancelled" } else { status.as_str() }, "command_id": command_id, "memo": memo}),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: id,
            replayed: false,
        })
    }
}

pub(super) async fn resolve_outbound_review_policy(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    order_id: Uuid,
    order: &OutboundOrder,
    second_reviewer_id: Option<Uuid>,
) -> Result<
    (
        crate::dual_person_policy::ResolvedDualPersonPolicy,
        Option<Uuid>,
    ),
    Wave4RepositoryError,
> {
    let product_codes = order
        .lines
        .iter()
        .map(|line| line.product_code.clone())
        .collect::<Vec<_>>();
    let strategy = crate::dual_person_policy::resolve_for_product_codes_in_tx(
        tx,
        ctx.owner_id,
        order.warehouse_id,
        &product_codes,
        "出库",
        "复核",
    )
    .await
    .map_err(|error| Wave4RepositoryError::Database(format!("M-VR 双人策略解析失败: {error:?}")))?;
    if strategy.policy != wms_domain::DualPersonPolicy::Single && second_reviewer_id.is_none() {
        return Err(Wave4RepositoryError::MissingSecondReviewer);
    }
    if let Some(second_reviewer_id) = second_reviewer_id {
        let qualified = crate::dual_person_policy::is_active_operator_with_role_in_tx(
            tx,
            ctx.owner_id,
            second_reviewer_id,
            "custodian",
        )
        .await
        .map_err(|error| {
            Wave4RepositoryError::Database(format!("M-VR 第二复核员资质校验失败: {error:?}"))
        })?;
        if !qualified {
            return Err(Wave4RepositoryError::UnqualifiedSecondReviewer);
        }
    }
    let approval_record_id =
        if strategy.policy == wms_domain::DualPersonPolicy::DualScanWithApproval {
            crate::dual_person_policy::approved_dual_person_record_in_tx(
                tx,
                ctx.owner_id,
                &order_id.to_string(),
            )
            .await
            .map_err(|error| {
                Wave4RepositoryError::Database(format!("M-VR 审批记录查询失败: {error:?}"))
            })?
            .ok_or(Wave4RepositoryError::DualPersonApprovalRequired)?
            .into()
        } else {
            None
        };
    Ok((strategy, approval_record_id))
}
