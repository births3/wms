//! M-TE 调度骨架：Min-Max 巡检作业。补货任务不写入 warehouse_tasks。

use chrono::{DateTime, NaiveTime, Utc};
use sqlx::PgPool;
use std::time::Duration;

use crate::{
    replenishment_repository::PgReplenishmentRepository,
    replenishment_service::ReplenishmentService,
};

pub const JOB_NAME: &str = "replenishment_min_max";

const DAY_INTERVAL: Duration = Duration::from_secs(60 * 60);
const NIGHT_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub fn next_interval(now: DateTime<Utc>) -> Duration {
    let time = now.time();
    let day_start = NaiveTime::from_hms_opt(8, 0, 0).unwrap_or(NaiveTime::MIN);
    let day_end = NaiveTime::from_hms_opt(20, 0, 0).unwrap_or(NaiveTime::MIN);
    if time >= day_start && time < day_end {
        DAY_INTERVAL
    } else {
        NIGHT_INTERVAL
    }
}

pub async fn run_once(pool: &PgPool, now: DateTime<Utc>) -> Result<usize, sqlx::Error> {
    let created = ReplenishmentService::new(PgReplenishmentRepository::new(pool.clone()))
        .run_min_max_patrol(now)
        .await
        .map_err(|error| sqlx::Error::Protocol(format!("{error:?}")))?;
    Ok(created.len())
}

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        loop {
            let now = Utc::now();
            if let Err(error) = run_once(&pool, now).await {
                tracing::error!(?error, job = JOB_NAME, "补货 Min-Max 巡检失败");
            }
            tokio::time::sleep(next_interval(Utc::now())).await;
        }
    });
}
