use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::DocumentNumberAllocation;

use crate::idempotency;

use super::{
    AllocationRow, AllocationWithHashRow, DocumentNumberRule, DocumentNumberRuleRow,
    DocumentNumberingError, GenerateDocumentNumberRequest,
};

pub(super) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), DocumentNumberingError> {
    idempotency::lock_key(tx, "document-numbering", owner_id, idempotency_key)
        .await
        .map_err(Into::into)
}

pub(super) fn document_number_request_hash(
    req: &GenerateDocumentNumberRequest,
) -> Result<String, DocumentNumberingError> {
    json_request_hash(
        &serde_json::to_value(req)
            .map_err(|error| DocumentNumberingError::Serialize(error.to_string()))?,
    )
}

pub(super) fn json_request_hash(value: &Value) -> Result<String, DocumentNumberingError> {
    idempotency::request_hash(value).map_err(Into::into)
}

pub(super) fn map_db_error(error: sqlx::Error) -> DocumentNumberingError {
    DocumentNumberingError::Database(error.to_string())
}

impl From<DocumentNumberRuleRow> for DocumentNumberRule {
    fn from(row: DocumentNumberRuleRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            document_type: row.document_type,
            rule_code: row.rule_code,
            rule_name: row.rule_name,
            template: row.template,
            reset_policy: row.reset_policy,
            sequence_width: row.sequence_width,
            sequence_mode: row.sequence_mode,
            enabled: row.enabled,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

impl From<AllocationRow> for DocumentNumberAllocation {
    fn from(row: AllocationRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            rule_id: row.rule_id,
            document_type: row.document_type,
            generated_no: row.generated_no,
            sequence_value: row.sequence_value,
            counter_key: row.counter_key,
            source_module: row.source_module,
            source_document_id: row.source_document_id,
            created_at: row.created_at,
        }
    }
}

impl From<AllocationWithHashRow> for DocumentNumberAllocation {
    fn from(row: AllocationWithHashRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            rule_id: row.rule_id,
            document_type: row.document_type,
            generated_no: row.generated_no,
            sequence_value: row.sequence_value,
            counter_key: row.counter_key,
            source_module: row.source_module,
            source_document_id: row.source_document_id,
            created_at: row.created_at,
        }
    }
}
