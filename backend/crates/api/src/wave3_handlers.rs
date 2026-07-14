//! Wave 3 Axum handlers with H2 audit integration.

mod inventory_count;
#[path = "m2_putaway.rs"]
mod m2_putaway;
mod maintenance;
use inventory_count::apply_inventory_count_routes;
use maintenance::apply_maintenance_routes;

include!("wave3_handlers_part3.rs");
include!("wave3_handlers_part4.rs");
include!("wave3_handlers_part6.rs");
include!("wave3_handlers_part5.rs");
