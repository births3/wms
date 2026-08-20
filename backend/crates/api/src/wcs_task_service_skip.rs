//! 跳过确认（规格 §10.5）。

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::audit::{append_event_in_tx, AuditWriteRequest};
use crate::auth::AuthContext;
use crate::device_service::DeviceError;
use crate::h2_lifecycle::publish_event_in_tx;
use crate::idempotency;
use crate::wcs_task_repository::{
    clear_pod_unreachable_in_tx, get_task, location_is_unreachable, transition_in_tx,
    TaskTransition, WcsTaskRow,
};
use crate::wcs_task_service::{
    idempotency_err, ConfirmSkipRequest, WcsTaskResponse, WcsTaskService,
};
use wms_domain::{confirm_skip_allowed, ErrorResponse};

impl WcsTaskService {
    /// 跳过确认（§10.5）：现场已人工完成，凭证据补录账务并置 succeeded。
    pub async fn confirm_skip(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: ConfirmSkipRequest,
        idempotency_key: &str,
    ) -> Result<WcsTaskResponse, DeviceError> {
        self.ensure_task_warehouse_access(ctx, task_id).await?;
        let now = Utc::now();
        let path = format!("/api/v1/wcs-tasks/{task_id}/confirm-skip");
        let hash = idempotency::request_hash(&serde_json::json!({
            "task_id": task_id,
            "request": &req,
        }))
        .map_err(idempotency_err)?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(
            &mut tx,
            "wcs_task_confirm_skip",
            ctx.owner_id,
            idempotency_key,
        )
        .await
        .map_err(idempotency_err)?;
        if let Some((status_code, response_body)) = idempotency::replay_value(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            now,
        )
        .await
        .map_err(idempotency_err)?
        {
            if status_code == 422
                && response_body.get("code").and_then(|value| value.as_str())
                    == Some("M1_EVENT_TASK_MISMATCH")
            {
                return Err(DeviceError::EventTaskMismatch);
            }
            return serde_json::from_value(response_body)
                .map_err(|error| DeviceError::Database(error.to_string()));
        }
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if !confirm_skip_allowed(&task.status) {
            return Err(DeviceError::TaskStateInvalid);
        }
        let ref_type = task.business_ref_type.as_deref().unwrap_or("");
        if matches!(ref_type, "putaway" | "replenish") {
            if let Some(location_id) = task.location_id {
                if location_is_unreachable(&self.pool, task.owner_id, location_id).await? {
                    return Err(DeviceError::LocationUnreachable);
                }
            }
        }
        if task.task_type == "pod_move" {
            let pod_code = task
                .payload
                .get("pod_code")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !pod_code.is_empty() {
                clear_pod_unreachable_in_tx(&mut tx, task.owner_id, pod_code, now).await?;
            }
        }
        let settled_qty = req
            .qty
            .or_else(|| task.payload.get("qty").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        if ref_type == "replenish" {
            let source_batch_id = task
                .payload
                .get("source_batch_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            let target_batch_id = task
                .payload
                .get("target_batch_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            match (settled_qty > 0, source_batch_id, target_batch_id) {
                (true, Some(source_batch_id), Some(target_batch_id)) => {
                    crate::inventory::confirm_replenish_in_tx(
                        &mut tx,
                        task.owner_id,
                        source_batch_id,
                        target_batch_id,
                        wms_domain::Quantity::from(settled_qty),
                        task.id,
                        "device_platform_skip",
                        &ctx.actor_name,
                        now,
                    )
                    .await
                    .map_err(|error| match error {
                        crate::inventory::InventoryReplenishError::LocationUnreachable => {
                            DeviceError::LocationUnreachable
                        }
                        other => DeviceError::Database(format!("{other:?}")),
                    })?;
                }
                _ => {
                    mark_skip_putaway_failed(
                        &mut tx,
                        ctx,
                        &task,
                        now,
                        "跳过确认缺少补货落账证据",
                        idempotency_key,
                        &hash,
                        &path,
                    )
                    .await?;
                    tx.commit().await.map_err(db_err)?;
                    return Err(DeviceError::EventTaskMismatch);
                }
            }
        }
        if ref_type == "putaway" {
            let product_id = task
                .payload
                .get("product_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            let location_id = task
                .payload
                .get("location_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .or(task.location_id);
            match (settled_qty > 0, product_id, location_id) {
                (true, Some(product_id), Some(location_id)) => {
                    let settled = crate::inventory::confirm_putaway_in_tx(
                        &mut tx,
                        task.owner_id,
                        location_id,
                        product_id,
                        wms_domain::Quantity::from(settled_qty),
                        "wcs_task_skip",
                        task.id,
                        now,
                    )
                    .await
                    .map_err(|error| DeviceError::Database(error.to_string()))?;
                    if !settled {
                        mark_skip_putaway_failed(
                            &mut tx,
                            ctx,
                            &task,
                            now,
                            "跳过确认落账失败",
                            idempotency_key,
                            &hash,
                            &path,
                        )
                        .await?;
                        tx.commit().await.map_err(db_err)?;
                        return Err(DeviceError::EventTaskMismatch);
                    }
                }
                _ => {
                    mark_skip_putaway_failed(
                        &mut tx,
                        ctx,
                        &task,
                        now,
                        "跳过确认缺少落账证据",
                        idempotency_key,
                        &hash,
                        &path,
                    )
                    .await?;
                    tx.commit().await.map_err(db_err)?;
                    return Err(DeviceError::EventTaskMismatch);
                }
            }
        }
        let row = transition_in_tx(
            &mut tx,
            TaskTransition {
                owner_id: task.owner_id,
                id: task.id,
                from_statuses: &["sent", "executing", "timeout", "failed"],
                to: "succeeded",
                retry_count: None,
                error_code: None,
                error_message: None,
                ack_payload: Some(json!({
                    "settled_by_skip": true,
                    "reason": req.reason,
                    "operator_id": ctx.user_id,
                    "operator_name": ctx.actor_name
                })),
                sent_at: None,
                finished_at: Some(now),
                expected_version: task.version,
                now,
            },
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)?;
        let response = WcsTaskResponse::from(row);
        idempotency::store_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            "wcs_task",
            &task_id.to_string(),
            &response,
            now,
        )
        .await
        .map_err(idempotency_err)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "confirm_skip_wcs_task",
                "M1",
                "wcs_task",
                task_id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        self.enqueue_ptl_light_off(ctx, &task).await?;
        Ok(response)
    }
}

#[allow(clippy::too_many_arguments)]
async fn mark_skip_putaway_failed(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    task: &WcsTaskRow,
    now: DateTime<Utc>,
    message: &str,
    idempotency_key: &str,
    request_hash: &str,
    path: &str,
) -> Result<(), DeviceError> {
    let failed = transition_in_tx(
        tx,
        TaskTransition {
            owner_id: task.owner_id,
            id: task.id,
            from_statuses: &["sent", "executing", "timeout", "failed"],
            to: "failed",
            retry_count: None,
            error_code: Some("M1_EVENT_TASK_MISMATCH"),
            error_message: Some(message),
            ack_payload: Some(json!({"settled_by_skip": false})),
            sent_at: None,
            finished_at: Some(now),
            expected_version: task.version,
            now,
        },
    )
    .await?
    .ok_or(DeviceError::TaskStateInvalid)?;
    let error_response = ErrorResponse {
        code: "M1_EVENT_TASK_MISMATCH".into(),
        message: "设备事件与指令任务不匹配".into(),
        severity: "error".into(),
        details: json!({}),
        trace_id: "unavailable".into(),
        retry_hint: None,
    };
    idempotency::store_success_with_status(
        tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        "POST",
        path,
        422,
        "wcs_task",
        &task.id.to_string(),
        &error_response,
        now,
    )
    .await
    .map_err(idempotency_err)?;
    append_event_in_tx(
        tx,
        &AuditWriteRequest::from_auth_context(
            ctx,
            "confirm_skip_wcs_task_failed",
            "M1",
            "wcs_task",
            task.id.to_string(),
            None,
        ),
    )
    .await
    .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
    publish_event_in_tx(
        tx,
        task.owner_id,
        &format!("wcs_task_failed:{}:v{}", failed.id, failed.version),
        "business.wcs_task_failed",
        "M1",
        "wcs_task",
        &task.id.to_string(),
        json!({
            "task_no": failed.task_no,
            "task_type": failed.task_type,
            "retry_count": failed.retry_count,
            "reason": message,
        }),
        now,
    )
    .await
    .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
    Ok(())
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}
