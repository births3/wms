//! 主仓 OpenAPI 契约。

pub mod admin_menu;
pub mod admin_menu_handlers;
mod admin_menu_idempotency;
mod admin_menu_model;
mod admin_menu_repository;
pub mod audit;
pub mod auth;
pub mod auth_handlers;
pub mod auth_repository;
pub mod auth_service;
pub mod billing;
pub mod cold_chain;
pub mod config_center;
pub mod deploy_audit;
pub mod document_numbering;
pub mod document_numbering_handlers;
mod document_numbering_repository;
pub mod express;
pub mod feature_flags;
pub mod h2_lifecycle;
pub mod h2_lifecycle_handlers;
pub mod inbound;
pub mod inventory;
pub mod master_data;
pub mod master_data_handlers;
pub mod master_data_postgres;
mod openapi_contract;
pub mod outbound;
pub mod packing_station;
pub mod parameter_mapping;
pub mod print_template;
pub mod print_template_handlers;
pub mod reports;
pub mod resilience;
pub mod retail_chain;
pub mod state_machine;
pub mod system_dictionary;
pub mod system_dictionary_handlers;
pub mod tms_plus;
pub mod traceability_code;
pub mod wave3_handlers;
pub mod wave3_repository;
pub mod wave4_handlers;
pub mod wave4_repository;
pub mod wave5_handlers;
pub mod wave5_repository;
pub mod wechat_notify;
mod wechat_notify_idempotency;
pub mod wechat_notify_service;

mod openapi_doc;
mod openapi_paths;

#[cfg(test)]
mod openapi_tests;

pub use openapi_doc::ApiDoc;
