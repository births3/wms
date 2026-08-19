//! M-PM persistent value mapping used by H8 before business API calls.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    ErrorResponse, MapParameterRequest, MapParameterResponse, ParameterMappingStatus,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::{AuthContext, AuthError},
    idempotency,
};

const EXECUTE_PERMISSION: &str = "mpm.execute";
const MAP_PATH: &str = "/api/v1/parameter-mapping/map";

#[derive(Clone, Debug)]
pub struct ParameterMappingAppState {
    pool: PgPool,
}

impl ParameterMappingAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug)]
pub enum ParameterMappingHandlerError {
    Auth(AuthError),
    MissingIdempotencyKey,
    IdempotencyConflict,
    InvalidRequest,
    DictionaryNotFound,
    Persistence(String),
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
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "PM_IDEMPOTENCY_REQUIRED",
                "Idempotency-Key header is required",
            ),
            Self::IdempotencyConflict => (
                StatusCode::CONFLICT,
                "PM_IDEMPOTENCY_CONFLICT",
                "Idempotency-Key was already used for a different request",
            ),
            Self::InvalidRequest => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "PM_REQUEST_INVALID",
                "parameter mapping request is invalid",
            ),
            Self::DictionaryNotFound => (
                StatusCode::NOT_FOUND,
                "PM_DICTIONARY_NOT_FOUND",
                "parameter mapping dictionary was not found",
            ),
            Self::Persistence(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "PM_PERSISTENCE_FAILED",
                "parameter mapping persistence failed",
            ),
            Self::Auth(_) => unreachable!(),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
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
        .route(MAP_PATH, post(map_parameter_handler))
        .with_state(state)
}

async fn map_parameter_handler(
    ctx: AuthContext,
    State(state): State<ParameterMappingAppState>,
    headers: HeaderMap,
    Json(req): Json<MapParameterRequest>,
) -> Result<Json<MapParameterResponse>, ParameterMappingHandlerError> {
    ctx.require_permission(EXECUTE_PERMISSION)?;
    let key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(ParameterMappingHandlerError::MissingIdempotencyKey)?;
    Ok(Json(map_parameter(&state, &ctx, &req, key).await?))
}

pub(crate) async fn map_parameter(
    state: &ParameterMappingAppState,
    ctx: &AuthContext,
    req: &MapParameterRequest,
    key: &str,
) -> Result<MapParameterResponse, ParameterMappingHandlerError> {
    validate_request(req)?;
    let request_hash = idempotency::request_hash(req).map_err(map_idempotency_error)?;
    let now = Utc::now();
    let mut tx = state.pool.begin().await.map_err(db_error)?;
    idempotency::lock_key(&mut tx, "wms-idempotency", ctx.owner_id, key)
        .await
        .map_err(map_idempotency_error)?;

    if let Some(response) =
        replay_idempotent(&mut tx, ctx.owner_id, key, &request_hash, now).await?
    {
        tx.commit().await.map_err(db_error)?;
        return Ok(response);
    }

    let response = execute_mapping(&mut tx, ctx, req).await?;
    let resource_id = response
        .rule_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| req.dict_code.clone());
    append_event_in_tx(
        &mut tx,
        &AuditWriteRequest::from_auth_context(
            ctx,
            "map_parameter",
            "M-PM",
            "parameter_mapping",
            resource_id.clone(),
            Some(AuditDiff::compute(
                serde_json::json!({
                    "dict_code": req.dict_code,
                    "source_system": req.source_system,
                    "source_digest": format!("{:x}", Sha256::digest(req.source_value.as_bytes())),
                }),
                serde_json::json!({
                    "status": response.status,
                    "target_value": response.target_value,
                    "rule_id": response.rule_id,
                    "queued": response.queued,
                }),
            )),
        ),
    )
    .await
    .map_err(|error| ParameterMappingHandlerError::Persistence(format!("{error:?}")))?;
    store_idempotent(
        &mut tx,
        ctx.owner_id,
        key,
        &request_hash,
        &resource_id,
        &response,
        now,
    )
    .await?;
    tx.commit().await.map_err(db_error)?;
    Ok(response)
}

