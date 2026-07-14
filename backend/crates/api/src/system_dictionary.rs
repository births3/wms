//! US-M1-011 system dictionary first backend slice.

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    DisableSystemDictionaryItemRequest, SystemDictionaryCategory, SystemDictionaryImpactPreview,
    SystemDictionaryImpactReference, SystemDictionaryItem, UpsertSystemDictionaryItemRequest,
    DOCUMENT_TYPE_PURCHASE_INBOUND, DOCUMENT_TYPE_PURCHASE_RETURN_OUTBOUND,
    DOCUMENT_TYPE_SALES_OUTBOUND, DOCUMENT_TYPE_SALES_RETURN, SYSTEM_DICTIONARY_DOCUMENT_TYPE,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

mod system_dictionary_validation;

use system_dictionary_validation::{allowed_owner_params, validate_params};

#[derive(Clone, Debug)]
pub struct PgSystemDictionaryRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SystemDictionaryError {
    NotFound,
    InvalidScope,
    CrossOwnerAccess,
    InvalidEffectiveWindow,
    ParamInvalid { field: String, message: String },
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(FromRow)]
struct SystemDictionaryCategoryRow {
    dict_code: String,
    dict_name: String,
    enabled: bool,
    control_level: String,
    param_schema: Value,
    scope_mode: String,
    override_policy: Value,
    sort_order: i32,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, FromRow)]
struct SystemDictionaryItemRow {
    id: Uuid,
    dict_code: String,
    item_code: String,
    item_name: String,
    enabled: bool,
    owner_id: Option<Uuid>,
    params: Value,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    source: String,
    disabled_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgSystemDictionaryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_effective_items(
        &self,
        ctx: &AuthContext,
        dict_code: &str,
        effective_at: DateTime<Utc>,
    ) -> Result<Vec<SystemDictionaryItem>, SystemDictionaryError> {
        let category = self.load_enabled_category(dict_code).await?;
        let rows = sqlx::query_as::<_, SystemDictionaryItemRow>(
            r#"
            WITH scoped_items AS (
                SELECT
                    id,
                    dict_code,
                    item_code,
                    item_name,
                    enabled,
                    owner_id,
                    params,
                    effective_from,
                    effective_to,
                    source,
                    disabled_reason,
                    created_at,
                    updated_at,
                    ROW_NUMBER() OVER (
                        PARTITION BY item_code
                        ORDER BY
                            CASE WHEN owner_id = $2 THEN 1 ELSE 0 END DESC,
                            updated_at DESC
                    ) AS scope_rank
                  FROM system_dictionary_items
                 WHERE dict_code = $1
                   AND (owner_id IS NULL OR owner_id = $2)
                   AND (effective_from IS NULL OR effective_from <= $3)
                   AND (effective_to IS NULL OR effective_to > $3)
            )
            SELECT id, dict_code, item_code, item_name, enabled, owner_id, params,
                   effective_from, effective_to, source, disabled_reason, created_at, updated_at
              FROM scoped_items
             WHERE scope_rank = 1
               AND enabled = TRUE
             ORDER BY item_code
            "#,
        )
        .bind(dict_code)
        .bind(ctx.owner_id)
        .bind(effective_at)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        rows.into_iter()
            .map(|row| {
                validate_params(&category.param_schema, &row.params)?;
                Ok(row.into())
            })
            .collect()
    }

    pub async fn preview_item_impact(
        &self,
        ctx: &AuthContext,
        dict_code: &str,
        item_code: &str,
        owner_id: Uuid,
        effective_at: DateTime<Utc>,
    ) -> Result<SystemDictionaryImpactPreview, SystemDictionaryError> {
        ensure_request_owner(ctx, Some(owner_id))?;
        let items = self
            .list_effective_items(ctx, dict_code, effective_at)
            .await?;
        if !items.iter().any(|item| item.item_code == item_code) {
            return Err(SystemDictionaryError::NotFound);
        }

        let references = if dict_code == SYSTEM_DICTIONARY_DOCUMENT_TYPE {
            self.count_document_type_references(owner_id, item_code, effective_at)
                .await?
        } else {
            Vec::new()
        };
        let total_references = references
            .iter()
            .map(|reference| reference.reference_count)
            .sum();

        Ok(SystemDictionaryImpactPreview {
            dict_code: dict_code.to_string(),
            item_code: item_code.to_string(),
            owner_id,
            effective_at,
            total_references,
            references,
        })
    }

