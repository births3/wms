//! H8 ERP 连接应用状态。

use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::repository::{
    H8ErpConnectorRepository, MemoryH8ErpConnectorRepository, PgH8ErpConnectorRepository,
};

pub const H8_CONFIG_READ: &str = "h8.erp_connector.read";
pub const H8_CONFIG_WRITE: &str = "h8.erp_connector.write";

type IdempotencyRecord = (String, u16, Value);
type IdempotencyCache = Arc<Mutex<HashMap<String, IdempotencyRecord>>>;

#[derive(Clone)]
pub struct H8ErpConnectorAppState {
    pub repository: Arc<dyn H8ErpConnectorRepository>,
    pub audit_pool: Option<PgPool>,
    /// AC15：无 DB 时本地幂等缓存；有 pool 时优先写 `idempotency_request`
    pub(crate) idempotency: IdempotencyCache,
}

impl H8ErpConnectorAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgH8ErpConnectorRepository { pool: pool.clone() }),
            audit_pool: Some(pool),
            idempotency: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_memory() -> Self {
        Self {
            repository: Arc::new(MemoryH8ErpConnectorRepository::default()),
            audit_pool: None,
            idempotency: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
