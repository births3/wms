//! US-M1-011 system dictionary first backend slice.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    DisableSystemDictionaryItemRequest, SystemDictionaryCategory, SystemDictionaryImpactPreview,
    SystemDictionaryImpactReference, SystemDictionaryItem, UpsertSystemDictionaryItemRequest,
    DOCUMENT_TYPE_PURCHASE_INBOUND, DOCUMENT_TYPE_PURCHASE_RETURN_OUTBOUND,
    DOCUMENT_TYPE_SALES_OUTBOUND, DOCUMENT_TYPE_SALES_RETURN, SYSTEM_DICTIONARY_DOCUMENT_TYPE,
    SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    idempotency::{self, IdempotencyError},
};

const IDEMPOTENCY_NAMESPACE: &str = "system-dictionary";

mod system_dictionary_rows;
mod system_dictionary_validation;

use system_dictionary_rows::{SystemDictionaryCategoryRow, SystemDictionaryItemRow};
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
    PrintTemplateFieldLibraryRequired,
    ParamInvalid { field: String, message: String },
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<IdempotencyError> for SystemDictionaryError {
    fn from(error: IdempotencyError) -> Self {
        match error {
            IdempotencyError::Conflict => Self::IdempotencyConflict,
            IdempotencyError::Database(error) => Self::Database(error.to_string()),
            IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
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
                    sort_order,
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
            SELECT id, dict_code, item_code, item_name, enabled, owner_id, sort_order, params,
                   effective_from, effective_to, source, disabled_reason, created_at, updated_at
              FROM scoped_items
             WHERE scope_rank = 1
               AND enabled = TRUE
             ORDER BY sort_order, item_code
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
        if req.sort_order < 0 {
            return Err(SystemDictionaryError::ParamInvalid {
                field: "sort_order".to_string(),
                message: "排序号必须是非负整数".to_string(),
            });
        }
        if dict_code == SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE
            && req.enabled
            && req
                .params
                .get("field_library_code")
                .and_then(Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(SystemDictionaryError::PrintTemplateFieldLibraryRequired);
        }
        ensure_request_owner(ctx, req.owner_id)?;

        let request_hash = idempotency::request_hash(&serde_json::json!({
            "dict_code": dict_code,
            "item_code": item_code,
            "request": &req,
        }))?;
        let mut tx = self.begin().await?;
        idempotency::lock_key(
            &mut tx,
            IDEMPOTENCY_NAMESPACE,
            ctx.owner_id,
            idempotency_key,
        )
        .await?;
        let path = format!("/api/v1/system-dictionaries/{dict_code}/items/{item_code}");
        if let Some(value) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            &path,
            now,
        )
        .await?
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
                       sort_order = $3,
                       params = $4,
                       effective_from = $5,
                       effective_to = $6,
                       source = $7,
                       disabled_reason = CASE WHEN $2 THEN NULL ELSE disabled_reason END,
                       updated_at = $8,
                       version = version + 1
                 WHERE id = $9 AND dict_code = $10 AND owner_id IS NOT DISTINCT FROM $11
                 RETURNING id, dict_code, item_code, item_name, enabled, owner_id, sort_order, params,
                           effective_from, effective_to, source, disabled_reason,
                           created_at, updated_at
                "#,
            )
            .bind(&req.item_name)
            .bind(req.enabled)
            .bind(req.sort_order)
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
                    id, dict_code, item_code, item_name, enabled, owner_id, sort_order, params,
                    effective_from, effective_to, source, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
                RETURNING id, dict_code, item_code, item_name, enabled, owner_id, sort_order, params,
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
            .bind(req.sort_order)
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
            &path,
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
        let request_hash = idempotency::request_hash(&serde_json::json!({
            "dict_code": dict_code,
            "item_code": item_code,
            "request": &req,
        }))?;
        ensure_request_owner(ctx, req.owner_id)?;
        let mut tx = self.begin().await?;
        idempotency::lock_key(
            &mut tx,
            IDEMPOTENCY_NAMESPACE,
            ctx.owner_id,
            idempotency_key,
        )
        .await?;
        let path = format!("/api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable");
        if let Some(value) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &path,
            now,
        )
        .await?
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
             RETURNING id, dict_code, item_code, item_name, enabled, owner_id, sort_order, params,
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
            &path,
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
        SELECT id, dict_code, item_code, item_name, enabled, owner_id, sort_order, params,
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
    idempotency::store_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "system_dictionary_item",
        &resource_id,
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

fn map_db_error(error: sqlx::Error) -> SystemDictionaryError {
    SystemDictionaryError::Database(error.to_string())
}

#[cfg(test)]
#[path = "system_dictionary_tests.rs"]
mod tests;
