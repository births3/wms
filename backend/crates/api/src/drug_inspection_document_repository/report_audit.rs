use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::DrugInspectionReportVersion;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::DrugInspectionDocumentRepositoryError;

pub(super) async fn append_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    before: Value,
    after: Value,
    now: DateTime<Utc>,
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "DI",
        resource_type,
        resource_id.to_string(),
        Some(AuditDiff::compute(before, after)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| DrugInspectionDocumentRepositoryError::Audit(format!("{error:?}")))?;
    Ok(())
}

pub(super) fn version_snapshot(value: &DrugInspectionReportVersion) -> Value {
    serde_json::json!({
        "id": value.id,
        "report_id": value.report_id,
        "version_number": value.version_number,
        "report_no": value.report_no,
        "original_file_id": value.original_file_id,
        "original_file_hash": value.original_file_hash,
        "processing_mode": value.processing_mode,
        "qualified": value.qualified,
        "status": value.status,
        "replaces_version_id": value.replaces_version_id,
        "uploaded_by": value.uploaded_by,
        "reviewed_by": value.reviewed_by,
        "review_result": value.review_result,
        "customer_copy_status": value.customer_copy_status,
    })
}
