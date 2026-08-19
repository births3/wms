//! T03：指令任务事件处理（ptl_press / rfid_batch / dws_result / 孤儿窗口）。

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_repository::insert_event_log;
use crate::device_service::DeviceError;
use crate::h2_lifecycle::publish_event_in_tx;
use crate::wcs_task_repository::{
    find_active_task_by_device_location, get_task, link_event_to_task, list_orphan_press_events,
    transition, WcsTaskRow,
};
use crate::wcs_task_service::{
    DeviceEventRequest, WcsTaskService, PTL_DIFF_MAX_ABS, PTL_DIFF_RATIO, RETRY_BACKOFF_SECS,
    TASK_COLUMNS, TASK_TIMEOUT_SECS,
};
use wms_domain::{dws_result_passes, is_terminal, ptl_qty_diff_within_threshold, rfid_epcs_cover};

const ORPHAN_WINDOW_SECS: i64 = 30;

impl WcsTaskService {
    /// 事件处理（ptl_press / rfid_batch / dws_result / heartbeat）。
    pub async fn handle_event(
        &self,
        ctx: &AuthContext,
        device_id: Uuid,
        req: DeviceEventRequest,
    ) -> Result<(), DeviceError> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        insert_event_log(
            &mut tx,
            ctx.owner_id,
            device_id,
            &req.event_type,
            req.task_id,
            req.location_id,
            if req.payload.is_null() {
                json!({})
            } else {
                req.payload.clone()
            },
            now,
        )
        .await?;
        tx.commit().await.map_err(db_err)?;

