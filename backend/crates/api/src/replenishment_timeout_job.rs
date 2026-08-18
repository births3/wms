//! M-TE 调度骨架：补货超时扫描。每分钟一次，不写入 warehouse_tasks。

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::time::Duration;

use crate::{
    replenishment_repository::PgReplenishmentRepository,
    replenishment_service::ReplenishmentService,
};

pub const JOB_NAME: &str = "replenishment_timeout";

pub fn next_interval() -> Duration {
    Duration::from_secs(60)
}

pub async fn run_once(pool: &PgPool, now: DateTime<Utc>) -> Result<usize, sqlx::Error> {
    ReplenishmentService::new(PgReplenishmentRepository::new(pool.clone()))
        .run_timeout_scan(now)
        .await
        .map_err(|error| sqlx::Error::Protocol(format!("{error:?}")))
}

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(next_interval());
        loop {
            interval.tick().await;
            if let Err(error) = run_once(&pool, Utc::now()).await {
                tracing::error!(?error, job = JOB_NAME, "补货超时扫描失败");
            }
        }
    });
}
