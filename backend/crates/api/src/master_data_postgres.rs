//! PostgreSQL repository for M1 master data.
// Governance anchors retained after mechanical split: pub async fn create_product,
// pub async fn create_supplier, pub async fn create_customer, append_master_data_audit.

include!("master_data_postgres_part1.rs");
include!("master_data_postgres_part2.rs");
include!("master_data_postgres_part3.rs");
include!("master_data_postgres/customer_addresses.rs");
include!("master_data_postgres/customer_profile.rs");
