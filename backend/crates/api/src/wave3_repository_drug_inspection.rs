enum DrugInspectionAcceptanceDecision {
    Continue,
    MissingBlocked,
    UnqualifiedBlocked(Uuid),
}

async fn validate_drug_inspection_for_acceptance(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    receiving_order_id: Uuid,
    receipt_no: &str,
    product_id: Uuid,
    batch_no: &str,
    idempotency_key: &str,
    now: DateTime<Utc>,
) -> Result<DrugInspectionAcceptanceDecision, Wave3RepositoryError> {
    let category: String = sqlx::query_scalar(
        "SELECT special_drug_category
         FROM products
         WHERE owner_id = $1 AND id = $2",
    )
    .bind(ctx.owner_id)
    .bind(product_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::MissingProduct)?;
    let rule: Option<(Uuid, i64, String)> = sqlx::query_as(
        r#"
        SELECT id, version, missing_behavior
          FROM drug_inspection_requirement_rules
         WHERE owner_id = $1
           AND enabled
           AND special_drug_category IN ($2, '*')
         ORDER BY (special_drug_category = $2) DESC
         LIMIT 1
        "#,
    )
    .bind(ctx.owner_id)
    .bind(&category)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let report: Option<(Uuid, bool)> = sqlx::query_as(
        r#"
        SELECT version.id, version.qualified
          FROM drug_inspection_asn_links link
          JOIN drug_inspection_reports report
            ON report.id = link.report_id
           AND report.owner_id = link.owner_id
           AND report.product_id = $3
           AND report.batch_no = $4
          JOIN drug_inspection_report_versions version
            ON version.id = report.current_version_id
           AND version.owner_id = report.owner_id
           AND version.status = 'confirmed'
         WHERE link.owner_id = $1
           AND link.asn_id = $2
           AND link.batch_no = $4
         LIMIT 1
        "#,
    )
    .bind(ctx.owner_id)
    .bind(receiving_order_id)
    .bind(product_id)
    .bind(batch_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (result, decision, rule_id, rule_version, report_version_id) =
        match (rule.as_ref(), report) {
            (_, Some((version_id, false))) => (
                "unqualified_blocked",
                DrugInspectionAcceptanceDecision::UnqualifiedBlocked(version_id),
                rule.as_ref().map(|value| value.0),
                rule.as_ref().map(|value| value.1),
                Some(version_id),
            ),
            (_, Some((version_id, true))) => (
                "valid",
                DrugInspectionAcceptanceDecision::Continue,
                rule.as_ref().map(|value| value.0),
                rule.as_ref().map(|value| value.1),
                Some(version_id),
            ),
            (None, None) => (
                "not_required",
                DrugInspectionAcceptanceDecision::Continue,
                None,
                None,
                None,
            ),
            (Some((id, version, behavior)), None) if behavior == "warning" => (
                "missing_warning",
                DrugInspectionAcceptanceDecision::Continue,
                Some(*id),
                Some(*version),
                None,
            ),
            (Some((id, version, _)), None) => (
                "missing_blocked",
                DrugInspectionAcceptanceDecision::MissingBlocked,
                Some(*id),
                Some(*version),
                None,
            ),
        };
    sqlx::query(
        r#"
        INSERT INTO drug_inspection_acceptance_validations (
            id, owner_id, receiving_order_id, batch_no, product_id,
            rule_id, rule_version, result, report_version_id,
            idempotency_key, detail, validated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (owner_id, receiving_order_id, batch_no, idempotency_key)
        DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(receiving_order_id)
    .bind(batch_no)
    .bind(product_id)
    .bind(rule_id)
    .bind(rule_version)
    .bind(result)
    .bind(report_version_id)
    .bind(idempotency_key)
    .bind(serde_json::json!({
        "receipt_no": receipt_no,
        "special_drug_category": category,
        "result": result
    }))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(decision)
}

async fn enqueue_drug_inspection_unqualified_liaison(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    receiving_order_id: Uuid,
    receipt_no: &str,
    product_id: Uuid,
    batch_no: &str,
    report_version_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave3RepositoryError> {
    let liaison_no = format!(
        "QL-DI-{}-{}",
        &receiving_order_id.simple().to_string()[..8],
        &report_version_id.simple().to_string()[..8]
    );
    let content = format!(
        "药检单结论不合格：ASN {receipt_no} 批号 {batch_no}，已阻塞验收并触发质量联系单。"
    );
    let order_id: Uuid = sqlx::query_scalar("SELECT md5($1 || ':' || $2::text)::uuid")
        .bind(&liaison_no)
        .bind(report_version_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let approval_id: Uuid = sqlx::query_scalar("SELECT md5('approval:' || $1)::uuid")
        .bind(&liaison_no)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let inventory_batch_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT batch.id
          FROM inventory_batches AS batch
          JOIN products AS product
            ON product.owner_id = batch.owner_id
           AND product.product_code = batch.product_code
         WHERE batch.owner_id = $1
           AND product.id = $2
           AND batch.batch_no = $3
         ORDER BY batch.id
         FOR UPDATE OF batch
        "#,
    )
    .bind(ctx.owner_id)
    .bind(product_id)
    .bind(batch_no)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let business_payload = serde_json::json!({
        "action": "quarantine_inventory_batches",
        "receiving_order_id": receiving_order_id,
        "product_id": product_id,
        "batch_no": batch_no,
        "report_version_id": report_version_id,
        "inventory_batch_ids": inventory_batch_ids,
        "approval_source": "quality_liaison"
    });
    sqlx::query(
        r#"
        INSERT INTO h4_approval_records (
            id, owner_id, scenario, business_ref, dedupe_key, approver_user,
            process_id, callback_path, summary, status, created_at, updated_at
        )
        SELECT $1, $2, 'quality_liaison', $3, $4,
               type.approver_user_id::TEXT, type.approval_template_id,
               $5, $6, 'pending', $7, $7
          FROM quality_liaison_types AS type
         WHERE type.owner_id = $2
           AND type.type_code = 'inbound_unqualified'
           AND type.enabled
        ON CONFLICT (owner_id, scenario, business_ref, dedupe_key) DO NOTHING
        "#,
    )
    .bind(approval_id)
    .bind(ctx.owner_id)
    .bind(order_id.to_string())
    .bind(format!("quality-liaison:{order_id}"))
    .bind(format!(
        "/api/v1/quality-liaisons/{order_id}/approval-callback"
    ))
    .bind(format!("入库药检不合格：{receipt_no} / {batch_no}"))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO quality_liaison_orders (
            id, owner_id, liaison_no, type_code, related_document_type,
            related_document_no, problem_description, disposition_suggestion,
            trigger_source, business_payload, status, approval_record_id, created_by,
            created_at, updated_at
        )
        SELECT
            $2, $1, $3, type.type_code,
            'receiving_order', $4, $5, '隔离同商品同批号现有库存并由质量负责人处置',
            'm-di.acceptance', $6, 'pending_approval', $7, $8, $9, $9
          FROM quality_liaison_types type
         WHERE type.owner_id = $1
           AND type.type_code = 'inbound_unqualified'
           AND type.enabled
        ON CONFLICT (owner_id, liaison_no) DO NOTHING
        "#,
    )
    .bind(ctx.owner_id)
    .bind(order_id)
    .bind(&liaison_no)
    .bind(receipt_no)
    .bind(&content)
    .bind(business_payload)
    .bind(approval_id)
    .bind(ctx.user_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?
    .rows_affected();
    if inserted == 0 {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM quality_liaison_orders
                WHERE owner_id = $1 AND liaison_no = $2
             )",
        )
        .bind(ctx.owner_id)
        .bind(&liaison_no)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if !exists {
            return Err(Wave3RepositoryError::MissingRequiredField(
                "quality_liaison_type.inbound_unqualified".to_string(),
            ));
        }
    }
    Ok(())
}
