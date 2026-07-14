use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    DualPersonPolicy, DualPersonPolicyResponse, DualPersonPolicyRule, DualPersonPolicyScope,
    ResolveDualPersonPolicyQuery, UpsertDualPersonPolicyRuleRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

mod persistence;

use self::persistence::{
    json_value, load_rule_for_update, lock_key, replay_idempotency, request_hash, row_to_domain,
    store_idempotency_success, sync_dictionary_matrix_cell,
};

#[derive(Clone, Debug)]
pub struct PgDualPersonPolicyRepository {
    pool: PgPool,
    cache: Option<redis::aio::MultiplexedConnection>,
}

const POLICY_CACHE_TTL_SECONDS: i64 = 600;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DualPersonPolicyError {
    CrossOwner,
    InvalidProcessNode,
    InvalidRule,
    ProductNotFound,
    WarehouseNotFound,
    CategoryNotFound,
    SamePerson,
    UnqualifiedConfirmer,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Debug, FromRow)]
struct PolicyRow {
    id: Uuid,
    policy: String,
}

#[derive(Clone, Debug, FromRow)]
struct PolicyRuleRow {
    id: Uuid,
    special_drug_category: String,
    process_code: String,
    node_code: String,
    owner_id: Option<Uuid>,
    warehouse_id: Option<Uuid>,
    policy: String,
    priority: i32,
    enabled: bool,
    confirmed_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentDualPersonPolicyMutation {
    pub value: DualPersonPolicyRule,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedDualPersonPolicy {
    pub policy: DualPersonPolicy,
    pub source_rule_id: Option<Uuid>,
}

pub(crate) const DUAL_PERSON_APPROVAL_SCENARIO: &str = "mvr.dual_person";

impl PgDualPersonPolicyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, cache: None }
    }

    pub fn with_redis_cache(pool: PgPool, cache: redis::aio::MultiplexedConnection) -> Self {
        Self {
            pool,
            cache: Some(cache),
        }
    }

    pub async fn resolve(
        &self,
        ctx: &AuthContext,
        query: &ResolveDualPersonPolicyQuery,
    ) -> Result<DualPersonPolicyResponse, DualPersonPolicyError> {
        if query.owner_id != ctx.owner_id {
            return Err(DualPersonPolicyError::CrossOwner);
        }
        if !valid_process_node(&query.process, &query.node) {
            return Err(DualPersonPolicyError::InvalidProcessNode);
        }
        if let Some(value) = self.read_cache(query).await {
            return Ok(value);
        }
        if let Some(warehouse_id) = query.warehouse_id {
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM warehouses WHERE id = $1 AND owner_id = $2)",
            )
            .bind(warehouse_id)
            .bind(ctx.owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_database_error)?;
            if !exists {
                return Err(DualPersonPolicyError::WarehouseNotFound);
            }
        }

        let category = sqlx::query_scalar::<_, String>(
            "SELECT special_drug_category FROM products WHERE id = $1 AND owner_id = $2 AND status = 'active'",
        )
        .bind(query.product_id)
        .bind(ctx.owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_database_error)?
        .ok_or(DualPersonPolicyError::ProductNotFound)?;

        let row = sqlx::query_as::<_, PolicyRow>(
            r#"
            SELECT id, policy
              FROM dual_person_policy_rules
             WHERE special_drug_category = $1
               AND process_code = $2
               AND node_code = $3
               AND enabled
               AND (owner_id IS NULL OR owner_id = $4)
               AND (warehouse_id IS NULL OR warehouse_id = $5)
             ORDER BY CASE
                        WHEN owner_id = $4 AND warehouse_id IS NULL THEN 3
                        WHEN warehouse_id = $5 THEN 2
                        ELSE 1
                      END DESC,
                      priority DESC,
                      updated_at DESC,
                      id
             LIMIT 1
            "#,
        )
        .bind(category)
        .bind(&query.process)
        .bind(&query.node)
        .bind(ctx.owner_id)
        .bind(query.warehouse_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_database_error)?;

        let (policy, source_rule_id) = match row {
            Some(row) => (
                DualPersonPolicy::try_from(row.policy.as_str()).map_err(|_| {
                    DualPersonPolicyError::Database("数据库包含非法双人策略".to_string())
                })?,
                Some(row.id),
            ),
            None => (DualPersonPolicy::Single, None),
        };
        let value = DualPersonPolicyResponse {
            policy,
            source_rule_id,
            process: query.process.clone(),
            node: query.node.clone(),
        };
        self.write_cache(query, &value).await;
        Ok(value)
    }

    pub async fn list(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<Uuid>,
    ) -> Result<Vec<DualPersonPolicyRule>, DualPersonPolicyError> {
        if let Some(warehouse_id) = warehouse_id {
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM warehouses WHERE id = $1 AND owner_id = $2)",
            )
            .bind(warehouse_id)
            .bind(ctx.owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_database_error)?;
            if !exists {
                return Err(DualPersonPolicyError::WarehouseNotFound);
            }
        }
        sqlx::query_as::<_, PolicyRuleRow>(
            r#"
            SELECT id, special_drug_category, process_code, node_code, owner_id,
                   warehouse_id, policy, priority, enabled, confirmed_by_user_id,
                   created_at, updated_at, version
              FROM dual_person_policy_rules
             WHERE (owner_id IS NULL OR owner_id = $1)
               AND (warehouse_id IS NULL OR warehouse_id = $2)
             ORDER BY special_drug_category, process_code, node_code,
                      CASE
                        WHEN owner_id = $1 AND warehouse_id IS NULL THEN 3
                        WHEN warehouse_id = $2 THEN 2
                        ELSE 1
                      END DESC,
                      priority DESC,
                      id
            "#,
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_database_error)?
        .into_iter()
        .map(row_to_domain)
        .collect()
    }

    pub async fn upsert(
        &self,
        ctx: &AuthContext,
        mut request: UpsertDualPersonPolicyRuleRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentDualPersonPolicyMutation, DualPersonPolicyError> {
        request.special_drug_category = request.special_drug_category.trim().to_string();
        request.process = request.process.trim().to_string();
        request.node = request.node.trim().to_string();
        validate_rule_request(ctx, &request)?;
        self.ensure_confirmer(ctx, request.confirmed_by_user_id)
            .await?;
        self.ensure_rule_references(ctx, &request).await?;

        let hash = request_hash(&request)?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentDualPersonPolicyMutation {
                value,
                replayed: true,
            });
        }

        let (rule_owner_id, warehouse_id, source_dictionary_item_id) = match request.scope {
            DualPersonPolicyScope::Global => {
                let item_id = sync_dictionary_matrix_cell(&mut tx, None, &request, now).await?;
                (None, None, Some(item_id))
            }
            DualPersonPolicyScope::Owner => {
                let item_id =
                    sync_dictionary_matrix_cell(&mut tx, Some(ctx.owner_id), &request, now).await?;
                (Some(ctx.owner_id), None, Some(item_id))
            }
            DualPersonPolicyScope::Warehouse => (Some(ctx.owner_id), request.warehouse_id, None),
        };

        let before = load_rule_for_update(
            &mut tx,
            &request.special_drug_category,
            &request.process,
            &request.node,
            rule_owner_id,
            warehouse_id,
        )
        .await?;
        let row = if let Some(existing) = before.as_ref() {
            sqlx::query_as::<_, PolicyRuleRow>(
                r#"
                UPDATE dual_person_policy_rules
                   SET policy = $1, priority = $2, enabled = $3,
                       source_dictionary_item_id = COALESCE($4, source_dictionary_item_id),
                       confirmed_by_user_id = $5, updated_at = $6, version = version + 1
                 WHERE id = $7
                 RETURNING id, special_drug_category, process_code, node_code, owner_id,
                           warehouse_id, policy, priority, enabled, confirmed_by_user_id,
                           created_at, updated_at, version
                "#,
            )
            .bind(request.policy.as_str())
            .bind(request.priority)
            .bind(request.enabled)
            .bind(source_dictionary_item_id)
            .bind(request.confirmed_by_user_id)
            .bind(now)
            .bind(existing.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_database_error)?
        } else {
            sqlx::query_as::<_, PolicyRuleRow>(
                r#"
                INSERT INTO dual_person_policy_rules (
                    id, special_drug_category, process_code, node_code, owner_id,
                    warehouse_id, policy, priority, enabled, source_dictionary_item_id,
                    confirmed_by_user_id, created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$12)
                RETURNING id, special_drug_category, process_code, node_code, owner_id,
                          warehouse_id, policy, priority, enabled, confirmed_by_user_id,
                          created_at, updated_at, version
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(&request.special_drug_category)
            .bind(&request.process)
            .bind(&request.node)
            .bind(rule_owner_id)
            .bind(warehouse_id)
            .bind(request.policy.as_str())
            .bind(request.priority)
            .bind(request.enabled)
            .bind(source_dictionary_item_id)
            .bind(request.confirmed_by_user_id)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_database_error)?
        };
        let value = row_to_domain(row)?;

        store_idempotency_success(&mut tx, ctx.owner_id, idempotency_key, &hash, &value, now)
            .await?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "upsert_dual_person_policy_rule",
            "M-VR",
            "dual_person_policy_rule",
            value.id.to_string(),
            Some(AuditDiff::compute(
                before
                    .map(row_to_domain)
                    .transpose()?
                    .map_or_else(|| Ok(serde_json::json!({})), |item| json_value(&item))?,
                json_value(&value)?,
            )),
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DualPersonPolicyError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_database_error)?;
        invalidate_policy_cache(
            &self.pool,
            self.cache.clone(),
            (!matches!(request.scope, DualPersonPolicyScope::Global)).then_some(ctx.owner_id),
        )
        .await;

        Ok(IdempotentDualPersonPolicyMutation {
            value,
            replayed: false,
        })
    }

    async fn ensure_confirmer(
        &self,
        ctx: &AuthContext,
        confirmer_id: Uuid,
    ) -> Result<(), DualPersonPolicyError> {
        if confirmer_id == ctx.user_id {
            return Err(DualPersonPolicyError::SamePerson);
        }
        let qualified = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM auth_users user_row
                  JOIN auth_user_owner_bindings binding
                    ON binding.user_id = user_row.id AND binding.owner_id = $2 AND binding.is_active
                  JOIN auth_user_roles user_role
                    ON user_role.user_id = user_row.id AND user_role.owner_id = $2
                  JOIN auth_role_permissions role_permission ON role_permission.role_id = user_role.role_id
                  JOIN auth_permissions permission ON permission.id = role_permission.permission_id
                 WHERE user_row.id = $1
                   AND user_row.status = 'active'
                   AND permission.permission_code = 'mvr.dual_person.write'
            )
            "#,
        )
        .bind(confirmer_id)
        .bind(ctx.owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_database_error)?;
        if !qualified {
            return Err(DualPersonPolicyError::UnqualifiedConfirmer);
        }
        Ok(())
    }

    async fn ensure_rule_references(
        &self,
        ctx: &AuthContext,
        request: &UpsertDualPersonPolicyRuleRequest,
    ) -> Result<(), DualPersonPolicyError> {
        let category_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM system_dictionary_items
                 WHERE dict_code = 'special_drug_category'
                   AND item_code = $1
                   AND enabled
                   AND (owner_id IS NULL OR owner_id = $2)
            )
            "#,
        )
        .bind(&request.special_drug_category)
        .bind(ctx.owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_database_error)?;
        if !category_exists {
            return Err(DualPersonPolicyError::CategoryNotFound);
        }
        if let Some(warehouse_id) = request.warehouse_id {
            let warehouse_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM warehouses WHERE id = $1 AND owner_id = $2)",
            )
            .bind(warehouse_id)
            .bind(ctx.owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_database_error)?;
            if !warehouse_exists {
                return Err(DualPersonPolicyError::WarehouseNotFound);
            }
        }
        Ok(())
    }

    async fn read_cache(
        &self,
        query: &ResolveDualPersonPolicyQuery,
    ) -> Option<DualPersonPolicyResponse> {
        let mut connection = self.cache.clone()?;
        let result: redis::RedisResult<Option<String>> = connection
            .hget(cache_key(query.owner_id), cache_field(query))
            .await;
        match result {
            Ok(Some(value)) => match serde_json::from_str(&value) {
                Ok(value) => Some(value),
                Err(error) => {
                    tracing::warn!(error = ?error, "invalid M-VR policy cache value; falling back to PostgreSQL");
                    None
                }
            },
            Ok(None) => None,
            Err(error) => {
                tracing::warn!(error = ?error, "M-VR policy cache unavailable; falling back to PostgreSQL");
                None
            }
        }
    }

    async fn write_cache(
        &self,
        query: &ResolveDualPersonPolicyQuery,
        value: &DualPersonPolicyResponse,
    ) {
        let Some(mut connection) = self.cache.clone() else {
            return;
        };
        let serialized = match serde_json::to_string(value) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(error = ?error, "failed to serialize M-VR policy cache value");
                return;
            }
        };
        let key = cache_key(query.owner_id);
        let write: redis::RedisResult<()> =
            connection.hset(&key, cache_field(query), serialized).await;
        if let Err(error) = write {
            tracing::warn!(error = ?error, "failed to populate M-VR policy cache");
            return;
        }
        let expire: redis::RedisResult<bool> = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(POLICY_CACHE_TTL_SECONDS)
            .arg("NX")
            .query_async(&mut connection)
            .await;
        if let Err(error) = expire {
            tracing::warn!(error = ?error, "failed to set M-VR policy cache TTL");
        }
    }
}

