use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};

const SCHEDULER_ACTOR: &str = "system-scheduler";

#[derive(Debug)]
pub enum InventoryExpiryJobError {
    Database(sqlx::Error),
    Repository(Wave3RepositoryError),
}

pub async fn run_once(pool: &PgPool, now: DateTime<Utc>) -> Result<usize, InventoryExpiryJobError> {
    let as_of = now.date_naive();
    let owner_ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT DISTINCT owner_id
          FROM inventory_batches
         WHERE expiry_date <= $1
           AND quality_status = $2
         ORDER BY owner_id
        "#,
    )
    .bind(as_of)
    .bind(STATUS_QUALIFIED)
    .fetch_all(pool)
    .await
    .map_err(InventoryExpiryJobError::Database)?;

    let repository = PgWave3Repository::new(pool.clone());
    let mut isolated = 0;
    for owner_id in owner_ids {
        let context = AuthContext {
            user_id: Uuid::nil(),
            owner_id,
            actor_name: SCHEDULER_ACTOR.to_string(),
            permissions: vec!["m3.write".to_string()],
            jti: format!("{SCHEDULER_ACTOR}:m3-expiry:{owner_id}:{as_of}"),
            warehouse_scope: None,
        };
        let idempotency_key = format!("m3-expiry-scheduler:{owner_id}:{as_of}");
        let result = repository
            .isolate_expired_inventory_batches(&context, as_of, now, &idempotency_key, None)
            .await
            .map_err(InventoryExpiryJobError::Repository)?;
        isolated += result.value.len();
        // 近效期预警事件（供 H4 / 看板消费）
        let _ = repository
            .generate_near_expiry_alerts(&context, now, None)
            .await;
        // 状态 ERP 反馈 outbox 重试
        let _ = repository.process_status_erp_feedback_outbox(now, 50).await;
    }
    Ok(isolated)
}

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(error) = run_once(&pool, Utc::now()).await {
                tracing::error!(?error, "M3 过期批次自动隔离调度失败");
            }
        }
    });
}
