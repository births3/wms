use crate::{audit::AuditLog, h8_erp_connectors::H8ErpConnectorRepository};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

use super::repository::{
    H8InterfaceTableRepository, MemoryH8InterfaceTableRepository, MssqlH8InterfaceTableRepository,
};

pub const H8_INTERFACE_TABLE_READ: &str = "h8.erp_interface_table.read";

#[derive(Clone)]
pub struct H8ErpInterfaceTableAppState {
    pub(crate) connectors: Arc<dyn H8ErpConnectorRepository>,
    pub(crate) repository: Arc<dyn H8InterfaceTableRepository>,
    pub(crate) audit_pool: Option<PgPool>,
    pub(crate) audit_log: Arc<Mutex<AuditLog>>,
}

impl H8ErpInterfaceTableAppState {
    pub fn with_postgres(pool: PgPool, connectors: Arc<dyn H8ErpConnectorRepository>) -> Self {
        Self {
            connectors,
            repository: Arc::new(MssqlH8InterfaceTableRepository::default()),
            audit_pool: Some(pool),
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
        }
    }

    pub fn with_memory(connectors: Arc<dyn H8ErpConnectorRepository>) -> Self {
        Self {
            connectors,
            repository: Arc::new(MemoryH8InterfaceTableRepository::default()),
            audit_pool: None,
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
        }
    }

    pub fn with_memory_rows(
        connectors: Arc<dyn H8ErpConnectorRepository>,
        rows: Vec<wms_domain::H8ErpInterfaceTableRow>,
    ) -> Self {
        Self {
            connectors,
            repository: Arc::new(MemoryH8InterfaceTableRepository::with_rows(rows)),
            audit_pool: None,
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
        }
    }
}