fn validate_request(req: &MapParameterRequest) -> Result<(), ParameterMappingHandlerError> {
    let source_system_len = req.source_system.as_deref().unwrap_or("*").trim().len();
    if req.dict_code.trim().is_empty()
        || req.dict_code.len() > 128
        || req.source_value.trim().is_empty()
        || req.source_value.len() > 2_000
        || source_system_len == 0
        || source_system_len > 128
        || req
            .source_record_id
            .as_deref()
            .is_some_and(|value| value.len() > 256)
    {
        return Err(ParameterMappingHandlerError::InvalidRequest);
    }
    Ok(())
}

#[derive(FromRow)]
struct DictionaryRow {
    id: Uuid,
    target_values: Value,
    case_sensitive: bool,
    normalize_whitespace: bool,
    default_strategy: String,
    fallback_value: Option<String>,
}

#[derive(FromRow)]
struct RuleRow {
    id: Uuid,
    target_value: String,
    confidence: i32,
}

async fn execute_mapping(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    req: &MapParameterRequest,
) -> Result<MapParameterResponse, ParameterMappingHandlerError> {
    let dictionaries: Vec<DictionaryRow> = sqlx::query_as(
        r#"
        SELECT id, target_values, case_sensitive, normalize_whitespace,
               default_strategy, fallback_value
          FROM parameter_mapping_dictionaries
         WHERE dict_code = $1 AND enabled
           AND (owner_id = $2 OR owner_id IS NULL)
         ORDER BY (owner_id IS NOT NULL) DESC
        "#,
    )
    .bind(req.dict_code.trim())
    .bind(ctx.owner_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_error)?;
    let dictionary = dictionaries
        .first()
        .ok_or(ParameterMappingHandlerError::DictionaryNotFound)?;
    let dictionary_ids = dictionaries
        .iter()
        .map(|candidate| candidate.id)
        .collect::<Vec<_>>();
    let source_system = req.source_system.as_deref().unwrap_or("*").trim();
    let normalized = normalize_source(
        &req.source_value,
        dictionary.normalize_whitespace,
        dictionary.case_sensitive,
    );
    let matches: Vec<RuleRow> = sqlx::query_as(
        r#"
        SELECT r.id, r.target_value, r.confidence,
               (r.owner_id IS NOT NULL) AS owner_specific,
               (r.source_system = $3) AS source_specific,
               CASE r.match_type
                   WHEN 'exact' THEN 1
                   WHEN 'contains' THEN 2
                   WHEN 'wildcard' THEN 3
                   ELSE 4
               END AS match_rank,
               r.priority
          FROM parameter_mapping_rules r
         WHERE r.dictionary_id = ANY($1) AND r.enabled
           AND (r.owner_id = $2 OR r.owner_id IS NULL)
           AND (r.source_system = $3 OR r.source_system = '*')
           AND (r.effective_from IS NULL OR r.effective_from <= now())
           AND (r.effective_to IS NULL OR r.effective_to > now())
           AND CASE r.match_type
               WHEN 'exact' THEN r.normalized_source_pattern = $4
               WHEN 'contains' THEN position(r.normalized_source_pattern IN $4) > 0
               WHEN 'wildcard' THEN $4 LIKE replace(r.normalized_source_pattern, '*', '%')
               WHEN 'regex' THEN CASE WHEN $6 THEN $5 ~ r.source_pattern ELSE $5 ~* r.source_pattern END
               ELSE FALSE
           END
         ORDER BY owner_specific DESC, source_specific DESC, match_rank,
                  r.priority, r.created_at DESC
         LIMIT 1
        "#,
    )
    .bind(&dictionary_ids)
    .bind(ctx.owner_id)
    .bind(source_system)
    .bind(&normalized)
    .bind(req.source_value.trim())
    .bind(dictionary.case_sensitive)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_error)?;

    if let Some(first) = matches.first() {
        if !dictionaries
            .iter()
            .any(|candidate| target_is_allowed(&candidate.target_values, &first.target_value))
        {
            return Err(ParameterMappingHandlerError::Persistence(
                "mapping rule target is not in dictionary values".to_string(),
            ));
        }
        return Ok(MapParameterResponse {
            status: ParameterMappingStatus::Matched,
            target_value: Some(first.target_value.clone()),
            rule_id: Some(first.id),
            confidence: first.confidence,
            fallback_used: false,
            queued: false,
        });
    }

    if dictionary.default_strategy == "fallback" {
        let fallback = dictionary
            .fallback_value
            .clone()
            .filter(|value| target_is_allowed(&dictionary.target_values, value));
        if let Some(target_value) = fallback {
            return Ok(MapParameterResponse {
                status: ParameterMappingStatus::Matched,
                target_value: Some(target_value),
                rule_id: None,
                confidence: 0,
                fallback_used: true,
                queued: false,
            });
        }
        return Err(ParameterMappingHandlerError::Persistence(
            "mapping dictionary fallback is invalid".to_string(),
        ));
    }

    let queued = dictionary.default_strategy == "mark_unmapped";
    if queued {
        sqlx::query(
            r#"
            INSERT INTO parameter_mapping_queue (
                id, owner_id, dictionary_id, source_system, source_record_id,
                source_value, normalized_source_value
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (owner_id, dictionary_id, normalized_source_value)
            DO UPDATE SET occurrence_count = parameter_mapping_queue.occurrence_count + 1,
                          last_seen_at = now(),
                          source_system = EXCLUDED.source_system,
                          source_record_id = EXCLUDED.source_record_id
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(dictionary.id)
        .bind(source_system)
        .bind(req.source_record_id.as_deref())
        .bind(req.source_value.trim())
        .bind(&normalized)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    }
    Ok(MapParameterResponse {
        status: ParameterMappingStatus::Unmatched,
        target_value: None,
        rule_id: None,
        confidence: 0,
        fallback_used: false,
        queued,
    })
}

fn normalize_source(value: &str, whitespace: bool, case_sensitive: bool) -> String {
    let normalized = if whitespace {
        value
            .replace('（', "(")
            .replace('）', ")")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        value.to_string()
    };
    if case_sensitive {
        normalized
    } else {
        normalized.to_lowercase()
    }
}

fn target_is_allowed(target_values: &Value, target: &str) -> bool {
    target_values
        .as_array()
        .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(target)))
}

