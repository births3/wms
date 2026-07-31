use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{StockLossReason, SubmitAlertDefinitionChangeRequest};

use crate::{
    alert_definition_repository::{apply_approved_change_in_tx, AlertDefinitionRepositoryError},
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    inventory::STATUS_QUARANTINED,
    operation_context::OperationContext as AuthContext,
    stock_adjustment::quality_liaison::{
        create_approved_stock_loss_order_in_tx, ApprovedStockLossRequest,
    },
    stock_adjustment::StockAdjustmentError,
    wave3_repository::PgWave3Repository,
};

use super::{QualityLiaisonError, QualityLiaisonOrderRow};

pub(super) async fn apply_approved_action_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    liaison: &QualityLiaisonOrderRow,
    now: DateTime<Utc>,
) -> Result<(), QualityLiaisonError> {
    let action = liaison
        .business_payload
        .get("action")
        .and_then(serde_json::Value::as_str);
    if action == Some("apply_alert_definition_change") {
        let change = liaison
            .business_payload
            .get("change")
            .cloned()
            .ok_or(QualityLiaisonError::BusinessActionInvalid)
            .and_then(|value| {
                serde_json::from_value::<SubmitAlertDefinitionChangeRequest>(value)
                    .map_err(|_| QualityLiaisonError::BusinessActionInvalid)
            })?;
        return apply_approved_change_in_tx(tx, ctx, &change, now)
            .await
            .map_err(map_alert_definition_error);
    }
    if action == Some("quarantine_inventory_batches") {
        return quarantine_inventory_batches(tx, ctx, liaison, now).await;
    }
    if action == Some("publish_archive_revision") {
        return publish_archive_revision_in_tx(tx, liaison, now).await;
    }
    if action != Some("create_stock_loss") {
        return Ok(());
    }
    let warehouse_id = payload_uuid(&liaison.business_payload, "warehouse_id")?;
    let batch_id = payload_uuid(&liaison.business_payload, "batch_id")?;
    let quantity = liaison
        .business_payload
        .get("quantity")
        .and_then(serde_json::Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or(QualityLiaisonError::BusinessActionInvalid)?;
    let reason = liaison
        .business_payload
        .get("reason_code")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| StockLossReason::try_from(value).ok())
        .filter(|value| value.is_destruction())
        .ok_or(QualityLiaisonError::BusinessActionInvalid)?;
    let recall_id = liaison
        .business_payload
        .get("recall_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    create_approved_stock_loss_order_in_tx(
        tx,
        ctx,
        ApprovedStockLossRequest {
            warehouse_id,
            batch_id,
            quantity,
            reason,
            recall_id,
            quality_liaison_id: liaison.id,
        },
        now,
    )
    .await
    .map(|_| ())
    .map_err(|error| match error {
        StockAdjustmentError::InvalidRequest
        | StockAdjustmentError::NotFound
        | StockAdjustmentError::QuantityExceeded => QualityLiaisonError::BusinessActionInvalid,
        _ => QualityLiaisonError::BusinessAction(format!("{error:?}")),
    })
}

async fn quarantine_inventory_batches(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    liaison: &QualityLiaisonOrderRow,
    now: DateTime<Utc>,
) -> Result<(), QualityLiaisonError> {
    let batch_ids = liaison
        .business_payload
        .get("inventory_batch_ids")
        .and_then(serde_json::Value::as_array)
        .ok_or(QualityLiaisonError::BusinessActionInvalid)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or(QualityLiaisonError::BusinessActionInvalid)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if batch_ids.is_empty() {
        return Ok(());
    }
    type InventoryStatusRow = (Uuid, String, String, String, Uuid, i64);
    let batches = sqlx::query_as::<_, InventoryStatusRow>(
        r#"
        SELECT id, product_code, batch_no, quality_status, location_id, qty_on_hand
          FROM inventory_batches
         WHERE owner_id = $1 AND id = ANY($2)
         ORDER BY id
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(&batch_ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_business_database_error)?;
    if batches.len() != batch_ids.len() {
        return Err(QualityLiaisonError::BusinessActionInvalid);
    }
    let target_enabled = crate::system_dictionary::effective_item_enabled_in_tx(
        tx,
        ctx.owner_id,
        "inventory_quality_status",
        STATUS_QUARANTINED,
        now,
    )
    .await
    .map_err(map_business_database_error)?;
    if !target_enabled {
        return Err(QualityLiaisonError::BusinessActionInvalid);
    }
    for (batch_id, product_code, batch_no, from_status, location_id, qty_on_hand) in batches {
        if from_status == STATUS_QUARANTINED {
            continue;
        }
        let allowed = crate::inventory_status_config::is_transition_allowed_in_tx(
            tx,
            ctx.owner_id,
            &from_status,
            STATUS_QUARANTINED,
            "quality_liaison",
        )
        .await
        .map_err(map_business_database_error)?;
        if !allowed {
            return Err(QualityLiaisonError::BusinessActionInvalid);
        }
        let status_change_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO inventory_status_changes (
                id, owner_id, batch_id, from_status, to_status,
                reason, approval_source, approval_id, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'quality_liaison', $7, $8)
            "#,
        )
        .bind(status_change_id)
        .bind(ctx.owner_id)
        .bind(batch_id)
        .bind(&from_status)
        .bind(STATUS_QUARANTINED)
        .bind("药检结论不合格，质量联系单审批后隔离")
        .bind(liaison.id.to_string())
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_business_database_error)?;
        sqlx::query(
            "UPDATE inventory_batches
             SET quality_status = $3, updated_at = $4, version = version + 1
             WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .bind(STATUS_QUARANTINED)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_business_database_error)?;
        let warehouse_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT warehouse_id
             FROM warehouse_locations
             WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(location_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_business_database_error)?;
        PgWave3Repository::enqueue_status_erp_feedback_in_tx(
            tx,
            ctx.owner_id,
            warehouse_id,
            batch_id,
            Some(status_change_id),
            &from_status,
            STATUS_QUARANTINED,
            &product_code,
            &batch_no,
            qty_on_hand,
            "药检结论不合格，质量联系单审批后隔离",
            now,
        )
        .await
        .map_err(|error| QualityLiaisonError::BusinessAction(format!("{error:?}")))?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "m3.inventory_status.changed_by_quality_liaison",
            "M3",
            "inventory_batch",
            batch_id.to_string(),
            Some(AuditDiff::compute(
                serde_json::json!({ "quality_status": from_status }),
                serde_json::json!({
                    "quality_status": STATUS_QUARANTINED,
                    "approval_source": "quality_liaison",
                    "approval_id": liaison.id,
                }),
            )),
        );
        audit.occurred_at = now;
        append_event_in_tx(tx, &audit)
            .await
            .map_err(|error| QualityLiaisonError::Audit(format!("{error:?}")))?;
    }
    Ok(())
}