    pub async fn upsert_item(
        &self,
        ctx: &AuthContext,
        dict_code: &str,
        item_code: &str,
        req: UpsertSystemDictionaryItemRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<SystemDictionaryItem>, SystemDictionaryError> {
        if let (Some(from), Some(to)) = (req.effective_from, req.effective_to) {
            if to <= from {
                return Err(SystemDictionaryError::InvalidEffectiveWindow);
            }
        }
        ensure_request_owner(ctx, req.owner_id)?;

        let request_hash = request_hash(&serde_json::json!({
            "dict_code": dict_code,
            "item_code": item_code,
            "request": &req,
        }))?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }

        let category = load_enabled_category_in_tx(&mut tx, dict_code).await?;
        validate_scope(&category, req.owner_id)?;
        validate_params(&category.param_schema, &req.params)?;
        validate_owner_override_params(&mut tx, &category, item_code, &req).await?;

        let source = if req.owner_id.is_some() {
            "owner"
        } else {
            "global"
        };
        let existing = load_item_for_update(&mut tx, dict_code, item_code, req.owner_id).await?;
        let before = existing.clone().map(SystemDictionaryItem::from);
        let row = if let Some(existing) = existing {
            sqlx::query_as::<_, SystemDictionaryItemRow>(
                r#"
                UPDATE system_dictionary_items
                   SET item_name = $1,
                       enabled = $2,
                       params = $3,
                       effective_from = $4,
                       effective_to = $5,
                       source = $6,
                       disabled_reason = CASE WHEN $2 THEN NULL ELSE disabled_reason END,
                       updated_at = $7,
                       version = version + 1
                 WHERE id = $8 AND dict_code = $9 AND owner_id IS NOT DISTINCT FROM $10
                 RETURNING id, dict_code, item_code, item_name, enabled, owner_id, params,
                           effective_from, effective_to, source, disabled_reason,
                           created_at, updated_at
                "#,
            )
            .bind(&req.item_name)
            .bind(req.enabled)
            .bind(&req.params)
            .bind(req.effective_from)
            .bind(req.effective_to)
            .bind(source)
            .bind(now)
            .bind(existing.id)
            .bind(dict_code)
            .bind(req.owner_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        } else {
            sqlx::query_as::<_, SystemDictionaryItemRow>(
                r#"
                INSERT INTO system_dictionary_items (
                    id, dict_code, item_code, item_name, enabled, owner_id, params,
                    effective_from, effective_to, source, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
                RETURNING id, dict_code, item_code, item_name, enabled, owner_id, params,
                          effective_from, effective_to, source, disabled_reason,
                          created_at, updated_at
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(dict_code)
            .bind(item_code)
            .bind(&req.item_name)
            .bind(req.enabled)
            .bind(req.owner_id)
            .bind(&req.params)
            .bind(req.effective_from)
            .bind(req.effective_to)
            .bind(source)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        };
        let item = SystemDictionaryItem::from(row);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PUT",
            &format!("/api/v1/system-dictionaries/{dict_code}/items/{item_code}"),
            before.as_ref(),
            &item,
            "upsert_system_dictionary_item",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: item,
            replayed: false,
        })
    }

    pub async fn disable_item(
        &self,
        ctx: &AuthContext,
        dict_code: &str,
        item_code: &str,
        req: DisableSystemDictionaryItemRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<SystemDictionaryItem>, SystemDictionaryError> {
        let request_hash = request_hash(&serde_json::json!({
            "dict_code": dict_code,
            "item_code": item_code,
            "request": &req,
        }))?;
        ensure_request_owner(ctx, req.owner_id)?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }

        let category = load_enabled_category_in_tx(&mut tx, dict_code).await?;
        validate_scope(&category, req.owner_id)?;
        let existing = load_item_for_update(&mut tx, dict_code, item_code, req.owner_id).await?;
        if existing.is_none() {
            return Err(SystemDictionaryError::NotFound);
        }
        let before = existing.clone().map(SystemDictionaryItem::from);

        let row = sqlx::query_as::<_, SystemDictionaryItemRow>(
            r#"
            UPDATE system_dictionary_items
               SET enabled = FALSE,
                   disabled_reason = $1,
                   updated_at = $2,
                   version = version + 1
             WHERE dict_code = $3
               AND item_code = $4
               AND owner_id IS NOT DISTINCT FROM $5
             RETURNING id, dict_code, item_code, item_name, enabled, owner_id, params,
                       effective_from, effective_to, source, disabled_reason,
                       created_at, updated_at
            "#,
        )
        .bind(&req.disabled_reason)
        .bind(now)
        .bind(dict_code)
        .bind(item_code)
        .bind(req.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let item = SystemDictionaryItem::from(row);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable"),
            before.as_ref(),
            &item,
            "disable_system_dictionary_item",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: item,
            replayed: false,
        })
    }

