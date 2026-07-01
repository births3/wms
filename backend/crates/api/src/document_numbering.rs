//! Internal M-CG document numbering service exports.

pub use crate::document_numbering_repository::{
    DocumentNumberAllocation, DocumentNumberingError, GenerateDocumentNumberRequest,
    IdempotentMutation, PgDocumentNumberingService,
};
