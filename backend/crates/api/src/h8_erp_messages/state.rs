//! H8 消息应用状态。

use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use crate::audit::AuditLog;
use crate::h8_erp_connectors::{H8ErpConnectorAppState, H8ErpConnectorRepository};

use super::payload_repository::{
    H8PayloadRepository, MemoryH8PayloadRepository, PgH8PayloadRepository,
};
use super::pg_repository::PgH8ErpMessageRepository;
use super::repository::{H8ErpMessageRepository, MemoryH8ErpMessageRepository};
use super::runtime_repository::{
    H8WorkerRuntimeRepository, MemoryH8WorkerRuntimeRepository, PgH8WorkerRuntimeRepository,
};

pub const H8_MSG_READ: &str = "h8.erp_connector.read";
pub const H8_MSG_WRITE: &str = "h8.erp_connector.write";
pub const H8_RECEIPT_WRITE: &str = "h8.erp_receipt.write";

#[derive(Clone)]
pub struct H8ErpMessageAppState {
    pub repository: Arc<dyn H8ErpMessageRepository>,
    pub connector_repository: Arc<dyn H8ErpConnectorRepository>,
    pub runtime_repository: Arc<dyn H8WorkerRuntimeRepository>,
    pub payload_repository: Arc<dyn H8PayloadRepository>,
    pub audit_pool: Option<PgPool>,
    /// 软件路径可观测审计 sink（始终写入，单测可断言）
    pub audit_log: Arc<Mutex<AuditLog>>,
}

impl H8ErpMessageAppState {
    pub fn with_memory() -> Self {
        Self {
            repository: Arc::new(MemoryH8ErpMessageRepository::default()),
            connector_repository: H8ErpConnectorAppState::with_memory().repository,
            runtime_repository: Arc::new(MemoryH8WorkerRuntimeRepository::default()),
            payload_repository: Arc::new(MemoryH8PayloadRepository::default()),
            audit_pool: None,
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
        }
    }

    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgH8ErpMessageRepository::new(pool.clone())),
            connector_repository: H8ErpConnectorAppState::with_postgres(pool.clone()).repository,
            runtime_repository: Arc::new(PgH8WorkerRuntimeRepository::new(pool.clone())),
            payload_repository: Arc::new(PgH8PayloadRepository::new(pool.clone())),
            audit_pool: Some(pool),
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
        }
    }
}
