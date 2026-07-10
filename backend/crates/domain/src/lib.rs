//! 主仓 OpenAPI 契约使用的 domain schema。

mod audit;
mod billing;
mod cold_chain;
mod common;
mod h4;
mod logistics;
mod master_dictionary;
mod menu;
mod operations;
mod receiving_outbound;
mod reporting;

pub use audit::*;
pub use billing::*;
pub use cold_chain::*;
pub use common::{
    CurrentUser, ErrorResponse, HealthzResponse, LoginRequest, LoginResponse, PageMeta,
    ResilienceStatus,
};
pub use h4::*;
pub use logistics::*;
pub use master_dictionary::*;
pub use menu::*;
pub use operations::*;
pub use receiving_outbound::*;
pub use reporting::*;

// Schema helper fns are re-exported from common for module-level attributes.
