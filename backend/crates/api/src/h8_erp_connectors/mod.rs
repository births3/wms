//! US-H8-001：ERP 连接配置 API（h8.erp_connector.read / h8.erp_connector.write）。

mod audit;
mod error;
mod handlers;
mod idempotency;
mod probe;
mod repository;
mod row;
mod state;

#[cfg(test)]
mod tests;

pub use error::H8ErpConnectorRepoError;
pub use handlers::h8_erp_connector_router;
pub use repository::H8ErpConnectorRepository;
pub use state::{H8ErpConnectorAppState, H8_CONFIG_READ, H8_CONFIG_WRITE};
