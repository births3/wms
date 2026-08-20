//! T03：受控模拟网关命令（派发 / 同步回执）。

use chrono::Utc;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::audit::{append_event_in_tx, AuditWriteRequest};
use crate::device_repository::get_device_dispatch_status_in_tx;
use crate::device_service::DeviceError;
use crate::h2_lifecycle::publish_event_in_tx;
use crate::idempotency;
use crate::operation_context::OperationContext as AuthContext;
use crate::wcs_task_repository::{
    clear_pod_unreachable_in_tx, get_task_in_tx, set_pod_unreachable_in_tx, transition_in_tx,
    TaskTransition, WcsTaskRow,
};
use crate::wcs_task_service::{idempotency_err, ReceiptRequest, WcsTaskResponse, WcsTaskService};
use wms_domain::{can_transition, is_terminal};

impl WcsTaskService {
    /// 模拟网关派发：pending → sent（写 sent_at）。
    pub async fn dispatch(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
    ) -> Result<WcsTaskResponse, DeviceError> {
        self.ensure_task_warehouse_access(ctx, task_id).await?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = self
            .dispatch_in_tx(&mut tx, ctx.owner_id, task_id, now)
            .await?;
        tx.commit().await.map_err(db_err)?;
        Ok(row.into())
    }

    pub async fn dispatch_command(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        idempotency_key: &str,
    ) -> Result<WcsTaskResponse, DeviceError> {
        self.ensure_task_warehouse_access(ctx, task_id).await?;
        let now = Utc::now();
        let path = format!("/api/v1/wcs-tasks/{task_id}/dispatch");
        let hash =
            idempotency::request_hash(&json!({"task_id": task_id})).map_err(idempotency_err)?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(&mut tx, "wcs_task_dispatch", ctx.owner_id, idempotency_key)
            .await
            .map_err(idempotency_err)?;
        if let Some(replay) = idempotency::replay(
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
            return Ok(replay);
        }
        let row = self
            .dispatch_in_tx(&mut tx, ctx.owner_id, task_id, now)
            .await?;
        let response = WcsTaskResponse::from(row);
        self.store_command(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            &path,
            task_id,
            "dispatch_wcs_task",
            &response,
            now,
        )
        .await?;
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    async fn dispatch_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        task_id: Uuid,
        now: chrono::DateTime<Utc>,
    ) -> Result<WcsTaskRow, DeviceError> {
        let task = get_task_in_tx(tx, owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if task.status != "pending" {
            return Err(DeviceError::TaskStateInvalid);
        }
        let (enabled, online_status) = get_device_dispatch_status_in_tx(tx, task.device_id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        if !enabled {
            return Err(DeviceError::Disabled);
        }
        if online_status != "online" {
            return Err(DeviceError::Offline);
        }
        transition_in_tx(
            tx,
            TaskTransition {
                owner_id: task.owner_id,
                id: task_id,
                from_statuses: &["pending"],
                to: "sent",
                retry_count: None,
                error_code: None,
                error_message: None,
                ack_payload: None,
                sent_at: Some(now),
                finished_at: None,
                expected_version: task.version,
                now,
            },
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)
    }

    /// 模拟网关同步回执：start → executing；success → succeeded；fail → 重试/耗尽。
    pub async fn apply_receipt(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        outcome: &str,
        error_code: Option<&str>,
    ) -> Result<WcsTaskResponse, DeviceError> {
        self.ensure_task_warehouse_access(ctx, task_id).await?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = self
            .apply_receipt_in_tx(&mut tx, ctx.owner_id, task_id, outcome, error_code, now)
            .await?;
        tx.commit().await.map_err(db_err)?;
        Ok(row.into())
    }

    pub async fn apply_receipt_command(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: ReceiptRequest,
        idempotency_key: &str,
    ) -> Result<WcsTaskResponse, DeviceError> {
        self.ensure_task_warehouse_access(ctx, task_id).await?;
        let now = Utc::now();
        let path = format!("/api/v1/wcs-tasks/{task_id}/receipt");
        let hash = idempotency::request_hash(&json!({"task_id": task_id, "request": &req}))
            .map_err(idempotency_err)?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(&mut tx, "wcs_task_receipt", ctx.owner_id, idempotency_key)
            .await
            .map_err(idempotency_err)?;
        if let Some(replay) = idempotency::replay(
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
            return Ok(replay);
        }
        let row = self
            .apply_receipt_in_tx(
                &mut tx,
                ctx.owner_id,
                task_id,
                &req.outcome,
                req.error_code.as_deref(),
                now,
            )
            .await?;
        let response = WcsTaskResponse::from(row);
        self.store_command(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            &path,
            task_id,
            "apply_wcs_receipt",
            &response,
            now,
        )
        .await?;
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    async fn apply_receipt_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        task_id: Uuid,
        outcome: &str,
        error_code: Option<&str>,
        now: chrono::DateTime<Utc>,
    ) -> Result<WcsTaskRow, DeviceError> {
        let task = get_task_in_tx(tx, owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        match outcome {
            "start" => {
                if !can_transition(&task.status, "executing") {
                    return Err(DeviceError::TaskStateInvalid);
                }
                if task.task_type == "pod_move" {
                    let pod_code = task.payload["pod_code"].as_str().unwrap_or("");
                    if !pod_code.is_empty() {
                        set_pod_unreachable_in_tx(tx, task.owner_id, pod_code, now).await?;
                    }
                }
                transition_in_tx(
                    tx,
                    TaskTransition {
                        owner_id: task.owner_id,
                        id: task_id,
                        from_statuses: &["sent", "timeout"],
                        to: "executing",
                        retry_count: None,
                        error_code: None,
                        error_message: None,
                        ack_payload: None,
                        sent_at: None,
                        finished_at: None,
                        expected_version: task.version,
                        now,
                    },
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)
            }
            "success" => {
                if is_terminal(&task.status) {
                    return Ok(task);
                }
                if !matches!(task.task_type.as_str(), "pod_move" | "ptl_light_off") {
                    // PTL / DWS / RFID 必须走对应事件校验，不能用通用成功回执绕过业务规则。
                    return Err(DeviceError::EventTaskMismatch);
                }
                if !can_transition(&task.status, "succeeded") {
                    return Err(DeviceError::TaskStateInvalid);
                }
                if task.task_type == "pod_move" {
                    let pod_code = task.payload["pod_code"].as_str().unwrap_or("");
                    if !pod_code.is_empty() {
                        clear_pod_unreachable_in_tx(tx, task.owner_id, pod_code, now).await?;
                    }
                }
                transition_in_tx(
                    tx,
                    TaskTransition {
                        owner_id: task.owner_id,
                        id: task_id,
                        from_statuses: &["sent", "executing", "timeout"],
                        to: "succeeded",
                        retry_count: None,
                        error_code: None,
                        error_message: None,
                        ack_payload: Some(json!({"outcome": "success"})),
                        sent_at: None,
                        finished_at: Some(now),
                        expected_version: task.version,
                        now,
                    },
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)
            }
            "fail" => self.apply_failure_in_tx(tx, task, error_code, now).await,
            _ => Err(DeviceError::TaskStateInvalid),
        }
    }

    async fn apply_failure_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        task: WcsTaskRow,
        error_code: Option<&str>,
        now: chrono::DateTime<Utc>,
    ) -> Result<WcsTaskRow, DeviceError> {
        let exhausted = task.retry_count + 1 >= task.max_retries;
        if exhausted && task.task_type == "pod_move" {
            let pod_code = task.payload["pod_code"].as_str().unwrap_or("");
            if !pod_code.is_empty() {
                clear_pod_unreachable_in_tx(tx, task.owner_id, pod_code, now).await?;
            }
        }
        let row = transition_in_tx(
            tx,
            TaskTransition {
                owner_id: task.owner_id,
                id: task.id,
                from_statuses: &["sent", "executing", "timeout"],
                to: if exhausted { "failed" } else { "sent" },
                retry_count: Some(task.retry_count + 1),
                error_code,
                error_message: Some("设备侧失败回执"),
                ack_payload: Some(json!({"outcome": "failed", "error_code": error_code})),
                sent_at: if exhausted { None } else { Some(now) },
                finished_at: if exhausted { Some(now) } else { None },
                expected_version: task.version,
                now,
            },
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)?;
        if exhausted {
            publish_event_in_tx(
                tx,
                row.owner_id,
                &format!("wcs_task_failed:{}:v{}", row.id, row.version),
                "business.wcs_task_failed",
                "M1",
                "wcs_task",
                &row.id.to_string(),
                json!({
                    "task_no": row.task_no,
                    "task_type": row.task_type,
                    "retry_count": row.retry_count
                }),
                now,
            )
            .await
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        }
        Ok(row)
    }

    #[allow(clippy::too_many_arguments)]
    async fn store_command(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        idempotency_key: &str,
        hash: &str,
        path: &str,
        task_id: Uuid,
        audit_action: &str,
        response: &WcsTaskResponse,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), DeviceError> {
        idempotency::store_success(
            tx,
            ctx.owner_id,
            idempotency_key,
            hash,
            "POST",
            path,
            "wcs_task",
            &task_id.to_string(),
            response,
            now,
        )
        .await
        .map_err(idempotency_err)?;
        append_event_in_tx(
            tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                audit_action,
                "M1",
                "wcs_task",
                task_id.to_string(),
                None,
            ),
        )
        .await
        .map(|_| ())
        .map_err(|error| DeviceError::Database(format!("{error:?}")))
    }
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}