async fn replay_idempotent(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    now: chrono::DateTime<Utc>,
) -> Result<Option<MapParameterResponse>, ParameterMappingHandlerError> {
    idempotency::replay(tx, owner_id, key, request_hash, "POST", MAP_PATH, now)
        .await
        .map_err(map_idempotency_error)
}

async fn store_idempotent(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    resource_id: &str,
    response: &MapParameterResponse,
    now: chrono::DateTime<Utc>,
) -> Result<(), ParameterMappingHandlerError> {
    idempotency::store_success(
        tx,
        owner_id,
        key,
        request_hash,
        "POST",
        MAP_PATH,
        "parameter_mapping",
        resource_id,
        response,
        now,
    )
    .await
    .map_err(map_idempotency_error)
}

fn map_idempotency_error(error: idempotency::IdempotencyError) -> ParameterMappingHandlerError {
    match error {
        idempotency::IdempotencyError::Conflict => {
            ParameterMappingHandlerError::IdempotencyConflict
        }
        idempotency::IdempotencyError::Database(error) => db_error(error),
        idempotency::IdempotencyError::Serialize(error) => {
            ParameterMappingHandlerError::Persistence(error)
        }
    }
}

fn db_error(error: sqlx::Error) -> ParameterMappingHandlerError {
    ParameterMappingHandlerError::Persistence(error.to_string())
}
