//! T03：指令任务服务层（生成/派发/回执/事件处理/超时重试/人工介入）。

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{append_event_in_tx, AuditWriteRequest};
use crate::auth::AuthContext;
use crate::device_repository::get_device;
use crate::device_service::DeviceError;
use crate::h2_lifecycle::publish_event_in_tx;
use crate::idempotency;
use crate::wcs_task_repository::{
    clear_pod_unreachable_in_tx, find_active_task_by_device_location, find_task_by_idempotency,
    get_task, insert_task, list_tasks, set_pod_unreachable_in_tx, transition, transition_in_tx,
    TaskTransition, WcsTaskRow,
};
use wms_domain::{can_transition, confirm_skip_allowed, is_terminal, resend_allowed};

pub(crate) const TASK_TIMEOUT_SECS: i64 = 120;
pub(crate) const ORPHAN_WINDOW_SECS: i64 = 30;
pub(crate) const PTL_DIFF_RATIO: f64 = 0.2;
pub(crate) const PTL_DIFF_MAX_ABS: i64 = 10;
pub(crate) const RETRY_BACKOFF_SECS: [i64; 3] = [60, 300, 900];
#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct WcsTaskResponse {
    pub id: Uuid,
    pub task_no: String,
    pub task_type: String,
    pub device_id: Uuid,
    pub location_id: Option<Uuid>,
    pub business_ref_type: Option<String>,
    pub business_ref_no: Option<String>,
    pub payload: Value,
    pub status: String,
    pub ack_payload: Value,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_by: String,
    pub version: i64,
}

