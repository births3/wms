//! US-H8-001：ERP 连接配置 API（h8.erp_connector.read / h8.erp_connector.write）。

mod audit;
mod error;
mod handlers;
mod idempotency;
mod persistence;
mod probe;
mod repository;
mod row;
mod state;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod transaction_tests;

pub use error::H8ErpConnectorRepoError;
pub use handlers::h8_erp_connector_router;
#[doc(hidden)]
pub use idempotency::H8IdempotencyWrite;
pub use repository::{H8ConnectorStatusTransition, H8ErpConnectorRepository};
pub use state::{H8ErpConnectorAppState, H8_CONFIG_READ, H8_CONFIG_WRITE};
