//! H9 delivery-note aggregation application facade.

mod aggregation_rule;
mod category_pdf;
mod category_pdf_completion;
mod category_pdf_repository;
mod configuration;
mod cutoff_plan;
mod print_suite;
mod render_worker;
mod repository;
mod service;
mod workbench;

use serde::{Deserialize, Serialize};

pub use render_worker::CategoryPdfRenderer;
pub use service::PrintOrchestrationService;
pub use wms_domain::DeliveryNoteGroup;

use crate::document_numbering::DocumentNumberingError;
use crate::file_attachment::FileAttachmentError;
pub(crate) use configuration::freeze_outbound_route_in_tx;
use render_worker::RenderWorkerError;

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
    AggregationRuleMismatch,
    AggregationRuleNotFound,
    AggregationRuleInvalidState,
    OrderAlreadyCutoff,
    PrintSuiteNotFound,
    PrintSuiteInvalidState,
    PrintSuiteCategoryInvalid,
    PrintSuiteBindingInvalid,
    CategoryPdfNotFound,
    CategoryPdfDocumentsNotReady,
    DeliveryNoteGroupNotFound,
    IdempotencyConflict,
    DocumentNumbering(DocumentNumberingError),
    FileAttachment(FileAttachmentError),
    RenderWorker(RenderWorkerError),
    Audit(String),
    Database(String),
    Serialize(String),
}
