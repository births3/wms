//! T03：指令任务事件处理（ptl_press / rfid_batch / dws_result / 孤儿窗口）。

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_repository::insert_event_log;
use crate::device_service::DeviceError;
use crate::h2_lifecycle::publish_event_in_tx;
use crate::wcs_task_repository::{
    find_active_task_by_device_location, get_task, list_orphan_press_events,
    location_is_unreachable, transition, transition_in_tx, TaskTransition, WcsTaskRow,
    TASK_COLUMNS,
};
use crate::wcs_task_service::{
    CreateWcsTaskRequest, DeviceEventRequest, WcsTaskResponse, WcsTaskService, ORPHAN_WINDOW_SECS,
    PTL_DIFF_MAX_ABS, PTL_DIFF_RATIO, RETRY_BACKOFF_SECS, TASK_TIMEOUT_SECS,
};
use wms_domain::{dws_result_passes, is_terminal, ptl_qty_diff_within_threshold, rfid_epcs_cover};

impl WcsTaskService {
    /// 事件处理（ptl_press / rfid_batch / dws_result / heartbeat）。
    pub async fn handle_event(
        &self,
        ctx: &AuthContext,
        device_id: Uuid,
        req: DeviceEventRequest,
    ) -> Result<(), DeviceError> {
        let now = Utc::now();
        if req.event_type == "heartbeat" {
            crate::device_service::DeviceService::new(self.pool.clone())
                .heartbeat(ctx, device_id)
                .await?;
            return Ok(());
        }
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
            "ptl_press" => {
                let task = if let Some(task_id) = req.task_id {
                    // 显式 task_id：校验设备/类型/库位归属（§10.2 事件-任务匹配）
                    let found = get_task(&self.pool, ctx.owner_id, task_id).await?;
                    match found {
                        Some(t)
                            if t.task_type == "ptl_light_on"
                                && t.device_id == device_id
                                && (req.location_id.is_none()
                                    || t.location_id == req.location_id) =>
                        {
                            Some(t)
                        }
                        _ => None,
                    }
                } else {
                    find_active_task_by_device_location(
                        &self.pool,
                        ctx.owner_id,
                        device_id,
                        req.location_id,
                        "ptl_light_on",
                        None,
                    )
                    .await?
                };
                match task {
                    Some(task) => {
                        if task.status == "succeeded" {
                            // 重复拍灯幂等；补发可能失败的灭灯收尾。
                            self.enqueue_ptl_light_off(ctx, &task).await?;
                            return Ok(());
                        }
                        if !wms_domain::retry_allowed(&task.status) {
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
                        if expected != pressed {
                            let mut tx = self.pool.begin().await.map_err(db_err)?;
                            publish_event_in_tx(
                                &mut tx,
                                ctx.owner_id,
                                &format!("ptl_qty_diff:{}", task.id),
                                "business.ptl_qty_diff",
                                "M1",
                                "wcs_task",
                                &task.id.to_string(),
                                json!({
                                    "task_no": task.task_no,
                                    "expected_qty": expected,
                                    "pressed_qty": pressed
                                }),
                                now,
                            )
                            .await
                            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
                            tx.commit().await.map_err(db_err)?;
                        }
                        if !ptl_qty_diff_within_threshold(
                            expected,
                            pressed,
                            PTL_DIFF_RATIO,
                            PTL_DIFF_MAX_ABS,
                        ) {
                            // 超阈值：任务回 failed，不落账（GWT 14 在 T04 细化，此处直接阻断）
                            let row = transition(
                                &self.pool,
                                TaskTransition {
                                    owner_id: ctx.owner_id,
                                    id: task.id,
                                    from_statuses: &["sent", "executing", "timeout"],
                                    to: "failed",
                                    retry_count: None,
                                    error_code: Some("M1_PTL_QTY_DIFF_EXCEEDED"),
                                    error_message: Some("拍灯数量差异超阈值"),
                                    ack_payload: Some(json!({"pressed_qty": pressed})),
                                    sent_at: None,
                                    finished_at: Some(now),
                                    expected_version: task.version,
                                    now,
                                },
                            )
                            .await?
                            .ok_or(DeviceError::TaskStateInvalid)?;
                            self.publish_task_failed(&row, now).await?;
                            return Err(DeviceError::PtQtyDiffExceeded);
                        }
                        self.confirm_and_settle(ctx, task.id, pressed, now).await
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
                    self.confirm_and_settle(ctx, task.id, 0, now).await
                } else {
                    let row = transition(
                        &self.pool,
                        TaskTransition {
                            owner_id: task.owner_id,
                            id: task.id,
                            from_statuses: &["sent", "executing", "timeout"],
                            to: "failed",
                            retry_count: None,
                            error_code: Some("M1_EVENT_TASK_MISMATCH"),
                            error_message: Some("DWS 校验未通过"),
                            ack_payload: Some(json!({"pass": pass, "weight_g": weight})),
                            sent_at: None,
                            finished_at: Some(now),
                            expected_version: task.version,
                            now,
                        },
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
                    self.confirm_and_settle(ctx, task.id, 0, now).await
                } else {
                    let row = transition(
                        &self.pool,
                        TaskTransition {
                            owner_id: task.owner_id,
                            id: task.id,
                            from_statuses: &["sent", "executing", "timeout"],
                            to: "failed",
                            retry_count: None,
                            error_code: Some("M1_EVENT_TASK_MISMATCH"),
                            error_message: Some("RFID EPC 集合未覆盖目标"),
                            ack_payload: Some(json!({"scanned_epcs": scanned.len()})),
                            sent_at: None,
                            finished_at: Some(now),
                            expected_version: task.version,
                            now,
                        },
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
    pub(crate) async fn confirm_and_settle(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        qty: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), DeviceError> {
        let task = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        if is_terminal(&task.status) {
            if task.status == "succeeded" {
                self.enqueue_ptl_light_off(ctx, &task).await?;
            }
            return Ok(());
        }
        if let Some(location_id) = task.location_id {
            if location_is_unreachable(&self.pool, task.owner_id, location_id).await? {
                return Err(DeviceError::LocationUnreachable);
            }
        }
        let ref_type = task.business_ref_type.as_deref().unwrap_or("");
        let settled_qty = if qty > 0 {
            qty
        } else {
            task.payload
                .get("qty")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
        };
        // 账务确认与状态推进同一事务（I7）：落账走既有 inventory 上下文命令（§9），
        // 状态推进走 transition_in_tx；任一失败整体回滚，业务账不回。
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        if ref_type == "replenish" && settled_qty > 0 {
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
            if let (Some(source_batch_id), Some(target_batch_id)) =
                (source_batch_id, target_batch_id)
            {
                crate::inventory::confirm_replenish_in_tx(
                    &mut tx,
                    task.owner_id,
                    source_batch_id,
                    target_batch_id,
                    wms_domain::Quantity::from(settled_qty),
                    task.id,
                    "device_platform",
                    "system",
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
        }
        if ref_type == "putaway" && settled_qty > 0 {
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
            match (product_id, location_id) {
                (Some(product_id), Some(location_id)) => {
                    let settled = crate::inventory::confirm_putaway_in_tx(
                        &mut tx,
                        task.owner_id,
                        location_id,
                        product_id,
                        wms_domain::Quantity::from(settled_qty),
                        "wcs_task",
                        task.id,
                        now,
                    )
                    .await
                    .map_err(|error| DeviceError::Database(error.to_string()))?;
                    if !settled {
                        // 无库存行/数量非法：落账失败 → 任务 failed 不回账
                        let row = transition_in_tx(
                            &mut tx,
                            TaskTransition {
                                owner_id: task.owner_id,
                                id: task.id,
                                from_statuses: &["sent", "executing", "timeout"],
                                to: "failed",
                                retry_count: None,
                                error_code: Some("M1_EVENT_TASK_MISMATCH"),
                                error_message: Some("落账目标库存行不存在"),
                                ack_payload: Some(json!({"settled_qty": settled_qty})),
                                sent_at: None,
                                finished_at: Some(now),
                                expected_version: task.version,
                                now,
                            },
                        )
                        .await?
                        .ok_or(DeviceError::TaskStateInvalid)?;
                        tx.commit().await.map_err(db_err)?;
                        self.publish_task_failed(&row, now).await?;
                        return Err(DeviceError::EventTaskMismatch);
                    }
                }
                (None, _) | (_, None) => {
                    // payload 缺少商品/库位：任务 failed 不回账
                    let row = transition_in_tx(
                        &mut tx,
                        TaskTransition {
                            owner_id: task.owner_id,
                            id: task.id,
                            from_statuses: &["sent", "executing", "timeout"],
                            to: "failed",
                            retry_count: None,
                            error_code: Some("M1_EVENT_TASK_MISMATCH"),
                            error_message: Some("落账 payload 缺少 product_id/location_id"),
                            ack_payload: Some(json!({"settled_qty": settled_qty})),
                            sent_at: None,
                            finished_at: Some(now),
                            expected_version: task.version,
                            now,
                        },
                    )
                    .await?
                    .ok_or(DeviceError::TaskStateInvalid)?;
                    tx.commit().await.map_err(db_err)?;
                    self.publish_task_failed(&row, now).await?;
                    return Err(DeviceError::EventTaskMismatch);
                }
            }
        }
        transition_in_tx(
            &mut tx,
            TaskTransition {
                owner_id: task.owner_id,
                id: task.id,
                from_statuses: &["sent", "executing", "timeout"],
                to: "succeeded",
                retry_count: None,
                error_code: None,
                error_message: None,
                ack_payload: Some(json!({"settled_qty": settled_qty})),
                sent_at: None,
                finished_at: Some(now),
                expected_version: task.version,
                now,
            },
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
        self.enqueue_ptl_light_off(ctx, &task).await?;
        Ok(())
    }

    /// 规格 §2.1 / 模型 §6.6：账务确认后下发 `ptl_light_off` 收尾；幂等键按源任务。
    pub(crate) async fn enqueue_ptl_light_off(
        &self,
        ctx: &AuthContext,
        task: &WcsTaskRow,
    ) -> Result<(), DeviceError> {
        if task.task_type != "ptl_light_on" {
            return Ok(());
        }
        // 打破 create_task → confirm_and_settle → enqueue 的 async 递归（E0733）。
        let created = Box::pin(self.create_task(
            ctx,
            CreateWcsTaskRequest {
                task_type: "ptl_light_off".into(),
                device_id: task.device_id,
                location_id: task.location_id,
                business_ref_type: task.business_ref_type.clone(),
                business_ref_no: task.business_ref_no.clone(),
                payload: json!({"closes": task.id}),
            },
            &format!("{}:ptl_light_off", task.id),
        ))
        .await?;
        if created.task.status == "pending" {
            self.dispatch(ctx, created.task.id).await?;
        }
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
    /// 窗口内认领由 handle_event 同步路径完成（按设备+库位匹配活跃任务）；扫描不做任务表 UPDATE
    /// （iot_event_logs 纯审计只 INSERT，wms_app 无 UPDATE 权限）。
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

    pub(crate) async fn claim_pending_press(
        &self,
        ctx: &AuthContext,
        task: &WcsTaskResponse,
    ) -> Result<(), DeviceError> {
        let pending = crate::wcs_task_repository::list_pending_press_in_window(
            &self.pool,
            task.device_id,
            task.location_id,
            ORPHAN_WINDOW_SECS,
            Utc::now(),
        )
        .await?;
        let Some((_event_id, _location_id, payload)) = pending.into_iter().next() else {
            return Ok(());
        };
        let pressed = payload
            .get("press_qty")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        self.dispatch(ctx, task.id).await?;
        self.confirm_and_settle(ctx, task.id, pressed, Utc::now())
            .await
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
        ctx: &AuthContext,
        device_id: Option<Uuid>,
        event_type: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<crate::wcs_task_service::DeviceEventLog>, DeviceError> {
        let rows = crate::wcs_task_repository::list_events(
            &self.pool,
            ctx.owner_id,
            device_id,
            event_type.as_deref(),
            limit.unwrap_or(50).clamp(1, 500),
        )
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, device_id, event_type, task_id, payload, received_at)| {
                    crate::wcs_task_service::DeviceEventLog {
                        id,
                        device_id,
                        event_type,
                        task_id,
                        payload,
                        received_at,
                    }
                },
            )
            .collect())
    }

    pub async fn dashboard_summary(
        &self,
        ctx: &AuthContext,
    ) -> Result<crate::wcs_task_service::DeviceDashboardSummary, DeviceError> {
        let (total, online, offline, failed, timeout, pending) =
            crate::wcs_task_repository::device_dashboard_summary(&self.pool, ctx.owner_id).await?;
        let affected_location_ids =
            crate::wcs_task_repository::list_affected_location_ids(&self.pool, ctx.owner_id)
                .await?;
        Ok(crate::wcs_task_service::DeviceDashboardSummary {
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
    ) -> Result<crate::wcs_task_service::WcsTaskResponse, DeviceError> {
        let row = get_task(&self.pool, ctx.owner_id, task_id)
            .await?
            .ok_or(DeviceError::TaskNotFound)?;
        Ok(row.into())
    }
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}
