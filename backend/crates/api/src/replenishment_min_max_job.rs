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
    next_interval_with(
        now,
        DAY_INTERVAL,
        NIGHT_INTERVAL,
        NaiveTime::from_hms_opt(8, 0, 0).unwrap_or(NaiveTime::MIN),
        NaiveTime::from_hms_opt(20, 0, 0).unwrap_or(NaiveTime::MIN),
    )
}

pub fn next_interval_with(
    now: DateTime<Utc>,
    day_interval: Duration,
    night_interval: Duration,
    day_start: NaiveTime,
    day_end: NaiveTime,
) -> Duration {
    let time = now.time();
    if time >= day_start && time < day_end {
        day_interval
    } else {
        night_interval
    }
}

async fn configured_interval(pool: &PgPool, now: DateTime<Utc>) -> Duration {
    let repo = PgReplenishmentRepository::new(pool.clone());
    let day_minutes = parse_minutes(
        repo.runtime_setting(None, "replenishment.day_interval_minutes")
            .await
            .ok()
            .flatten()
            .as_deref(),
        60,
    );
    let night_minutes = parse_minutes(
        repo.runtime_setting(None, "replenishment.night_interval_minutes")
            .await
            .ok()
            .flatten()
            .as_deref(),
        15,
    );
    let day_start = parse_clock(
        repo.runtime_setting(None, "replenishment.day_start")
            .await
            .ok()
            .flatten()
            .as_deref(),
        8,
        0,
    );
    let day_end = parse_clock(
        repo.runtime_setting(None, "replenishment.day_end")
            .await
            .ok()
            .flatten()
            .as_deref(),
        20,
        0,
    );
    next_interval_with(
        now,
        Duration::from_secs(day_minutes * 60),
        Duration::from_secs(night_minutes * 60),
        day_start,
        day_end,
    )
}

fn parse_minutes(raw: Option<&str>, default: u64) -> u64 {
    raw.and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn parse_clock(raw: Option<&str>, hour: u32, minute: u32) -> NaiveTime {
    raw.and_then(|value| NaiveTime::parse_from_str(value, "%H:%M").ok())
        .unwrap_or_else(|| NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(NaiveTime::MIN))
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
            tokio::time::sleep(configured_interval(&pool, Utc::now()).await).await;
        }
    });
}