impl From<WcsTaskRow> for WcsTaskResponse {
    fn from(row: WcsTaskRow) -> Self {
        WcsTaskResponse {
            id: row.id,
            task_no: row.task_no,
            task_type: row.task_type,
            device_id: row.device_id,
            location_id: row.location_id,
            business_ref_type: row.business_ref_type,
            business_ref_no: row.business_ref_no,
            payload: row.payload,
            status: row.status,
            ack_payload: row.ack_payload,
            error_code: row.error_code,
            error_message: row.error_message,
            retry_count: row.retry_count,
            max_retries: row.max_retries,
            created_by: row.created_by,
            version: row.version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateWcsTaskRequest {
    pub task_type: String,
    pub device_id: Uuid,
    #[serde(default)]
    pub location_id: Option<Uuid>,
    #[serde(default)]
    pub business_ref_type: Option<String>,
    #[serde(default)]
    pub business_ref_no: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct DeviceEventRequest {
    pub event_type: String,
    #[serde(default)]
    pub task_id: Option<Uuid>,
    #[serde(default)]
    pub location_id: Option<Uuid>,
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ResendRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct VoidRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ConfirmSkipRequest {
    pub reason: String,
    pub qty: Option<i64>,
}
#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DeviceEventLog {
    pub id: Uuid,
    pub device_id: Uuid,
    pub event_type: String,
    pub task_id: Option<Uuid>,
    pub payload: Value,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DeviceDashboardSummary {
    pub total_devices: i64,
    pub online_devices: i64,
    pub offline_devices: i64,
    pub failed_tasks: i64,
    pub timeout_tasks: i64,
    pub pending_tasks: i64,
    pub affected_location_ids: Vec<Uuid>,
}

pub struct CreatedWcsTask {
    pub task: WcsTaskResponse,
    pub created: bool,
}

impl std::ops::Deref for CreatedWcsTask {
    type Target = WcsTaskResponse;
    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

#[derive(Clone)]
pub struct WcsTaskService {
    pub(crate) pool: PgPool,
}

impl WcsTaskService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 指令生成（幂等）：校验设备可用 → M-CG 编号 → pending 插入。
    pub async fn create_task(
        &self,
        ctx: &AuthContext,
        req: CreateWcsTaskRequest,
        idempotency_key: &str,
    ) -> Result<CreatedWcsTask, DeviceError> {
        let device = get_device(&self.pool, ctx.owner_id, req.device_id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        if !device.enabled {
            return Err(DeviceError::Disabled);
        }
        if req.task_type == "sorter_divert" {
            // §2.2：sorter_divert 仅登记类型不派发
            return Err(DeviceError::TypeInvalid);
        }
        let now = Utc::now();
        let hash = idempotency::request_hash(&req)
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(&mut tx, "wcs_task", ctx.owner_id, idempotency_key)
            .await
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/wcs-tasks",
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?
        {
            return Ok(CreatedWcsTask {
                task: replay,
                created: false,
            });
        }
        if let Some(existing) =
            find_task_by_idempotency(&self.pool, ctx.owner_id, idempotency_key).await?
        {
            return Ok(CreatedWcsTask {
                task: existing.into(),
                created: false,
            });
        }
        if req.task_type == "ptl_light_on"
            && find_active_task_by_device_location(
                &self.pool,
                ctx.owner_id,
                req.device_id,
                req.location_id,
                "ptl_light_on",
                None,
            )
            .await?
            .is_some()
        {
            return Err(DeviceError::PtLightBusy);
        }
        if req.task_type == "pod_move"
            && find_active_task_by_device_location(
                &self.pool,
                ctx.owner_id,
                req.device_id,
                req.location_id,
                "pod_move",
                req.payload.get("pod_code").and_then(|v| v.as_str()),
            )
            .await?
            .is_some()
        {
            return Err(DeviceError::PodMoveActive);
        }
        let task_no = self
            .generate_task_no(&mut tx, ctx, idempotency_key, now)
            .await?;
        let id = Uuid::new_v4();
        insert_task(
            &mut tx,
            id,
            ctx.owner_id,
            &task_no,
            &req.task_type,
            req.device_id,
            req.location_id,
            req.business_ref_type.as_deref(),
            req.business_ref_no.as_deref(),
            if req.payload.is_null() {
                json!({})
            } else {
                req.payload.clone()
            },
            idempotency_key,
            &ctx.actor_name,
            now,
        )
        .await?;
        let response = WcsTaskResponse {
            id,
            task_no,
            task_type: req.task_type.clone(),
            device_id: req.device_id,
            location_id: req.location_id,
            business_ref_type: req.business_ref_type.clone(),
            business_ref_no: req.business_ref_no.clone(),
            payload: if req.payload.is_null() {
                json!({})
            } else {
                req.payload.clone()
            },
            status: "pending".into(),
            ack_payload: json!({}),
            error_code: None,
            error_message: None,
            retry_count: 0,
            max_retries: 3,
            created_by: ctx.actor_name.clone(),
            version: 1,
        };
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/wcs-tasks",
            200,
            "wcs_task",
            &id.to_string(),
            &response,
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "create_wcs_task",
                "M1",
                "wcs_task",
                id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        if req.task_type == "ptl_light_on" {
            self.claim_pending_press(ctx, &response).await?;
        }
        Ok(CreatedWcsTask {
            task: response,
            created: true,
        })
    }

    async fn generate_task_no(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ctx: &AuthContext,
        idempotency_key: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<String, DeviceError> {
        let service = crate::document_numbering::PgDocumentNumberingService::new();
        let mutation = service
            .generate_in_tx(
                tx,
                ctx,
                crate::document_numbering::GenerateDocumentNumberRequest {
                    document_type: "wcs_task".into(),
                    idempotency_key: format!("{idempotency_key}:number"),
                    source_module: "M1".into(),
                    source_document_id: None,
                },
                now,
            )
            .await
            .map_err(|error| match error {
                crate::document_numbering::DocumentNumberingError::RuleNotFound => {
                    DeviceError::NumberingUnavailable
                }
                other => DeviceError::Database(format!("{other:?}")),
            })?;
        Ok(mutation.value.generated_no)
    }

    /// 模拟网关派发：pending → sent（写 sent_at）。
    pub async fn dispatch(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
    ) -> Result<WcsTaskResponse, DeviceError> {
        let now = Utc::now();
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if task.status != "pending" {
            return Err(DeviceError::TaskStateInvalid);
        }
        let row = transition(
            &self.pool,
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
        .ok_or(DeviceError::TaskStateInvalid)?;
        Ok(row.into())
    }

    /// 模拟网关同步回执：start → executing；success → 校验+落账 → succeeded；fail → 重试/耗尽。
    pub async fn apply_receipt(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        outcome: &str,
        error_code: Option<&str>,
    ) -> Result<WcsTaskResponse, DeviceError> {
        let now = Utc::now();
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        match outcome {
            "start" => {
                if !can_transition(&task.status, "executing") {
                    return Err(DeviceError::TaskStateInvalid);
                }
                let mut tx = self.pool.begin().await.map_err(db_err)?;
                if task.task_type == "pod_move" {
                    let pod_code = task
                        .payload
                        .get("pod_code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !pod_code.is_empty() {
                        set_pod_unreachable_in_tx(&mut tx, task.owner_id, pod_code, now).await?;
                    }
                }
                let row = transition_in_tx(
                    &mut tx,
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
                .ok_or(DeviceError::TaskStateInvalid)?;
                tx.commit().await.map_err(db_err)?;
                Ok(row.into())
            }
            "success" => {
                if is_terminal(&task.status) {
                    // I6：终态重复回执幂等忽略，返回当前任务
                    return Ok(task.into());
                }
                if !can_transition(&task.status, "succeeded") {
                    return Err(DeviceError::TaskStateInvalid);
                }
                let mut tx = self.pool.begin().await.map_err(db_err)?;
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
                let row = transition_in_tx(
                    &mut tx,
                    TaskTransition {
                        owner_id: task.owner_id,
                        id: task_id,
                        from_statuses: &["sent", "executing", "timeout"],
                        to: "succeeded",
                        retry_count: None,
                        error_code: None,
                        error_message: None,
                        ack_payload: Some(json!({"outcome": "success"})),
                        sent_at: Some(now),
                        finished_at: None,
                        expected_version: task.version,
                        now,
                    },
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)?;
                tx.commit().await.map_err(db_err)?;
                Ok(row.into())
            }
            "fail" => {
                let exhausted = task.retry_count >= task.max_retries;
                let mut tx = self.pool.begin().await.map_err(db_err)?;
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
                let to = if exhausted { "failed" } else { "sent" };
                let new_retry = if exhausted {
                    task.retry_count
                } else {
                    task.retry_count + 1
                };
                let row = transition_in_tx(
                    &mut tx,
                    TaskTransition {
                        owner_id: task.owner_id,
                        id: task_id,
                        from_statuses: &["sent", "executing", "timeout"],
                        to: to,
                        retry_count: Some(new_retry),
                        error_code: error_code,
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
                tx.commit().await.map_err(db_err)?;
                if exhausted {
                    self.publish_task_failed(&row, now).await?;
                }
                Ok(row.into())
            }
            _ => Err(DeviceError::TaskStateInvalid),
        }
    }

    /// 标记一致性扫描：活跃 pod_move 无标记 / 有标记无活跃任务 → H4 agv_marker_inconsistent。
    pub async fn run_marker_scan(&self) -> Result<usize, DeviceError> {
        let now = Utc::now();
        let active: Vec<WcsTaskRow> = sqlx::query_as(&format!(
            r#"
            SELECT {}
              FROM wcs_tasks
             WHERE task_type = 'pod_move'
               AND status IN ('pending', 'sent', 'executing', 'timeout')
               AND NOT EXISTS (
                    SELECT 1 FROM warehouse_locations
                     WHERE agv_pod_code = wcs_tasks.payload->>'pod_code'
                       AND agv_unreachable_at IS NOT NULL
               )
            "#,
            crate::wcs_task_repository::TASK_COLUMNS
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| DeviceError::Database(error.to_string()))?;
        let marked_without_task: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT agv_pod_code
              FROM warehouse_locations
             WHERE agv_unreachable_at IS NOT NULL
               AND NOT EXISTS (
                    SELECT 1 FROM wcs_tasks
                     WHERE task_type = 'pod_move'
                       AND status IN ('pending', 'sent', 'executing', 'timeout')
                       AND payload->>'pod_code' = warehouse_locations.agv_pod_code
               )
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| DeviceError::Database(error.to_string()))?;
        let total = active.len() + marked_without_task.len();
        if total == 0 {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        publish_event_in_tx(
            &mut tx,
            Uuid::nil(),
            &format!("agv_marker_inconsistent:{}", now.timestamp()),
            "business.agv_marker_inconsistent",
            "M1",
            "wcs_task",
            "scan",
            json!({
                "active_without_marker": active.len(),
                "marked_without_task": marked_without_task.len()
            }),
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(total)
    }

    /// 人工重发（仅 failed / timeout）：重置 retry_count 重新入队（原因记入 ack_payload）。
    pub async fn resend(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        reason: String,
    ) -> Result<WcsTaskResponse, DeviceError> {
        let now = Utc::now();
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if !resend_allowed(&task.status) {
            return Err(DeviceError::TaskStateInvalid);
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = transition_in_tx(
            &mut tx,
            TaskTransition {
                owner_id: task.owner_id,
                id: task_id,
                from_statuses: &["failed", "timeout"],
                to: "sent",
                retry_count: Some(0),
                error_code: None,
                error_message: Some("人工重发"),
                ack_payload: Some(json!({"resend_reason": reason})),
                sent_at: Some(now),
                finished_at: None,
                expected_version: task.version,
                now,
            },
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "resend_wcs_task",
                "M1",
                "wcs_task",
                task_id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(row.into())
    }

    /// 人工作废（仅未落账任务：status != succeeded）。
    pub async fn void(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: VoidRequest,
    ) -> Result<WcsTaskResponse, DeviceError> {
        let now = Utc::now();
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if task.status == "succeeded" {
            return Err(DeviceError::TaskVoidBlocked);
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
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
        let row = transition_in_tx(
            &mut tx,
            TaskTransition {
                owner_id: task.owner_id,
                id: task_id,
                from_statuses: &["pending", "sent", "executing", "timeout", "failed"],
                to: "failed",
                retry_count: None,
                error_code: Some("M1_WCS_TASK_VOID"),
                error_message: Some(&req.reason),
                ack_payload: Some(json!({"voided": true, "reason": req.reason})),
                sent_at: None,
                finished_at: Some(now),
                expected_version: task.version,
                now,
            },
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "void_wcs_task",
                "M1",
                "wcs_task",
                task_id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(row.into())
    }

    pub async fn list(
        &self,
        ctx: &AuthContext,
        status: Option<String>,
        task_type: Option<String>,
    ) -> Result<Vec<WcsTaskResponse>, DeviceError> {
        let rows = list_tasks(
            &self.pool,
            ctx.owner_id,
            status.as_deref(),
            task_type.as_deref(),
        )
        .await?;
        Ok(rows.into_iter().map(WcsTaskResponse::from).collect())
    }

    /// 跳过确认（§10.5）：现场已人工完成，凭 payload 补录账务并置 succeeded（记录操作人）。
    pub async fn confirm_skip(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: crate::wcs_task_service::ConfirmSkipRequest,
    ) -> Result<WcsTaskResponse, DeviceError> {
        let now = Utc::now();
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if !confirm_skip_allowed(&task.status) {
            return Err(DeviceError::TaskStateInvalid);
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        if task.business_ref_type.as_deref() == Some("putaway") {
            let settled_qty = task
                .payload
                .get("qty")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
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
            if settled_qty > 0 {
                if let (Some(product_id), Some(location_id)) = (product_id, location_id) {
                    crate::inventory::confirm_putaway_in_tx(
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
        Ok(row.into())
    }
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}
