//! US-H8-002/003：ERP 消息日志查询、统计与人工重放。

mod audit;
mod error;
mod handlers;
mod payload_repository;
mod pg_repository;
mod pg_rows;
mod repository;
mod runtime_repository;
mod state;

#[cfg(test)]
mod payload_tests;
#[cfg(test)]
mod pg_repository_tests;
#[cfg(test)]
mod tests;

pub use handlers::h8_erp_message_router;
pub use state::H8ErpMessageAppState;

/// AC16：每小时清除到期密文；消息、尝试和 H2 审计保持不变。
pub fn spawn_payload_expiry_job(pool: sqlx::PgPool) {
    use payload_repository::{H8PayloadRepository, PgH8PayloadRepository};

    tokio::spawn(async move {
        let repository = PgH8PayloadRepository::new(pool);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            match repository.purge_expired(chrono::Utc::now()).await {
                Ok(deleted) if deleted > 0 => {
                    tracing::info!(deleted, "H8 到期完整报文密文已清理");
                }
                Ok(_) => {}
                Err(error) => tracing::error!(?error, "H8 到期完整报文清理失败"),
            }
        }
    });
}
