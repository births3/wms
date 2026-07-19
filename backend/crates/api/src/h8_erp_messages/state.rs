//! H8 消息应用状态。

use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use crate::audit::AuditLog;

use super::pg_repository::PgH8ErpMessageRepository;
use super::repository::{H8ErpMessageRepository, MemoryH8ErpMessageRepository};

pub const H8_MSG_READ: &str = "h8.erp_connector.read";
pub const H8_MSG_WRITE: &str = "h8.erp_connector.write";

#[derive(Clone)]
pub struct H8ErpMessageAppState {
    pub repository: Arc<dyn H8ErpMessageRepository>,
    pub audit_pool: Option<PgPool>,
    /// 软件路径可观测审计 sink（始终写入，单测可断言）
    pub audit_log: Arc<Mutex<AuditLog>>,
}

impl H8ErpMessageAppState {
    pub fn with_memory() -> Self {
        Self {
            repository: Arc::new(MemoryH8ErpMessageRepository::default()),
            audit_pool: None,
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
        }
    }

    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgH8ErpMessageRepository::new(pool.clone())),
            audit_pool: Some(pool),
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
        }
    }
}
