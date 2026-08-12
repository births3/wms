//! US-H8-002/003：ERP 消息日志查询、统计与人工重放。

mod audit;
mod error;
mod handlers;
mod lifecycle;
mod payload_repository;
mod pg_lifecycle;
mod pg_repository;
mod pg_rows;
mod repository;
mod runtime_repository;
mod scope;
mod state;

#[cfg(test)]
mod lifecycle_validation_tests;
#[cfg(test)]
mod outbound_lifecycle_reconciliation_tests;
#[cfg(test)]
mod outbound_lifecycle_tests;
#[cfg(test)]
mod partition_tests;
#[cfg(test)]
mod payload_tests;
#[cfg(test)]
mod pg_repository_tests;
#[cfg(test)]
mod replay_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod worker_auth_tests;

pub use handlers::h8_erp_message_router;
pub(crate) use lifecycle::{apply_lifecycle_failure, apply_lifecycle_status};
pub use state::H8ErpMessageAppState;
pub(crate) use state::H8_RECEIPT_WRITE;

/// AC10/16：每小时预建本月/下月分区并清除到期密文。
pub async fn spawn_maintenance_job(pool: sqlx::PgPool) -> Result<(), sqlx::Error> {
    use payload_repository::{H8PayloadRepository, PgH8PayloadRepository};

    ensure_partitions(&pool).await?;
    tokio::spawn(async move {
        let repository = PgH8PayloadRepository::new(pool.clone());
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(error) = ensure_partitions(&pool).await {
                tracing::error!(?error, "H8 月分区预创建失败");
            }
            match repository.purge_expired(chrono::Utc::now()).await {
                Ok(deleted) if deleted > 0 => {
                    tracing::info!(deleted, "H8 到期完整报文密文已清理");
                }
                Ok(_) => {}
                Err(error) => tracing::error!(?error, "H8 到期完整报文清理失败"),
            }
        }
    });
    Ok(())
}

async fn ensure_partitions(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "SELECT h8_erp_messages_ensure_month_partition(\
           (CURRENT_TIMESTAMP AT TIME ZONE 'UTC')::date);\
         SELECT h8_erp_messages_ensure_month_partition(\
           ((CURRENT_TIMESTAMP AT TIME ZONE 'UTC') + INTERVAL '1 month')::date);",
    )
    .execute(pool)
    .await?;
    Ok(())
}