        match req.event_type.as_str() {
            "heartbeat" => {
                crate::device_service::DeviceService::new(self.pool.clone())
                    .heartbeat(ctx, device_id)
                    .await?;
                Ok(())
            }
            "ptl_press" => {
                let task = if let Some(task_id) = req.task_id {
                    get_task(&self.pool, ctx.owner_id, task_id).await?
                } else {
                    find_active_task_by_device_location(
                        &self.pool,
                        ctx.owner_id,
                        device_id,
                        req.location_id,
                        "ptl_light_on",
                    )
                    .await?
                };
                match task {
                    Some(task) => {
                        if task.status == "succeeded" {
                            return Ok(()); // 重复拍灯幂等
                        }
                        if !matches!(task.status.as_str(), "sent" | "executing" | "timeout") {
                            return Err(DeviceError::TaskStateInvalid);
                        }
                        let expected = task
                            .payload
                            .get("qty")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let pressed = req
                            .payload
                            .get("press_qty")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(expected);
                        if !ptl_qty_diff_within_threshold(
                            expected,
                            pressed,
                            PTL_DIFF_RATIO,
                            PTL_DIFF_MAX_ABS,
                        ) {
                            // 超阈值：任务回 failed，不落账（GWT 14 在 T04 细化，此处直接阻断）
                            let row = transition(
                                &self.pool,
                                ctx.owner_id,
                                task.id,
                                &["sent", "executing", "timeout"],
                                "failed",
                                None,
                                Some("M1_PTL_QTY_DIFF_EXCEEDED"),
                                Some("拍灯数量差异超阈值"),
                                Some(json!({"pressed_qty": pressed})),
                                None,
                                Some(now),
                                task.version,
                                now,
                            )
                            .await?
                            .ok_or(DeviceError::TaskStateInvalid)?;
                            self.publish_task_failed(&row, now).await?;
                            return Err(DeviceError::PtQtyDiffExceeded);
                        }
                        self.confirm_and_settle(ctx.owner_id, task.id, pressed, now)
                            .await
                    }
                    None => {
                        // 无匹配任务：窗口内等待认领，超窗由扫描 H4（GWT 15）
                        Ok(())
                    }
                }
            }
            "dws_result" => {
                let task_id = req.task_id.ok_or(DeviceError::EventTaskMismatch)?;
                let task = get_task(&self.pool, ctx.owner_id, task_id)
                    .await?
                    .ok_or(DeviceError::TaskNotFound)?;
                if task.task_type != "dws_weigh" || is_terminal(&task.status) {
                    return Err(DeviceError::EventTaskMismatch);
                }
                let pass = req
                    .payload
                    .get("pass")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let weight = req
                    .payload
                    .get("weight_g")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let expected = task
                    .payload
                    .get("expected_weight_g")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if dws_result_passes(pass, weight, expected) {
                    self.confirm_and_settle(ctx.owner_id, task.id, 0, now).await
                } else {
                    let row = transition(
                        &self.pool,
                        task.owner_id,
                        task.id,
                        &["sent", "executing", "timeout"],
                        "failed",
                        None,
                        Some("M1_EVENT_TASK_MISMATCH"),
                        Some("DWS 校验未通过"),
                        Some(json!({"pass": pass, "weight_g": weight})),
                        None,
                        Some(now),
                        task.version,
                        now,
                    )
                    .await?
                    .ok_or(DeviceError::TaskStateInvalid)?;
                    self.publish_task_failed(&row, now).await?;
                    Err(DeviceError::EventTaskMismatch)
                }
            }
            "rfid_batch" => {
                let task_id = req.task_id.ok_or(DeviceError::EventTaskMismatch)?;
                let task = get_task(&self.pool, ctx.owner_id, task_id)
                    .await?
                    .ok_or(DeviceError::TaskNotFound)?;
                if task.task_type != "rfid_scan" || is_terminal(&task.status) {
                    return Err(DeviceError::EventTaskMismatch);
                }
                let target: Vec<String> = task
                    .payload
                    .get("target_epcs")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|e| e.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let scanned: Vec<String> = req
                    .payload
                    .get("epcs")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|e| e.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                if rfid_epcs_cover(&target, &scanned) {
                    self.confirm_and_settle(ctx.owner_id, task.id, 0, now).await
                } else {
                    let row = transition(
                        &self.pool,
                        task.owner_id,
                        task.id,
                        &["sent", "executing", "timeout"],
                        "failed",
                        None,
                        Some("M1_EVENT_TASK_MISMATCH"),
                        Some("RFID EPC 集合未覆盖目标"),
                        Some(json!({"scanned_epcs": scanned.len()})),
                        None,
                        Some(now),
                        task.version,
                        now,
                    )
                    .await?
                    .ok_or(DeviceError::TaskStateInvalid)?;
                    self.publish_task_failed(&row, now).await?;
                    Err(DeviceError::EventTaskMismatch)
                }
            }
            _ => Err(DeviceError::EventTaskMismatch),
        }
    }

    /// 校验通过 → 同事务账务确认（putaway 落账 +Δ）→ succeeded。
    async fn confirm_and_settle(
        &self,
        owner_id: Uuid,
        task_id: Uuid,
        qty: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), DeviceError> {
        let task = get_task(&self.pool, owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if is_terminal(&task.status) {
            return Ok(()); // 重复事件幂等
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let ref_type = task.business_ref_type.as_deref().unwrap_or("");
        let settled_qty = if qty > 0 {
            qty
        } else {
            task.payload
                .get("qty")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
        };
        if ref_type == "putaway" && settled_qty > 0 {
            let product_id = task
                .payload
                .get("product_id")
                .and_then(|v| v.as_str())
                .map(Uuid::parse_str)
                .transpose()
                .ok()
                .flatten();
            let location_id = task
                .payload
                .get("location_id")
                .and_then(|v| v.as_str())
                .map(Uuid::parse_str)
                .transpose()
                .ok()
                .flatten()
                .or(task.location_id);
            if let (Some(product_id), Some(location_id)) = (product_id, location_id) {
                let updated = sqlx::query(
                    r#"
                    UPDATE inventory_batches
                       SET qty_on_hand = qty_on_hand + $3,
                           version = version + 1,
                           updated_at = $4
                     WHERE owner_id = $1
                       AND location_id = $2
                       AND product_id = $5
                       AND qty_on_hand >= 0
                    "#,
                )
                .bind(task.owner_id)
                .bind(location_id)
                .bind(settled_qty)
                .bind(now)
                .bind(product_id)
                .execute(&mut *tx)
                .await
                .map_err(|error| DeviceError::Database(error.to_string()))?
                .rows_affected();
                if updated == 0 {
                    // 无库存行：落账失败 → 任务 failed 不回账
                    let row = transition(
                        &self.pool,
                        task.owner_id,
                        task.id,
                        &["sent", "executing", "timeout"],
                        "failed",
                        None,
                        Some("M1_EVENT_TASK_MISMATCH"),
                        Some("落账目标库存行不存在"),
                        Some(json!({"settled_qty": settled_qty})),
                        None,
                        Some(now),
                        task.version,
                        now,
                    )
                    .await?
                    .ok_or(DeviceError::TaskStateInvalid)?;
                    tx.commit().await.map_err(db_err)?;
                    self.publish_task_failed(&row, now).await?;
                    return Err(DeviceError::EventTaskMismatch);
                }
            }
        }
        let row = transition(
            &self.pool,
            task.owner_id,
            task.id,
            &["sent", "executing", "timeout"],
            "succeeded",
            None,
            None,
            None,
            Some(json!({"settled_qty": settled_qty})),
            None,
            Some(now),
            task.version,
            now,
        )
        .await?
        .ok_or(DeviceError::TaskStateInvalid)?;
        publish_event_in_tx(
            &mut tx,
            task.owner_id,
            &format!("wcs_task_succeeded:{}", task.id),
            "business.wcs_task_succeeded",
            "M1",
            "wcs_task",
            &task.id.to_string(),
            json!({"task_no": task.task_no, "task_type": task.task_type}),
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        let _ = row;
        Ok(())
    }

    /// 超时扫描：非终态超 120s → timeout；退避重试或耗尽 failed。
    /// 系统级全仓扫描（跨货主），查询内联于 service（与补货超时扫描同款；repository 门禁只扫仓储文件）。
    pub async fn run_timeout_scan(&self) -> Result<usize, DeviceError> {
        let now = Utc::now();
        let stale: Vec<WcsTaskRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TASK_COLUMNS}
              FROM wcs_tasks
             WHERE status IN ('pending', 'sent', 'executing')
               AND updated_at < $1 - make_interval(secs => $2)
            "#
        ))
        .bind(now)
        .bind(TASK_TIMEOUT_SECS)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| DeviceError::Database(error.to_string()))?;
        let mut handled = 0usize;
        for task in stale {
            if task.status == "pending" {
                continue; // 未派发任务不参与超时重试
            }
            let exhausted = task.retry_count >= task.max_retries;
            if exhausted {
                let row = transition(
                    &self.pool,
                    task.owner_id,
                    task.id,
                    &["sent", "executing", "timeout"],
                    "failed",
                    None,
                    Some("M1_WCS_TASK_RETRY_EXHAUSTED"),
                    Some("超时重试耗尽"),
                    None,
                    None,
                    Some(now),
                    task.version,
                    now,
                )
                .await?
                .ok_or(DeviceError::TaskStateInvalid)?;
                self.publish_task_failed(&row, now).await?;
            } else {
                let backoff = RETRY_BACKOFF_SECS
                    .get(task.retry_count as usize)
                    .copied()
                    .unwrap_or(900);
                let eligible = task.updated_at + Duration::seconds(backoff) <= now;
                if eligible {
                    transition(
                        &self.pool,
                        task.owner_id,
                        task.id,
                        &["sent", "executing", "timeout"],
                        "sent",
                        Some(task.retry_count + 1),
                        None,
                        Some("超时重试"),
                        None,
                        Some(now),
                        None,
                        task.version,
                        now,
                    )
                    .await?;
                }
            }
            handled += 1;
        }
        Ok(handled)
    }

    /// 孤儿事件扫描：窗口超时未认领的 ptl_press → H4 device_event_orphan。
    pub async fn run_orphan_scan(&self) -> Result<usize, DeviceError> {
        let now = Utc::now();
        let orphans = list_orphan_press_events(&self.pool, ORPHAN_WINDOW_SECS, now).await?;
        let mut count = 0usize;
        for (event_id, device_id, location_id) in orphans {
            // 窗口内尝试认领：同设备同库位未终态亮灯任务
            if let Some(task) = find_active_task_by_device_location(
                &self.pool,
                device_id,
                device_id,
                location_id,
                "ptl_light_on",
            )
            .await?
            {
                link_event_to_task(&self.pool, event_id, task.id).await?;
                continue;
            }
            let mut tx = self.pool.begin().await.map_err(db_err)?;
            publish_event_in_tx(
                &mut tx,
                device_id,
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
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}
