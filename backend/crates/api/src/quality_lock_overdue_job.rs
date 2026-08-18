//! 容器质量锁 lock_move 超时未移库巡检（每 5 分钟）：
//! 复用 PgLpnContainerRepository::scan_overdue_lock_moves 扫描超过 2 小时未完成的
//! 隔离移库任务，向 H4 通知记录写入质量告警（企微收件人 warehouse_manager）。

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::lpn_container_repository::PgLpnContainerRepository;

const DEFAULT_THRESHOLD_HOURS: i64 = 2;

#[derive(Debug)]
pub enum QualityLockOverdueJobError {
    Database(sqlx::Error),
    Repository(crate::lpn_container_repository::LpnContainerRepositoryError),
}

/// 单轮扫描：对存在 lock_move 任务的货主逐一扫描，写入告警（同容器幂等去重）。
pub async fn run_once(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<usize, QualityLockOverdueJobError> {
    let repository = PgLpnContainerRepository::new(pool.clone());
    let owner_ids = repository
        .list_lock_move_owner_ids()
        .await
        .map_err(QualityLockOverdueJobError::Repository)?;

    let mut alerted = 0;
    for owner_id in owner_ids {
        let overdue = repository
            .scan_overdue_lock_moves(owner_id, DEFAULT_THRESHOLD_HOURS, now)
            .await
            .map_err(QualityLockOverdueJobError::Repository)?;
        for item in overdue {
            if repository
                .insert_overdue_lock_move_alert(owner_id, &item, DEFAULT_THRESHOLD_HOURS, now)
                .await
                .map_err(QualityLockOverdueJobError::Repository)?
            {
                alerted += 1;
            }
        }
    }
    Ok(alerted)
}

/// 每 5 分钟扫描一次（规格：超时未移库默认 2 小时告警）。
pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 5));
        loop {
            interval.tick().await;
            if let Err(error) = run_once(&pool, Utc::now()).await {
                tracing::error!(?error, "M1 质量锁 lock_move 超时巡检调度失败");
            }
        }
    });
}
