//! US-H8-004：ERP 接口表受控只读探查 API。

mod audit;
mod error;
mod handlers;
mod repository;
mod state;

pub use handlers::h8_erp_interface_table_router;
pub use state::{H8ErpInterfaceTableAppState, H8_INTERFACE_TABLE_READ};
