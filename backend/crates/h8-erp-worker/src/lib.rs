//! H8 ERP 接口表常驻 Worker。

pub mod config;
pub mod contract;
pub mod control_plane;
pub mod error;
pub mod inbound;
pub mod mssql;
mod mssql_publish;
pub mod outbound;
pub mod outbound_runner;
pub mod outbox_repository;
pub mod receipts;
pub mod runner;
