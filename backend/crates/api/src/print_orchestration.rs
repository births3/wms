//! H9 delivery-note aggregation application facade.

mod configuration;
mod cutoff_plan;
mod repository;
mod service;
mod workbench;

use serde::{Deserialize, Serialize};

pub use service::PrintOrchestrationService;
pub use wms_domain::DeliveryNoteGroup;

use crate::document_numbering::DocumentNumberingError;
pub(crate) use configuration::freeze_outbound_route_in_tx;

/// H9 delivery-note aggregation mutation result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

/// H9 delivery-note aggregation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintOrchestrationError {
    InvalidRequest,
    EffectivePeriodOverlap,
    CutoffPlanNotFound,
    InvalidState,
    RouteBindingNotFound,
    OrderNotFound,
    OrderNotEligibleForCutoff,
    AggregationBoundaryMismatch,
    OrderAlreadyCutoff,
    IdempotencyConflict,
    DocumentNumbering(DocumentNumberingError),
    Audit(String),
    Database(String),
    Serialize(String),
}
