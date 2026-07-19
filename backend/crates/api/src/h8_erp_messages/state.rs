//! H8 消息应用状态。

use std::sync::Arc;

use sqlx::PgPool;

use super::pg_repository::PgH8ErpMessageRepository;
use super::repository::{H8ErpMessageRepository, MemoryH8ErpMessageRepository};

pub const H8_MSG_READ: &str = "h8.erp_connector.read";
pub const H8_MSG_WRITE: &str = "h8.erp_connector.write";

#[derive(Clone)]
pub struct H8ErpMessageAppState {
    pub repository: Arc<dyn H8ErpMessageRepository>,
}

impl H8ErpMessageAppState {
    pub fn with_memory() -> Self {
        Self {
            repository: Arc::new(MemoryH8ErpMessageRepository::default()),
        }
    }

    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgH8ErpMessageRepository::new(pool)),
        }
    }
}
