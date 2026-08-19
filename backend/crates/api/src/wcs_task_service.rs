//! T03：指令任务服务层（生成/派发/回执/事件处理/超时重试/人工介入）。

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_repository::get_device;
use crate::device_service::DeviceError;
use crate::h2_lifecycle::publish_event_in_tx;
use crate::idempotency;
use crate::wcs_task_repository::{
    find_active_task_by_device_location, find_task_by_idempotency, get_task, insert_task,
    list_tasks, transition, WcsTaskRow,
};
use wms_domain::can_transition;

pub(crate) const TASK_TIMEOUT_SECS: i64 = 120;
const ORPHAN_WINDOW_SECS: i64 = 30;
pub(crate) const PTL_DIFF_RATIO: f64 = 0.2;
pub(crate) const PTL_DIFF_MAX_ABS: i64 = 10;
pub(crate) const RETRY_BACKOFF_SECS: [i64; 3] = [60, 300, 900];
pub(crate) const TASK_COLUMNS: &str = "id, owner_id, task_no, task_type, device_id, location_id, \
     business_ref_type, business_ref_no, payload, status, ack_payload, error_code, \
     error_message, retry_count, max_retries, idempotency_key, sent_at, finished_at, \
     created_by, version, updated_at";

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceEventRequest {
    pub event_type: String,
    #[serde(default)]
    pub task_id: Option<Uuid>,
    #[serde(default)]
    pub location_id: Option<Uuid>,
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResendRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoidRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfirmSkipRequest {
    pub reason: String,
    pub qty: Option<i64>,
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
    ) -> Result<WcsTaskResponse, DeviceError> {
        let device = get_device(&self.pool, req.device_id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        if !device.enabled {
            return Err(DeviceError::Disabled);
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
            return Ok(replay);
        }
        if let Some(existing) =
            find_task_by_idempotency(&self.pool, ctx.owner_id, idempotency_key).await?
        {
            return Ok(existing.into());
        }
        if req.task_type == "ptl_light_on"
            && find_active_task_by_device_location(
                &self.pool,
                ctx.owner_id,
                req.device_id,
                req.location_id,
                "ptl_light_on",
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
        tx.commit().await.map_err(db_err)?;
        Ok(response)
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
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
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
            task.owner_id,
            task_id,
            &["pending"],
            "sent",
            None,
            None,
            None,
            None,
            Some(now),
            None,
            task.version,
            now,
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
                let row = transition(
                    &self.pool,
                    task.owner_id,
                    task_id,
                    &["sent", "timeout"],
                    "executing",
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    task.version,
                    now,
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)?;
                Ok(row.into())
            }
            "success" => {
                if !can_transition(&task.status, "succeeded") {
                    return Err(DeviceError::TaskStateInvalid);
                }
                let row = transition(
                    &self.pool,
                    task.owner_id,
                    task_id,
                    &["sent", "executing", "timeout"],
                    "succeeded",
                    None,
                    None,
                    None,
                    Some(json!({"outcome": "success"})),
                    Some(now),
                    None,
                    task.version,
                    now,
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)?;
                Ok(row.into())
            }
            "fail" => {
                let exhausted = task.retry_count >= task.max_retries;
                let to = if exhausted { "failed" } else { "sent" };
                let new_retry = if exhausted {
                    task.retry_count
                } else {
                    task.retry_count + 1
                };
                let row = transition(
                    &self.pool,
                    task.owner_id,
                    task_id,
                    &["sent", "executing", "timeout"],
                    to,
                    Some(new_retry),
                    error_code,
                    Some("设备侧失败回执"),
                    Some(json!({"outcome": "failed", "error_code": error_code})),
                    if exhausted { None } else { Some(now) },
                    if exhausted { Some(now) } else { None },
                    task.version,
                    now,
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)?;
                if exhausted {
                    self.publish_task_failed(&row, now).await?;
                }
                Ok(row.into())
            }
            _ => Err(DeviceError::TaskStateInvalid),
        }
    }

    /// 人工重发（仅 failed / timeout）：重置 retry_count 重新入队。
    pub async fn resend(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
    ) -> Result<WcsTaskResponse, DeviceError> {
        let now = Utc::now();
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if !matches!(task.status.as_str(), "failed" | "timeout") {
            return Err(DeviceError::TaskStateInvalid);
        }
        let row = transition(
            &self.pool,
            task.owner_id,
            task_id,
            &["failed", "timeout"],
            "sent",
            Some(0),
            None,
            Some("人工重发"),
            None,
            Some(now),
            None,
            task.version,
            now,
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)?;
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
        let row = transition(
            &self.pool,
            task.owner_id,
            task_id,
            &["pending", "sent", "executing", "timeout", "failed"],
            "failed",
            None,
            Some("M1_WCS_TASK_VOID"),
            Some(&req.reason),
            Some(json!({"voided": true, "reason": req.reason})),
            None,
            Some(now),
            task.version,
            now,
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)?;
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

    pub async fn get(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
    ) -> Result<WcsTaskResponse, DeviceError> {
        let row = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        Ok(row.into())
    }

    pub(super) async fn publish_task_failed(
        &self,
        task: &WcsTaskRow,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), DeviceError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        publish_event_in_tx(
            &mut tx,
            task.owner_id,
            &format!("wcs_task_failed:{}", task.id),
            "business.wcs_task_failed",
            "M1",
            "wcs_task",
            &task.id.to_string(),
            json!({
                "task_no": task.task_no,
                "task_type": task.task_type,
                "retry_count": task.retry_count
            }),
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)
    }
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}
