//! T02：设备心跳超时扫描（每 30 秒；超 90 秒无心跳置 offline 并写 H4 告警）。
//! 沿既有 api crate 调度骨架模式（replenishment_min_max_job 同款）。

use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

use crate::device_service::DeviceService;

pub fn spawn(pool: PgPool, registry: crate::feature_flags::FeatureFlagRegistry) {
    tokio::spawn(async move {
        let service = DeviceService::new(pool);
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.tick().await; // 首个周期立即执行一次
        loop {
            interval.tick().await;
            match service
                .run_heartbeat_scan_with_timeout(DeviceService::heartbeat_timeout_secs(&registry))
                .await
            {
                Ok(count) => {
                    if count > 0 {
                        info!(device_offline_count = count, "设备心跳超时置离线");
                    }
                }
                Err(error) => {
                    error!(?error, "设备心跳扫描失败");
                }
            }
        }
    });
}
