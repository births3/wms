//! 主仓 OpenAPI 契约使用的 domain schema。

mod alert_definition;
mod alert_engine;
mod alert_runtime;
mod api_key;
mod audit;
mod billing;
mod cold_chain;
mod common;
mod container_quality_lock;
mod dock;
mod drug_inspection;
mod drug_inspection_document;
mod dual_person_policy;
mod file_attachment;
mod h4;
mod h8_erp;
mod h8_erp_exchange;
mod h8_erp_interface_table;
mod h8_erp_message;
mod inventory_count;
mod logistics;
mod lpn_container;
mod m3_ops;
mod maintenance;
mod master_dictionary;
mod menu;
mod operations;
mod print_device;
mod print_orchestration;
mod putaway_validation;
mod quality_liaison;
mod quantity;
mod receiving_outbound;
mod replenishment;
mod reporting;
mod stock_adjustment;
mod system_dictionary;
mod task_engine;
mod task_type;

pub use alert_definition::*;
pub use alert_engine::*;
pub use alert_runtime::*;
pub use api_key::*;
pub use audit::*;
pub use billing::*;
pub use cold_chain::*;
pub use common::{
    AuthRevocationResponse, AuthSession, AuthSessionListResponse, AuthSessionRevokeResponse,
    AuthUserStatusRequest, CurrentUser, ErrorResponse, HealthzResponse, LoginRequest,
    LoginResponse, PageMeta, PasswordChangeRequest, ResilienceStatus,
};
pub use container_quality_lock::*;
pub use dock::*;
pub use drug_inspection::*;
pub use drug_inspection_document::*;
pub use dual_person_policy::*;
pub use file_attachment::*;
pub use h4::*;
pub use h8_erp::*;
pub use h8_erp_exchange::*;
pub use h8_erp_interface_table::*;
pub use h8_erp_message::*;
pub use inventory_count::*;
pub use logistics::*;
pub use lpn_container::*;
pub use m3_ops::*;
pub use maintenance::*;
pub use master_dictionary::*;
pub use menu::*;
pub use operations::*;
pub use print_device::*;
pub use print_orchestration::*;
pub use putaway_validation::*;
pub use quality_liaison::*;
pub use quantity::*;
pub use receiving_outbound::*;
pub use replenishment::*;
pub use reporting::*;
pub use stock_adjustment::*;
pub use system_dictionary::*;
pub use task_engine::*;
pub use task_type::*;

// Schema helper fns are re-exported from common for module-level attributes.