pub(crate) async fn invalidate_policy_cache(
    pool: &PgPool,
    cache: Option<redis::aio::MultiplexedConnection>,
    owner_id: Option<Uuid>,
) {
    let Some(mut connection) = cache else {
        return;
    };
    let owner_ids = if let Some(owner_id) = owner_id {
        vec![owner_id]
    } else {
        match sqlx::query_scalar::<_, Uuid>("SELECT id FROM auth_owners")
            .fetch_all(pool)
            .await
        {
            Ok(owner_ids) => owner_ids,
            Err(error) => {
                tracing::warn!(error = ?error, "failed to enumerate M-VR cache owners");
                return;
            }
        }
    };
    let keys = owner_ids.into_iter().map(cache_key).collect::<Vec<_>>();
    if keys.is_empty() {
        return;
    }
    let result: redis::RedisResult<usize> = connection.del(keys).await;
    if let Err(error) = result {
        tracing::warn!(error = ?error, "failed to invalidate M-VR policy cache");
    }
}

fn cache_key(owner_id: Uuid) -> String {
    format!("vr:matrix:{owner_id}")
}

fn cache_field(query: &ResolveDualPersonPolicyQuery) -> String {
    format!(
        "{}:{}:{}:{}",
        query
            .warehouse_id
            .map_or_else(|| "owner".to_string(), |id| id.to_string()),
        query.product_id,
        query.process,
        query.node
    )
}

