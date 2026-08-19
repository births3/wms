//! Wave 1 H2 append-only audit runtime contract.
//!
//! The PostgreSQL enforcement lives in `backend/migrations/*_audit_event.sql`.
//! This module provides the shared write helper every mutation handler should
//! call before returning success.

mod db;
mod models;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use db::AuditSealProgress;
pub use db::{
    append_event, append_event_in_tx, commit_with_audit, export_events, list_events,
    seal_audit_chain,
};
pub use models::{
    AuditChainSeal, AuditDiff, AuditError, AuditEventPage, AuditEventQuery, AuditEventQueryCursor,
    AuditEventRecord, AuditLog, AuditWriteRequest, AUDIT_SEAL_BATCH_SIZE,
    DEFAULT_AUDIT_EVENT_QUERY_LIMIT, MAX_AUDIT_EVENT_QUERY_LIMIT, MAX_AUDIT_EXPORT_EVENTS,
};
