//! T03：指令任务超时/重试扫描与孤儿事件窗口（每 60 秒）。
//! 沿既有 api crate 调度骨架模式（replenishment_timeout_job 同款）。

use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

use crate::wcs_task_service::WcsTaskService;

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let service = WcsTaskService::new(pool);
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.tick().await; // 首个周期立即执行一次
        loop {
            interval.tick().await;
            match service.run_timeout_scan().await {
                Ok(count) => {
                    if count > 0 {
                        info!(handled_task_count = count, "指令任务超时扫描完成");
                    }
                }
                Err(error) => {
                    error!(?error, "指令任务超时扫描失败");
                }
            }
            match service.run_orphan_scan().await {
                Ok(count) => {
                    if count > 0 {
                        info!(orphan_event_count = count, "孤儿事件窗口扫描完成");
                    }
                }
                Err(error) => {
                    error!(?error, "孤儿事件扫描失败");
                }
            }
        }
    });
}