    async fn load_enabled_category(
        &self,
        dict_code: &str,
    ) -> Result<SystemDictionaryCategory, SystemDictionaryError> {
        let mut tx = self.begin().await?;
        let category = load_enabled_category_in_tx(&mut tx, dict_code).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(category)
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, SystemDictionaryError> {
        self.pool.begin().await.map_err(map_db_error)
    }

    async fn count_document_type_references(
        &self,
        owner_id: Uuid,
        item_code: &str,
        effective_at: DateTime<Utc>,
    ) -> Result<Vec<SystemDictionaryImpactReference>, SystemDictionaryError> {
        let mut references = Vec::new();

        if matches!(
            item_code,
            DOCUMENT_TYPE_PURCHASE_INBOUND | DOCUMENT_TYPE_SALES_RETURN
        ) {
            let reference_count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*)::BIGINT
                  FROM receiving_orders
                 WHERE owner_id = $1
                   AND document_type = $2
                   AND created_at <= $3
                "#,
            )
            .bind(owner_id)
            .bind(item_code)
            .bind(effective_at)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
            push_reference(&mut references, "M2", "receiving_orders", reference_count);
        }

        match item_code {
            DOCUMENT_TYPE_SALES_OUTBOUND => {
                let reference_count: i64 = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*)::BIGINT
                      FROM outbound_orders
                     WHERE owner_id = $1
                       AND created_at <= $2
                    "#,
                )
                .bind(owner_id)
                .bind(effective_at)
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_error)?;
                push_reference(&mut references, "M4", "outbound_orders", reference_count);
            }
            DOCUMENT_TYPE_PURCHASE_RETURN_OUTBOUND => {}
            _ => {}
        }

        Ok(references)
    }
}

fn push_reference(
    references: &mut Vec<SystemDictionaryImpactReference>,
    module_code: &str,
    business_object: &str,
    reference_count: i64,
) {
    if reference_count == 0 {
        return;
    }
    references.push(SystemDictionaryImpactReference {
        module_code: module_code.to_string(),
        business_object: business_object.to_string(),
        reference_count,
    });
}

async fn load_enabled_category_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    dict_code: &str,
) -> Result<SystemDictionaryCategory, SystemDictionaryError> {
    let row = sqlx::query_as::<_, SystemDictionaryCategoryRow>(
        r#"
        SELECT dict_code, dict_name, enabled, control_level, param_schema, scope_mode,
               override_policy, sort_order, remark, created_at, updated_at
          FROM system_dictionary_categories
         WHERE dict_code = $1
        "#,
    )
    .bind(dict_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(SystemDictionaryError::NotFound)?;
    if !row.enabled {
        return Err(SystemDictionaryError::NotFound);
    }
    Ok(row.into())
}

async fn load_item_for_update(
    tx: &mut Transaction<'_, Postgres>,
    dict_code: &str,
    item_code: &str,
    owner_id: Option<Uuid>,
) -> Result<Option<SystemDictionaryItemRow>, SystemDictionaryError> {
    sqlx::query_as::<_, SystemDictionaryItemRow>(
        r#"
        SELECT id, dict_code, item_code, item_name, enabled, owner_id, params,
               effective_from, effective_to, source, disabled_reason, created_at, updated_at
          FROM system_dictionary_items
         WHERE dict_code = $1
           AND item_code = $2
           AND owner_id IS NOT DISTINCT FROM $3
         FOR UPDATE
        "#,
    )
    .bind(dict_code)
    .bind(item_code)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub(crate) async fn effective_item_enabled_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    dict_code: &str,
    item_code: &str,
    effective_at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        WITH scoped_items AS (
            SELECT item.enabled,
                   ROW_NUMBER() OVER (
                       ORDER BY
                           CASE WHEN item.owner_id = $1 THEN 1 ELSE 0 END DESC,
                           item.updated_at DESC
                   ) AS scope_rank
              FROM system_dictionary_items item
              JOIN system_dictionary_categories category
                ON category.dict_code = item.dict_code
               AND category.enabled = TRUE
             WHERE item.dict_code = $2
               AND item.item_code = $3
               AND (item.owner_id IS NULL OR item.owner_id = $1)
               AND (item.effective_from IS NULL OR item.effective_from <= $4)
               AND (item.effective_to IS NULL OR item.effective_to > $4)
        )
        SELECT EXISTS (
            SELECT 1
              FROM scoped_items
             WHERE scope_rank = 1
               AND enabled = TRUE
        )
        "#,
    )
    .bind(owner_id)
    .bind(dict_code)
    .bind(item_code)
    .bind(effective_at)
    .fetch_one(&mut **tx)
    .await
}