async fn publish_archive_revision_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    liaison: &QualityLiaisonOrderRow,
    now: DateTime<Utc>,
) -> Result<(), QualityLiaisonError> {
    if liaison.type_code != "archive_revision" {
        return Err(QualityLiaisonError::BusinessActionInvalid);
    }
    let warehouse_id = payload_uuid(&liaison.business_payload, "warehouse_id")?;
    let asn_id = payload_uuid(&liaison.business_payload, "asn_id")?;
    let receipt_record_id = payload_uuid(&liaison.business_payload, "receipt_record_id")?;
    let product_code = payload_text(&liaison.business_payload, "product_code")?;
    let field_name = payload_text(&liaison.business_payload, "field_name")?;
    let new_value = payload_text(&liaison.business_payload, "new_value")?;
    let source_valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1
              FROM receiving_orders receiving_order
              JOIN receiving_order_receipts receipt
                ON receipt.receiving_order_id = receiving_order.id
               AND receipt.owner_id = receiving_order.owner_id
              JOIN receiving_order_lines line
                ON line.receiving_order_id = receiving_order.id
               AND line.owner_id = receiving_order.owner_id
             WHERE receiving_order.owner_id = $1
               AND receiving_order.id = $2
               AND receipt.id = $3
               AND receiving_order.warehouse_id = $4
               AND receiving_order.receipt_no = $5
               AND receiving_order.status = 'archive_replenishing'
               AND line.product_code = $6
        )
        "#,
    )
    .bind(liaison.owner_id)
    .bind(asn_id)
    .bind(receipt_record_id)
    .bind(warehouse_id)
    .bind(&liaison.related_document_no)
    .bind(product_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(super::persistence::map_database_error)?;
    if liaison.related_document_type != "asn" || !source_valid {
        return Err(QualityLiaisonError::BusinessActionInvalid);
    }
    let photos = liaison
        .business_payload
        .get("photo_evidence_urls")
        .and_then(serde_json::Value::as_array)
        .filter(|values| {
            (1..=5).contains(&values.len())
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
        })
        .ok_or(QualityLiaisonError::BusinessActionInvalid)?;
    let payload = serde_json::json!({
        "warehouse_id": warehouse_id,
        "liaison_id": liaison.id,
        "liaison_no": liaison.liaison_no,
        "asn_id": asn_id,
        "asn_no": liaison.related_document_no,
        "receipt_record_id": receipt_record_id,
        "product_code": product_code,
        "field_name": field_name,
        "current_value": liaison.business_payload.get("current_value"),
        "new_value": new_value,
        "photo_evidence_urls": photos,
        "approved_at": now,
    });
    sqlx::query(
        r#"
        INSERT INTO archive_revision_erp_feedback_outbox (
            id, owner_id, liaison_id, asn_id, receipt_record_id,
            product_code, field_name, payload, deadline_at, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9 + interval '24 hours',$9,$9)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(liaison.owner_id)
    .bind(liaison.id)
    .bind(asn_id)
    .bind(receipt_record_id)
    .bind(product_code)
    .bind(field_name)
    .bind(payload)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(super::persistence::map_database_error)?;
    Ok(())
}

fn map_alert_definition_error(error: AlertDefinitionRepositoryError) -> QualityLiaisonError {
    match error {
        AlertDefinitionRepositoryError::Database(_) | AlertDefinitionRepositoryError::Audit(_) => {
            QualityLiaisonError::BusinessAction(format!("{error:?}"))
        }
        _ => QualityLiaisonError::BusinessActionInvalid,
    }
}

fn map_business_database_error(error: sqlx::Error) -> QualityLiaisonError {
    QualityLiaisonError::BusinessAction(error.to_string())
}

fn payload_uuid(payload: &serde_json::Value, key: &str) -> Result<Uuid, QualityLiaisonError> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or(QualityLiaisonError::BusinessActionInvalid)
}

fn payload_text<'a>(
    payload: &'a serde_json::Value,
    key: &str,
) -> Result<&'a str, QualityLiaisonError> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(QualityLiaisonError::BusinessActionInvalid)
}
