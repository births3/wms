//! Internal M-CG document numbering service exports.

pub use crate::document_numbering_repository::{
    DocumentNumberingError, GenerateDocumentNumberRequest, IdempotentMutation,
    PgDocumentNumberingService,
};
pub use wms_domain::DocumentNumberAllocation;
