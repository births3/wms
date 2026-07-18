//! Wave 2 M-PM parameter mapping runtime service.

use std::{collections::BTreeMap, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    ErrorResponse, ExecuteMappingRequest, ExecuteMappingResponse, MappingDictionary,
    MappingQueueItem, MappingRule, MappingTraceResponse,
};

use crate::{
    audit::{append_event, AuditDiff, AuditError, AuditWriteRequest},
    auth::{AuthContext, AuthError},
};

const EXECUTE_PERMISSION: &str = "mpm.execute";

#[derive(Clone, Debug)]
pub struct ParameterMappingAppState {
    service: Arc<Mutex<ParameterMappingService>>,
    pool: PgPool,
}

impl ParameterMappingAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            service: Arc::new(Mutex::new(ParameterMappingService::default())),
            pool,
        }
    }
}

#[derive(Debug)]
pub enum ParameterMappingHandlerError {
    Auth(AuthError),
    Mapping(MappingError),
    Audit(AuditError),
}

impl From<AuthError> for ParameterMappingHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl IntoResponse for ParameterMappingHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::Mapping(MappingError::RawPayloadMustBeObject) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "PM_RAW_PAYLOAD_INVALID",
                "raw_payload must be an object".to_string(),
            ),
            Self::Mapping(MappingError::TraceNotFound) => (
                StatusCode::NOT_FOUND,
                "PM_TRACE_NOT_FOUND",
                "mapping trace not found".to_string(),
            ),
            Self::Audit(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "PM_AUDIT_FAILED",
                format!("failed to append mapping audit: {error:?}"),
            ),
            Self::Auth(_) => unreachable!(),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message,
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn parameter_mapping_router(state: ParameterMappingAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/parameter-mapping/execute",
            post(execute_mapping_handler),
        )
        .with_state(state)
}

async fn execute_mapping_handler(
    ctx: AuthContext,
    State(state): State<ParameterMappingAppState>,
    Json(req): Json<ExecuteMappingRequest>,
) -> Result<Json<ExecuteMappingResponse>, ParameterMappingHandlerError> {
    ctx.require_permission(EXECUTE_PERMISSION)?;
    let source_system = req.source_system.clone();
    let mut service = state.service.lock().await;
    let mut next = service.clone();
    let result = next
        .execute(&ctx, req, Utc::now())
        .map_err(ParameterMappingHandlerError::Mapping)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "execute_mapping",
        "M-PM",
        "parameter_mapping_execution",
        result.execution_id.to_string(),
        Some(AuditDiff::compute(
            serde_json::json!({"source_system": source_system}),
            serde_json::json!({
                "unresolved_fields": &result.unresolved_fields,
                "queue_item_id": result.queue_item_id,
            }),
        )),
    );
    append_event(&state.pool, &audit)
        .await
        .map_err(ParameterMappingHandlerError::Audit)?;
    *service = next;
    Ok(Json(result))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MappingError {
    TraceNotFound,
    RawPayloadMustBeObject,
}

#[derive(Clone, Debug)]
struct MappingExecution {
    execution_id: Uuid,
    owner_id: Uuid,
    source_system: String,
    raw_payload: Value,
    normalized_payload: Value,
    applied_rule_ids: Vec<Uuid>,
    unresolved_fields: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ParameterMappingService {
    dictionaries: BTreeMap<Uuid, MappingDictionary>,
    rules: BTreeMap<Uuid, MappingRule>,
    queue: BTreeMap<Uuid, MappingQueueItem>,
    executions: BTreeMap<Uuid, MappingExecution>,
}

impl ParameterMappingService {
    pub fn add_dictionary(
        &mut self,
        ctx: &AuthContext,
        dictionary_code: impl Into<String>,
        dictionary_name: impl Into<String>,
        now: DateTime<Utc>,
    ) -> MappingDictionary {
        let dictionary = MappingDictionary {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            dictionary_code: dictionary_code.into(),
            dictionary_name: dictionary_name.into(),
            created_at: now,
        };
        self.dictionaries.insert(dictionary.id, dictionary.clone());
        dictionary
    }

    pub fn add_rule(
        &mut self,
        ctx: &AuthContext,
        source_system: impl Into<String>,
        external_field: impl Into<String>,
        canonical_field: impl Into<String>,
        transform: impl Into<String>,
        now: DateTime<Utc>,
    ) -> MappingRule {
        let rule = MappingRule {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            source_system: source_system.into(),
            external_field: external_field.into(),
            canonical_field: canonical_field.into(),
            transform: transform.into(),
            created_at: now,
        };
        self.rules.insert(rule.id, rule.clone());
        rule
    }

