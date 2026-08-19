//! Internal M-CG document numbering service exports.

pub use crate::document_numbering_repository::{
    DocumentNumberRule, DocumentNumberRuleListResponse, DocumentNumberingError,
    GenerateDocumentNumberRequest, IdempotentMutation, PgDocumentNumberingService,
    SetDocumentNumberRuleEnabledRequest, UpsertDocumentNumberRuleRequest,
};
pub use wms_domain::DocumentNumberAllocation;
