use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{DualPersonPolicy, DualPersonPolicyRule, UpsertDualPersonPolicyRuleRequest};

use crate::idempotency;

use super::{map_database_error, DualPersonPolicyError, PolicyRuleRow};

const IDEMPOTENCY_METHOD: &str = "PUT";
const IDEMPOTENCY_PATH: &str = "/api/v1/m-vr/dual-person-policy/rules";

pub(super) fn row_to_domain(
    row: PolicyRuleRow,
) -> Result<DualPersonPolicyRule, DualPersonPolicyError> {
    Ok(DualPersonPolicyRule {
        id: row.id,
        special_drug_category: row.special_drug_category,
        process: row.process_code,
        node: row.node_code,
        owner_id: row.owner_id,
        warehouse_id: row.warehouse_id,
        policy: DualPersonPolicy::try_from(row.policy.as_str())
            .map_err(|_| DualPersonPolicyError::Database("数据库包含非法双人策略".to_string()))?,
        priority: row.priority,
        enabled: row.enabled,
        confirmed_by_user_id: row.confirmed_by_user_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
        version: row.version,
    })
}

pub(super) async fn load_rule_for_update(
    tx: &mut Transaction<'_, Postgres>,
    category: &str,
    process: &str,
    node: &str,
    owner_id: Option<Uuid>,
    warehouse_id: Option<Uuid>,
) -> Result<Option<PolicyRuleRow>, DualPersonPolicyError> {
    sqlx::query_as::<_, PolicyRuleRow>(
        r#"
        SELECT id, special_drug_category, process_code, node_code, owner_id,
               warehouse_id, policy, priority, enabled, confirmed_by_user_id,
               created_at, updated_at, version
          FROM dual_person_policy_rules
         WHERE special_drug_category = $1 AND process_code = $2 AND node_code = $3
           AND owner_id IS NOT DISTINCT FROM $4
           AND warehouse_id IS NOT DISTINCT FROM $5
         FOR UPDATE
        "#,
    )
    .bind(category)
    .bind(process)
    .bind(node)
    .bind(owner_id)
    .bind(warehouse_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

pub(super) async fn sync_dictionary_matrix_cell(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Option<Uuid>,
    request: &UpsertDualPersonPolicyRuleRequest,
    now: DateTime<Utc>,
) -> Result<Uuid, DualPersonPolicyError> {
    if let Some(owner_id) = owner_id {
        sqlx::query(
            r#"
            INSERT INTO system_dictionary_items (
                id, dict_code, item_code, item_name, enabled, owner_id, params,
                effective_from, effective_to, source, created_at, updated_at
            )
            SELECT $1, dict_code, item_code, item_name, TRUE, $2, params,
                   effective_from, effective_to, 'owner', $3, $3
              FROM system_dictionary_items
             WHERE dict_code = 'special_drug_category' AND item_code = $4 AND owner_id IS NULL
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(now)
        .bind(&request.special_drug_category)
        .execute(&mut **tx)
        .await
        .map_err(map_database_error)?;
    }

    sqlx::query_scalar::<_, Uuid>(
        r#"
        UPDATE system_dictionary_items item
           SET params = jsonb_set(
                   params,
                   '{requires_dual_person_matrix}',
                   CASE
                       WHEN EXISTS (
                           SELECT 1
                             FROM jsonb_array_elements(
                                      COALESCE(params -> 'requires_dual_person_matrix', '[]'::jsonb)
                                  ) cell
                            WHERE cell ->> 'process' = $3 AND cell ->> 'node' = $4
                       ) THEN (
                           SELECT jsonb_agg(
                               CASE
                                   WHEN cell ->> 'process' = $3 AND cell ->> 'node' = $4
                                   THEN jsonb_build_object('process', $3, 'node', $4, 'policy', $5)
                                   ELSE cell
                               END
                               ORDER BY ordinal
                           )
                             FROM jsonb_array_elements(
                                      COALESCE(params -> 'requires_dual_person_matrix', '[]'::jsonb)
                                  ) WITH ORDINALITY AS matrix(cell, ordinal)
                       )
                       ELSE COALESCE(params -> 'requires_dual_person_matrix', '[]'::jsonb)
                            || jsonb_build_array(
                                   jsonb_build_object(
                                       'process', $3, 'node', $4, 'policy', $5
                                   )
                               )
                   END,
                   TRUE
               ),
               updated_at = $6,
               version = version + 1
         WHERE dict_code = 'special_drug_category'
           AND item_code = $1
           AND owner_id IS NOT DISTINCT FROM $2
         RETURNING id
        "#,
    )
    .bind(&request.special_drug_category)
    .bind(owner_id)
    .bind(&request.process)
    .bind(&request.node)
    .bind(request.policy.as_str())
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?
    .ok_or(DualPersonPolicyError::CategoryNotFound)
}

pub(super) async fn lock_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
) -> Result<(), DualPersonPolicyError> {
    idempotency::lock_key(tx, "dual-person-policy", owner_id, key)
        .await
        .map_err(Into::into)
}

pub(super) fn request_hash<T: Serialize>(value: &T) -> Result<String, DualPersonPolicyError> {
    idempotency::request_hash(value).map_err(Into::into)
}

pub(super) fn json_value<T: Serialize>(value: &T) -> Result<Value, DualPersonPolicyError> {
    serde_json::to_value(value).map_err(|error| DualPersonPolicyError::Serialize(error.to_string()))
}

pub(super) async fn replay_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<DualPersonPolicyRule>, DualPersonPolicyError> {
    idempotency::replay(
        tx,
        owner_id,
        key,
        request_hash,
        IDEMPOTENCY_METHOD,
        IDEMPOTENCY_PATH,
        now,
    )
    .await
    .map_err(Into::into)
}

pub(super) async fn store_idempotency_success(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    value: &DualPersonPolicyRule,
    now: DateTime<Utc>,
) -> Result<(), DualPersonPolicyError> {
    idempotency::store_success(
        tx,
        owner_id,
        key,
        request_hash,
        IDEMPOTENCY_METHOD,
        IDEMPOTENCY_PATH,
        "dual_person_policy_rule",
        &value.id.to_string(),
        value,
        now,
    )
    .await
    .map_err(Into::into)
}