    pub fn execute(
        &mut self,
        ctx: &AuthContext,
        req: ExecuteMappingRequest,
        now: DateTime<Utc>,
    ) -> Result<ExecuteMappingResponse, MappingError> {
        let Some(raw_object) = req.raw_payload.as_object() else {
            return Err(MappingError::RawPayloadMustBeObject);
        };

        let mut normalized = Map::new();
        let mut unresolved_fields = Vec::new();
        let mut applied_rule_ids = Vec::new();

        for (field, value) in raw_object {
            let matched_rule = self.rules.values().find(|rule| {
                rule.owner_id == ctx.owner_id
                    && rule.source_system == req.source_system
                    && rule.external_field == *field
            });
            if let Some(rule) = matched_rule {
                normalized.insert(
                    rule.canonical_field.clone(),
                    apply_transform(value, &rule.transform),
                );
                applied_rule_ids.push(rule.id);
            } else {
                unresolved_fields.push(field.clone());
            }
        }

        unresolved_fields.sort();
        let queue_item_id = if unresolved_fields.is_empty() {
            None
        } else {
            let item = MappingQueueItem {
                id: Uuid::new_v4(),
                owner_id: ctx.owner_id,
                source_system: req.source_system.clone(),
                raw_payload: req.raw_payload.clone(),
                status: "pending_mapping".to_string(),
                created_at: now,
            };
            let id = item.id;
            self.queue.insert(id, item);
            Some(id)
        };

        let execution_id = Uuid::new_v4();
        let normalized_payload = Value::Object(normalized);
        self.executions.insert(
            execution_id,
            MappingExecution {
                execution_id,
                owner_id: ctx.owner_id,
                source_system: req.source_system,
                raw_payload: req.raw_payload,
                normalized_payload: normalized_payload.clone(),
                applied_rule_ids,
                unresolved_fields: unresolved_fields.clone(),
            },
        );

        Ok(ExecuteMappingResponse {
            execution_id,
            queue_item_id,
            normalized_payload,
            unresolved_fields,
        })
    }

    pub fn trace(
        &self,
        ctx: &AuthContext,
        execution_id: Uuid,
    ) -> Result<MappingTraceResponse, MappingError> {
        let execution = self
            .executions
            .get(&execution_id)
            .filter(|execution| execution.owner_id == ctx.owner_id)
            .ok_or(MappingError::TraceNotFound)?;

        Ok(MappingTraceResponse {
            execution_id: execution.execution_id,
            source_system: execution.source_system.clone(),
            raw_payload: execution.raw_payload.clone(),
            normalized_payload: execution.normalized_payload.clone(),
            applied_rule_ids: execution.applied_rule_ids.clone(),
            unresolved_fields: execution.unresolved_fields.clone(),
        })
    }

    pub fn pending_queue_len(&self, ctx: &AuthContext) -> usize {
        self.queue
            .values()
            .filter(|item| item.owner_id == ctx.owner_id && item.status == "pending_mapping")
            .count()
    }

    pub fn dictionary_count(&self, ctx: &AuthContext) -> usize {
        self.dictionaries
            .values()
            .filter(|dictionary| dictionary.owner_id == ctx.owner_id)
            .count()
    }
}

fn apply_transform(value: &Value, transform: &str) -> Value {
    match (transform, value) {
        ("trim", Value::String(text)) => Value::String(text.trim().to_string()),
        ("upper", Value::String(text)) => Value::String(text.trim().to_ascii_uppercase()),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;
    use wms_domain::ExecuteMappingRequest;

    use super::ParameterMappingService;
    use crate::auth::AuthContext;

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["mpm.execute".to_string()],
            jti: Uuid::new_v4().to_string(),
            warehouse_scope: None,
        }
    }

    #[test]
    fn maps_irregular_erp_payload_and_traces_execution() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 11, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut service = ParameterMappingService::default();
        service.add_dictionary(&ctx, "erp_product", "ERP 商品字段", now);
        let rule = service.add_rule(&ctx, "ERP", "ITEM_NO", "product_code", "upper", now);
        service.add_rule(&ctx, "ERP", "ITEM_NAME", "product_name", "trim", now);

        let response = service
            .execute(
                &ctx,
                ExecuteMappingRequest {
                    source_system: "ERP".to_string(),
                    raw_payload: json!({
                        "ITEM_NO": " p-001 ",
                        "ITEM_NAME": " 感冒灵颗粒 ",
                        "UNKNOWN_COL": "legacy",
                    }),
                },
                now,
            )
            .expect("execute mapping");

        assert_eq!(response.normalized_payload["product_code"], "P-001");
        assert_eq!(response.normalized_payload["product_name"], "感冒灵颗粒");
        assert_eq!(response.unresolved_fields, vec!["UNKNOWN_COL"]);
        assert_eq!(service.pending_queue_len(&ctx), 1);

        let trace = service.trace(&ctx, response.execution_id).expect("trace");
        assert!(trace.applied_rule_ids.contains(&rule.id));
        assert_eq!(trace.raw_payload["UNKNOWN_COL"], "legacy");
    }
}