fn validate_rule_request(
    ctx: &AuthContext,
    request: &UpsertDualPersonPolicyRuleRequest,
) -> Result<(), DualPersonPolicyError> {
    if request.special_drug_category.is_empty()
        || !valid_process_node(&request.process, &request.node)
        || !(0..=1000).contains(&request.priority)
        || matches!(request.scope, DualPersonPolicyScope::Warehouse)
            != request.warehouse_id.is_some()
        || matches!(request.scope, DualPersonPolicyScope::Global)
            && !ctx
                .permissions
                .iter()
                .any(|value| value == "mvr.dual_person.global.write")
    {
        return Err(DualPersonPolicyError::InvalidRule);
    }
    Ok(())
}

pub fn valid_process_node(process: &str, node: &str) -> bool {
    matches!(
        (process, node),
        ("入库", "收货" | "验收" | "上架")
            | ("出库", "拣货" | "复核" | "装箱" | "发货交接")
            | ("报损", "报损执行")
            | ("报溢", "报溢执行")
            | ("销毁", "销毁执行")
            | ("退货", "退货验收" | "退货上架")
    )
}

fn map_database_error(error: sqlx::Error) -> DualPersonPolicyError {
    DualPersonPolicyError::Database(error.to_string())
}

pub(crate) async fn resolve_for_product_codes_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    warehouse_id: Uuid,
    product_codes: &[String],
    process: &str,
    node: &str,
) -> Result<ResolvedDualPersonPolicy, DualPersonPolicyError> {
    if !valid_process_node(process, node) {
        return Err(DualPersonPolicyError::InvalidProcessNode);
    }
    let row = sqlx::query_as::<_, PolicyRow>(
        r#"
        WITH product_categories AS (
            SELECT DISTINCT special_drug_category
              FROM products
             WHERE owner_id = $1
               AND product_code = ANY($3)
               AND status = 'active'
        ), ranked AS (
            SELECT category.special_drug_category,
                   rule.id,
                   rule.policy,
                   ROW_NUMBER() OVER (
                       PARTITION BY category.special_drug_category
                       ORDER BY CASE
                                  WHEN rule.owner_id = $1 AND rule.warehouse_id IS NULL THEN 3
                                  WHEN rule.warehouse_id = $2 THEN 2
                                  ELSE 1
                                END DESC,
                                rule.priority DESC,
                                rule.updated_at DESC,
                                rule.id
                   ) AS scope_rank
              FROM product_categories category
              LEFT JOIN dual_person_policy_rules rule
                ON rule.special_drug_category = category.special_drug_category
               AND rule.process_code = $4
               AND rule.node_code = $5
               AND rule.enabled
               AND (rule.owner_id IS NULL OR rule.owner_id = $1)
               AND (rule.warehouse_id IS NULL OR rule.warehouse_id = $2)
        )
        SELECT id, policy
          FROM ranked
         WHERE scope_rank = 1 AND id IS NOT NULL
         ORDER BY CASE policy
                    WHEN 'dual_scan_with_approval' THEN 3
                    WHEN 'dual_scan' THEN 2
                    ELSE 1
                  END DESC,
                  id
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(product_codes)
    .bind(process)
    .bind(node)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    match row {
        Some(row) => Ok(ResolvedDualPersonPolicy {
            policy: DualPersonPolicy::try_from(row.policy.as_str()).map_err(|_| {
                DualPersonPolicyError::Database("数据库包含非法双人策略".to_string())
            })?,
            source_rule_id: Some(row.id),
        }),
        None => Ok(ResolvedDualPersonPolicy {
            policy: DualPersonPolicy::Single,
            source_rule_id: None,
        }),
    }
}

pub(crate) async fn approved_dual_person_record_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    business_ref: &str,
) -> Result<Option<Uuid>, DualPersonPolicyError> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
          FROM h4_approval_records
         WHERE owner_id = $1
           AND scenario = $2
           AND business_ref = $3
           AND status = 'approved'
         ORDER BY approved_at DESC NULLS LAST, updated_at DESC, id
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(DUAL_PERSON_APPROVAL_SCENARIO)
    .bind(business_ref)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

pub(crate) async fn is_active_operator_with_role_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    user_id: Uuid,
    role_code: &str,
) -> Result<bool, DualPersonPolicyError> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM auth_users user_row
              JOIN auth_user_owner_bindings binding
                ON binding.user_id = user_row.id
               AND binding.owner_id = $2
               AND binding.is_active
              JOIN auth_user_roles user_role
                ON user_role.user_id = user_row.id
               AND user_role.owner_id = $2
              JOIN auth_roles role
                ON role.id = user_role.role_id
               AND role.owner_id = $2
             WHERE user_row.id = $1
               AND user_row.status = 'active'
               AND role.role_code = $3
        )
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .bind(role_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)
}
