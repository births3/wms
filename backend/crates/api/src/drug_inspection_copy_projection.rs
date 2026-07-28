use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    drug_inspection_copy_service::DrugInspectionCopyServiceError, h2_lifecycle::publish_event_in_tx,
};

pub(crate) async fn publish_report_projection_from_db(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    version_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), DrugInspectionCopyServiceError> {
    type ProjectionRow = (
        Uuid,
        Uuid,
        String,
        i32,
        String,
        String,
        bool,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        bool,
        DateTime<Utc>,
    );
    let row: Option<ProjectionRow> = sqlx::query_as(
        r#"
        SELECT report.id, report.product_id, report.batch_no,
               version.version_number, version.report_no, version.status,
               report.current_version_id = version.id AS is_current,
               version.modification_reason, version.customer_copy_status,
               attachment.storage_key, attachment.file_name, attachment.size_bytes,
               version.customer_copy_hash,
               version.digitally_signed_original,
               COALESCE(version.reviewed_at, version.updated_at)
          FROM drug_inspection_report_versions version
          JOIN drug_inspection_reports report
            ON report.id = version.report_id AND report.owner_id = version.owner_id
     LEFT JOIN attachments attachment
            ON attachment.id = version.customer_copy_file_id
           AND attachment.owner_id = version.owner_id
         WHERE version.owner_id = $1 AND version.id = $2
        "#,
    )
    .bind(owner_id)
    .bind(version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| DrugInspectionCopyServiceError::Database(error.to_string()))?;
    let Some((
        report_id,
        product_id,
        batch_no,
        version_number,
        report_no,
        status,
        is_current,
        modification_reason,
        copy_status,
        storage_key,
        file_name,
        size_bytes,
        copy_hash,
        digitally_signed_original,
        confirmed_at,
    )) = row
    else {
        return Ok(());
    };
    let event_key = format!(
        "portal-report:{version_id}:{copy_status}:{}:sig={}",
        copy_hash.as_deref().unwrap_or("none"),
        digitally_signed_original
    );
    let portal_copy_status = match copy_status.as_str() {
        "not_requested" | "queued" => "queued",
        "processing" => "processing",
        "available" => "available",
        _ => "failed",
    };
    let payload = json!({
        "projection_event_type": "drug_inspection_report.upsert",
        "id": version_id,
        "report_id": report_id,
        "owner_id": owner_id,
        "product_id": product_id,
        "batch_no": batch_no,
        "version_number": version_number,
        "report_no": report_no,
        "status": status,
        "is_current": is_current,
        "modification_reason": modification_reason,
        "customer_copy_status": portal_copy_status,
        "customer_copy_storage_key": storage_key,
        "customer_copy_file_name": file_name,
        "customer_copy_size": size_bytes,
        "customer_copy_hash": copy_hash,
        "digitally_signed_original": digitally_signed_original,
        "confirmed_at": confirmed_at,
        "updated_at": now
    });
    publish_event_in_tx(
        tx,
        owner_id,
        &event_key,
        "portal.drug_inspection_report.upsert",
        "M-DI",
        "drug_inspection_report_version",
        &version_id.to_string(),
        payload,
        now,
    )
    .await
    .map_err(|error| DrugInspectionCopyServiceError::Audit(format!("{error:?}")))?;
    Ok(())
}
