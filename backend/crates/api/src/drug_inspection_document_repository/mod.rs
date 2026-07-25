mod acceptance_rule;
pub(crate) mod helpers;
mod inbound_list;
mod report;
mod report_audit;
mod report_helpers;
mod review_queue;
mod stamp;
mod upstream_delivery;

use sqlx::PgPool;
use wms_domain::DrugInspectionDocumentValidationError;

pub use inbound_list::InboundDocumentQuery;
pub use stamp::PgDrugInspectionStampRepository;

#[derive(Clone)]
pub struct PgDrugInspectionDocumentRepository {
    pool: PgPool,
}

impl PgDrugInspectionDocumentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug)]
pub enum DrugInspectionDocumentRepositoryError {
    Invalid(DrugInspectionDocumentValidationError),
    NotFound,
    Conflict(&'static str),
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

fn map_db_error(error: sqlx::Error) -> DrugInspectionDocumentRepositoryError {
    DrugInspectionDocumentRepositoryError::Database(error.to_string())
}
