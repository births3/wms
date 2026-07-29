use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;
use wms_domain::{DrugInspectionReportVersion, DrugInspectionReviewQueueEntry};

use super::{
    map_db_error, report_helpers::DrugInspectionVersionRow, DrugInspectionDocumentRepositoryError,
    PgDrugInspectionDocumentRepository,
};

impl PgDrugInspectionDocumentRepository {
    pub async fn list_review_queue(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DrugInspectionReviewQueueEntry>, DrugInspectionDocumentRepositoryError> {
        let versions = sqlx::query_as::<_, DrugInspectionVersionRow>(
            r#"
            SELECT version.*
              FROM drug_inspection_report_versions AS version
             WHERE version.owner_id = $1
               AND version.status = 'pending_confirmation'
             ORDER BY version.submitted_at, version.created_at
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let metadata = sqlx::query_as::<_, ReviewQueueMetadata>(
            r#"
            SELECT version.id AS version_id, product.product_code, product.product_name,
                   report.batch_no, uploader.display_name AS uploader_name
              FROM drug_inspection_report_versions AS version
              JOIN drug_inspection_reports AS report
                ON report.id = version.report_id
               AND report.owner_id = version.owner_id
              JOIN products AS product
                ON product.id = report.product_id
               AND product.owner_id = report.owner_id
              JOIN auth_users AS uploader
                ON uploader.id = version.uploaded_by
             WHERE version.owner_id = $1
               AND version.status = 'pending_confirmation'
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|value| (value.version_id, value))
        .collect::<HashMap<_, _>>();
        versions
            .into_iter()
            .map(|version| {
                let version: DrugInspectionReportVersion = version.into();
                let meta = metadata.get(&version.id).ok_or_else(|| {
                    DrugInspectionDocumentRepositoryError::Serialize(
                        "药检审核队列元数据缺失".to_string(),
                    )
                })?;
                Ok(DrugInspectionReviewQueueEntry {
                    version,
                    product_code: meta.product_code.clone(),
                    product_name: meta.product_name.clone(),
                    batch_no: meta.batch_no.clone(),
                    uploader_name: meta.uploader_name.clone(),
                })
            })
            .collect()
    }

    pub async fn list_report_versions(
        &self,
        owner_id: Uuid,
        report_id: Uuid,
    ) -> Result<Vec<DrugInspectionReportVersion>, DrugInspectionDocumentRepositoryError> {
        sqlx::query_as::<_, DrugInspectionVersionRow>(
            r#"
            SELECT *
              FROM drug_inspection_report_versions
             WHERE owner_id = $1 AND report_id = $2
             ORDER BY version_number DESC
            "#,
        )
        .bind(owner_id)
        .bind(report_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_db_error)
    }
}

#[derive(FromRow)]
struct ReviewQueueMetadata {
    version_id: Uuid,
    product_code: String,
    product_name: String,
    batch_no: String,
    uploader_name: String,
}
