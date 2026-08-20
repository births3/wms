//! T03：WCS 任务超时扫描、孤儿事件扫描与只读查询。

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_service::DeviceError;
use crate::h2_lifecycle::publish_event_in_tx;
use crate::wcs_task_repository::{
    get_task, list_orphan_press_events, transition, TaskTransition, WcsTaskRow, TASK_COLUMNS,
};
use crate::wcs_task_service::{
    DeviceDashboardSummary, DeviceEventLog, WcsTaskResponse, WcsTaskService, ORPHAN_WINDOW_SECS,
    RETRY_BACKOFF_SECS, TASK_TIMEOUT_SECS,
};

impl WcsTaskService {
    /// 超时扫描：非终态超 120s → timeout；退避重试或耗尽 failed。
    /// 系统级全仓扫描（跨货主），查询内联于 service（与补货超时扫描同款；repository 门禁只扫仓储文件）。
    pub async fn run_timeout_scan(&self) -> Result<usize, DeviceError> {
        let now = Utc::now();
        let stale: Vec<WcsTaskRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TASK_COLUMNS}
              FROM wcs_tasks
             WHERE (
                    status IN ('sent', 'executing')
                    AND updated_at < $1 - make_interval(secs => $2)
               ) OR (
                    status = 'timeout'
                    AND updated_at <= $1 - make_interval(secs => CASE retry_count
                        WHEN 0 THEN $3
                        WHEN 1 THEN $4
                        ELSE $5
                    END)
               )
            "#
        ))
        .bind(now)
        .bind(TASK_TIMEOUT_SECS)
        .bind(RETRY_BACKOFF_SECS[0])
        .bind(RETRY_BACKOFF_SECS[1])
        .bind(RETRY_BACKOFF_SECS[2])
        .fetch_all(&self.pool)
        .await
        .map_err(|error| DeviceError::Database(error.to_string()))?;
        let mut handled = 0usize;
        for task in stale {
            let device_enabled: bool =
                sqlx::query_scalar("SELECT enabled FROM iot_devices WHERE id = $1")
                    .bind(task.device_id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|error| DeviceError::Database(error.to_string()))?
                    .unwrap_or(false);
            if !device_enabled {
                continue; // I10：停用设备停止重试，停滞告警由下方 stalled 扫描发出
            }
            // §10.5：先置 timeout 中间态，再按退避与重试计数决定去向
            let current = if task.status != "timeout" {
                transition(
                    &self.pool,
                    TaskTransition {
                        owner_id: task.owner_id,
                        id: task.id,
                        from_statuses: &["sent", "executing"],
                        to: "timeout",
                        retry_count: None,
                        error_code: None,
                        error_message: Some("超时未收到终态回执"),
                        ack_payload: None,
                        sent_at: None,
                        finished_at: None,
                        expected_version: task.version,
                        now,
                    },
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)?
            } else {
                task.clone()
            };
            let exhausted = current.retry_count >= current.max_retries;
            if exhausted {
                let row = transition(
                    &self.pool,
                    TaskTransition {
                        owner_id: current.owner_id,
                        id: current.id,
                        from_statuses: &["sent", "executing", "timeout"],
                        to: "failed",
                        retry_count: None,
                        error_code: Some("M1_WCS_TASK_RETRY_EXHAUSTED"),
                        error_message: Some("超时重试耗尽"),
                        ack_payload: None,
                        sent_at: None,
                        finished_at: Some(now),
                        expected_version: current.version,
                        now,
                    },
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)?;
                self.publish_task_failed(&row, now).await?;
            } else {
                let backoff = RETRY_BACKOFF_SECS
                    .get(current.retry_count as usize)
                    .copied()
                    .unwrap_or(900);
                let eligible = current.updated_at + Duration::seconds(backoff) <= now;
                if eligible {
                    transition(
                        &self.pool,
                        TaskTransition {
                            owner_id: current.owner_id,
                            id: current.id,
                            from_statuses: &["sent", "executing", "timeout"],
                            to: "sent",
                            retry_count: Some(current.retry_count + 1),
                            error_code: None,
                            error_message: Some("超时重试"),
                            ack_payload: None,
                            sent_at: Some(now),
                            finished_at: None,
                            expected_version: current.version,
                            now,
                        },
                    )
                    .await?;
                }
            }
            handled += 1;
        }
        // §6.3：设备停用致活跃任务停滞 → H4 wcs_task_stalled
        let stalled: Vec<WcsTaskRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TASK_COLUMNS}
              FROM wcs_tasks
             WHERE status IN ('pending', 'sent', 'executing', 'timeout')
               AND NOT EXISTS (
                    SELECT 1 FROM iot_devices
                     WHERE iot_devices.id = wcs_tasks.device_id
                       AND iot_devices.enabled = TRUE
               )
            "#
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| DeviceError::Database(error.to_string()))?;
        for task in &stalled {
            let mut tx = self.pool.begin().await.map_err(db_err)?;
            publish_event_in_tx(
                &mut tx,
                task.owner_id,
                &format!("wcs_task_stalled:{}", task.id),
                "business.wcs_task_stalled",
                "M1",
                "wcs_task",
                &task.id.to_string(),
                json!({"task_no": task.task_no, "stalled_minutes": 5}),
                now,
            )
            .await
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
            tx.commit().await.map_err(db_err)?;
        }
        Ok(handled + stalled.len())
    }

    /// 孤儿事件扫描：窗口超时未认领的 ptl_press → H4 device_event_orphan（系统级，owner=nil）。
    pub async fn run_orphan_scan(&self) -> Result<usize, DeviceError> {
        let now = Utc::now();
        let orphans = list_orphan_press_events(&self.pool, ORPHAN_WINDOW_SECS, now).await?;
        let mut count = 0usize;
        for (event_id, device_id, _location_id) in orphans {
            let mut tx = self.pool.begin().await.map_err(db_err)?;
            publish_event_in_tx(
                &mut tx,
                Uuid::nil(),
                &format!("device_event_orphan:{event_id}"),
                "business.device_event_orphan",
                "M1",
                "iot_event_log",
                &event_id.to_string(),
                json!({"event_type": "ptl_press", "device_id": device_id}),
                now,
            )
            .await
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
            tx.commit().await.map_err(db_err)?;
            count += 1;
        }
        Ok(count)
    }

    pub(crate) async fn publish_task_failed(
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

    pub async fn list_events(
        &self,
        warehouse_id: Uuid,
        device_id: Option<Uuid>,
        event_type: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<DeviceEventLog>, DeviceError> {
        let rows = crate::wcs_task_repository::list_events(
            &self.pool,
            warehouse_id,
            device_id,
            event_type.as_deref(),
            limit.unwrap_or(50).clamp(1, 500),
        )
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, device_id, event_type, task_id, payload, received_at)| DeviceEventLog {
                    id,
                    device_id,
                    event_type,
                    task_id,
                    payload,
                    received_at,
                },
            )
            .collect())
    }

    pub async fn dashboard_summary(
        &self,
        ctx: &AuthContext,
        warehouse_id: Uuid,
    ) -> Result<DeviceDashboardSummary, DeviceError> {
        let (total, online, offline, failed, timeout, pending) =
            crate::wcs_task_repository::device_dashboard_summary(
                &self.pool,
                warehouse_id,
                ctx.owner_id,
            )
            .await?;
        let affected_location_ids =
            crate::wcs_task_repository::list_affected_location_ids(&self.pool, ctx.owner_id)
                .await?;
        Ok(DeviceDashboardSummary {
            total_devices: total,
            online_devices: online,
            offline_devices: offline,
            failed_tasks: failed,
            timeout_tasks: timeout,
            pending_tasks: pending,
            affected_location_ids,
        })
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
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}
