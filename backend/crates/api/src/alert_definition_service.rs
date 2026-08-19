use chrono::{DateTime, Utc};
use sqlx::PgPool;
use wms_domain::{
    AlertDefinitionChangeOperation, CreateQualityLiaisonRequest, QualityLiaisonOrder,
    SubmitAlertDefinitionChangeRequest,
};

use crate::{
    alert_definition_repository::{AlertDefinitionRepositoryError, PgAlertDefinitionRepository},
    operation_context::OperationContext as AuthContext,
    quality_liaison::{PgQualityLiaisonRepository, QualityLiaisonError},
};

const APPROVAL_TYPE_CODE: &str = "alert_definition_change";

#[derive(Clone, Debug)]
pub struct AlertDefinitionService {
    definitions: PgAlertDefinitionRepository,
    quality_liaisons: PgQualityLiaisonRepository,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertDefinitionServiceError {
    Definition(AlertDefinitionRepositoryError),
    QualityLiaison(QualityLiaisonError),
}

impl AlertDefinitionService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            definitions: PgAlertDefinitionRepository::new(pool.clone()),
            quality_liaisons: PgQualityLiaisonRepository::new(pool),
        }
    }

    pub async fn submit_change(
        &self,
        ctx: &AuthContext,
        request: SubmitAlertDefinitionChangeRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<QualityLiaisonOrder, AlertDefinitionServiceError> {
        self.definitions
            .validate_change(&request)
            .map_err(AlertDefinitionServiceError::Definition)?;
        if let Some(definition) = request.definition.as_ref() {
            self.definitions
                .ensure_notification_channel(ctx.owner_id, &definition.event_type)
                .await
                .map_err(AlertDefinitionServiceError::Definition)?;
            if let Some(rule_code) = definition.escalation_ref.as_deref() {
                self.definitions
                    .ensure_escalation_rule(ctx.owner_id, rule_code)
                    .await
                    .map_err(AlertDefinitionServiceError::Definition)?;
            }
        }
        let (related_document_no, operation_label) = match request.operation {
            AlertDefinitionChangeOperation::Upsert if request.definition_id.is_none() => (
                request
                    .definition
                    .as_ref()
                    .map(|definition| definition.alert_code.trim().to_string())
                    .unwrap_or_default(),
                "新增",
            ),
            operation => {
                let id = request.definition_id.ok_or_else(|| {
                    AlertDefinitionServiceError::Definition(
                        AlertDefinitionRepositoryError::Invalid("告警定义 ID 不能为空".to_string()),
                    )
                })?;
                let current = self
                    .definitions
                    .get(ctx.owner_id, id)
                    .await
                    .map_err(AlertDefinitionServiceError::Definition)?;
                if request.expected_version != Some(current.version) {
                    return Err(AlertDefinitionServiceError::Definition(
                        AlertDefinitionRepositoryError::StaleVersion,
                    ));
                }
                let label = match operation {
                    AlertDefinitionChangeOperation::Upsert => "修改",
                    AlertDefinitionChangeOperation::SetEnabled => "启停",
                    AlertDefinitionChangeOperation::Delete => "删除",
                };
                (current.alert_code, label)
            }
        };
        let business_payload = serde_json::json!({
            "action": "apply_alert_definition_change",
            "change": request,
        });
        self.quality_liaisons
            .create(
                ctx,
                CreateQualityLiaisonRequest {
                    type_code: APPROVAL_TYPE_CODE.to_string(),
                    related_document_type: "alert_definition".to_string(),
                    related_document_no: related_document_no.clone(),
                    problem_description: format!(
                        "申请{operation_label}告警定义 {related_document_no}"
                    ),
                    disposition_suggestion: "审批通过后由系统原子应用变更".to_string(),
                    trigger_source: "H-AL".to_string(),
                    business_payload,
                },
                now,
                idempotency_key,
            )
            .await
            .map(|result| result.value)
            .map_err(AlertDefinitionServiceError::QualityLiaison)
    }
}