async fn validate_owner_override_params(
    tx: &mut Transaction<'_, Postgres>,
    category: &SystemDictionaryCategory,
    item_code: &str,
    req: &UpsertSystemDictionaryItemRequest,
) -> Result<(), SystemDictionaryError> {
    if category.scope_mode != "owner_override" || req.owner_id.is_none() {
        return Ok(());
    }
    let Some(global) = load_item_for_update(tx, &category.dict_code, item_code, None).await? else {
        return Err(SystemDictionaryError::NotFound);
    };
    let allowed = allowed_owner_params(&category.override_policy);
    let Some(params) = req.params.as_object() else {
        return Err(SystemDictionaryError::ParamInvalid {
            field: "$".to_string(),
            message: "params 必须是 JSON object".to_string(),
        });
    };
    for (field, value) in params {
        if allowed.contains(field) {
            continue;
        }
        if global.params.get(field) != Some(value) {
            return Err(SystemDictionaryError::ParamInvalid {
                field: field.clone(),
                message: "该参数不允许货主覆盖".to_string(),
            });
        }
    }
    Ok(())
}

fn validate_scope(
    category: &SystemDictionaryCategory,
    owner_id: Option<Uuid>,
) -> Result<(), SystemDictionaryError> {
    match (category.scope_mode.as_str(), owner_id) {
        ("global_only", Some(_)) => Err(SystemDictionaryError::InvalidScope),
        ("global_only" | "owner_extensible" | "owner_override", _) => Ok(()),
        _ => Err(SystemDictionaryError::InvalidScope),
    }
}

fn ensure_request_owner(
    ctx: &AuthContext,
    owner_id: Option<Uuid>,
) -> Result<(), SystemDictionaryError> {
    match owner_id {
        Some(owner_id) if owner_id != ctx.owner_id => Err(SystemDictionaryError::CrossOwnerAccess),
        _ => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn finish_mutation<T: Serialize>(
    mut tx: Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    before: Option<&SystemDictionaryItem>,
    response: &T,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), SystemDictionaryError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| SystemDictionaryError::Serialize(error.to_string()))?;
    let before_value = before
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| SystemDictionaryError::Serialize(error.to_string()))?
        .unwrap_or(Value::Null);
    let audit_diff = AuditDiff::compute(before_value, response_body.clone());
    let resource_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("system_dictionary_item")
        .to_string();
    store_idempotency_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "system_dictionary_item",
        resource_id.clone(),
        response,
        now,
    )
    .await?;
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M1",
        "system_dictionary_item",
        resource_id,
        Some(audit_diff),
    );
    audit.occurred_at = now;
    append_event_in_tx(&mut tx, &audit)
        .await
        .map_err(|error| SystemDictionaryError::Audit(format!("{error:?}")))?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(())
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, SystemDictionaryError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(SystemDictionaryError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| SystemDictionaryError::Serialize(error.to_string()))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), SystemDictionaryError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(idempotency_lock_id(owner_id, idempotency_key))
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), SystemDictionaryError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| SystemDictionaryError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

fn request_hash(value: &Value) -> Result<String, SystemDictionaryError> {
    let text = serde_json::to_string(value)
        .map_err(|error| SystemDictionaryError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn idempotency_lock_id(owner_id: Uuid, idempotency_key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update([0]);
    hasher.update(idempotency_key.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn map_db_error(error: sqlx::Error) -> SystemDictionaryError {
    SystemDictionaryError::Database(error.to_string())
}

impl From<SystemDictionaryCategoryRow> for SystemDictionaryCategory {
    fn from(row: SystemDictionaryCategoryRow) -> Self {
        Self {
            dict_code: row.dict_code,
            dict_name: row.dict_name,
            enabled: row.enabled,
            control_level: row.control_level,
            param_schema: row.param_schema,
            scope_mode: row.scope_mode,
            override_policy: row.override_policy,
            sort_order: row.sort_order,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<SystemDictionaryItemRow> for SystemDictionaryItem {
    fn from(row: SystemDictionaryItemRow) -> Self {
        Self {
            id: row.id,
            dict_code: row.dict_code,
            item_code: row.item_code,
            item_name: row.item_name,
            enabled: row.enabled,
            owner_id: row.owner_id,
            params: row.params,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            source: row.source,
            disabled_reason: row.disabled_reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[cfg(test)]
#[path = "system_dictionary_tests.rs"]
mod tests;
