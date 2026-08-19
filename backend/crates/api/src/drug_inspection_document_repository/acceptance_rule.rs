use chrono::Utc;
use sqlx::FromRow;
use uuid::Uuid;
use wms_domain::{DrugInspectionRequirementRule, UpsertDrugInspectionRequirementRuleRequest};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
};

use super::{
    helpers::{lock_idempotency_key, replay_idempotency, request_hash, store_idempotency},
    map_db_error, DrugInspectionDocumentRepositoryError, PgDrugInspectionDocumentRepository,
};

#[derive(FromRow)]
struct RuleRow {
    id: Uuid,
    owner_id: Uuid,
    special_drug_category: String,
    missing_behavior: String,
    enabled: bool,
    version: i64,
    updated_by: Uuid,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl PgDrugInspectionDocumentRepository {
    pub async fn list_requirement_rules(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DrugInspectionRequirementRule>, DrugInspectionDocumentRepositoryError> {
        sqlx::query_as::<_, RuleRow>(
            "SELECT id, owner_id, special_drug_category, missing_behavior,
                    enabled, version, updated_by, created_at, updated_at
             FROM drug_inspection_requirement_rules
             WHERE owner_id = $1
             ORDER BY special_drug_category",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_db_error)
    }

    pub async fn upsert_requirement_rule(
        &self,
        ctx: &AuthContext,
        request: UpsertDrugInspectionRequirementRuleRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionRequirementRule, DrugInspectionDocumentRepositoryError> {
        request
            .validate()
            .map_err(DrugInspectionDocumentRepositoryError::Invalid)?;
        let now = Utc::now();
        let request_hash = request_hash(&request)?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(replay);
        }
        let row = sqlx::query_as::<_, RuleRow>(
            r#"
            INSERT INTO drug_inspection_requirement_rules (
                id, owner_id, special_drug_category, missing_behavior,
                enabled, version, updated_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 1, $6, $7, $7)
            ON CONFLICT (owner_id, special_drug_category)
            DO UPDATE SET
                missing_behavior = EXCLUDED.missing_behavior,
                enabled = EXCLUDED.enabled,
                version = drug_inspection_requirement_rules.version + 1,
                updated_by = EXCLUDED.updated_by,
                updated_at = EXCLUDED.updated_at
            RETURNING id, owner_id, special_drug_category, missing_behavior,
                      enabled, version, updated_by, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(request.special_drug_category.trim())
        .bind(&request.missing_behavior)
        .bind(request.enabled)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "di.requirement_rule.upsert",
            "M-DI",
            "drug_inspection_requirement_rule",
            row.id.to_string(),
            Some(AuditDiff::compute(
                serde_json::json!({}),
                serde_json::json!({
                    "special_drug_category": row.special_drug_category,
                    "missing_behavior": row.missing_behavior,
                    "enabled": row.enabled,
                    "version": row.version
                }),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DrugInspectionDocumentRepositoryError::Audit(format!("{error:?}")))?;
        let result = DrugInspectionRequirementRule::from(row);
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            "/api/v1/drug-inspection/requirement-rules/current",
            "drug_inspection_requirement_rule",
            result.id,
            &result,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(result)
    }
}

impl From<RuleRow> for DrugInspectionRequirementRule {
    fn from(row: RuleRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            special_drug_category: row.special_drug_category,
            missing_behavior: row.missing_behavior,
            enabled: row.enabled,
            version: row.version,
            updated_by: row.updated_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
