//! Completion state and audit helpers for category PDF preparation.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::CategoryPdfPreparation;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::{
    category_pdf_repository::load_outputs, repository::map_db_error, PrintOrchestrationError,
    PrintOrchestrationService,
};

impl PrintOrchestrationService {
    pub(super) async fn load_preparation(
        &self,
        owner_id: Uuid,
        instance_id: Uuid,
        preparation_id: Uuid,
    ) -> Result<CategoryPdfPreparation, PrintOrchestrationError> {
        let row: (String, String) = sqlx::query_as(
            r#"
            SELECT status, idempotency_key
              FROM h9_category_pdf_preparations
             WHERE owner_id = $1 AND instance_id = $2 AND id = $3
            "#,
        )
        .bind(owner_id)
        .bind(instance_id)
        .bind(preparation_id)
        .fetch_optional(&self.repository.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintOrchestrationError::CategoryPdfNotFound)?;
        Ok(CategoryPdfPreparation {
            instance_id,
            status: row.0,
            idempotency_key: row.1,
            outputs: load_outputs(&self.repository.pool, owner_id, instance_id).await?,
        })
    }

    pub(super) async fn audit_preparation_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        action: &str,
        instance_id: Uuid,
        preparation_id: Uuid,
        failure: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<(), PrintOrchestrationError> {
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "H9",
            "category_pdf_preparation",
            preparation_id.to_string(),
            Some(AuditDiff::compute(
                Value::Null,
                json!({
                    "instance_id": instance_id,
                    "preparation_id": preparation_id,
                    "failure": failure,
                }),
            )),
        );
        audit.occurred_at = now;
        append_event_in_tx(tx, &audit)
            .await
            .map(|_| ())
            .map_err(|error| PrintOrchestrationError::Audit(format!("{error:?}")))
    }
}
