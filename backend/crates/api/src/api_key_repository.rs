// @governance: skip-page-size - API Key 生命周期与认证/限流事务共用仓储边界，幂等迁移不拆行为链。
use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{ApiKey, ApiKeyRotationResponse, CreateApiKeyRequest, RotateApiKeyRequest};

use crate::{
    audit::{append_event, append_event_in_tx, AuditDiff, AuditWriteRequest},
    idempotency,
    operation_context::OperationContext as AuthContext,
};

#[derive(Clone)]
pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        owner_id: Uuid,
        query: &ApiKeyListQuery,
    ) -> Result<(Vec<ApiKey>, i64), ApiKeyRepositoryError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*) FROM auth_api_keys
             WHERE owner_id = $1
               AND ($2::text IS NULL OR caller_name ILIKE '%' || $2 || '%' OR purpose ILIKE '%' || $2 || '%')
               AND ($3::text IS NULL OR status = $3)
            "#,
        )
        .bind(owner_id)
        .bind(query.keyword.as_deref())
        .bind(query.status.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT id, owner_id, caller_name, purpose, warehouse_ids, scopes,
                   responsible_user_id, key_hash, status, expires_at, grace_expires_at,
                   revoked_at, temporarily_disabled_until,
                   failed_auth_count, failed_auth_window_started_at,
                   rate_limit_window_started_at, rate_limit_count, last_used_at,
                   created_at, updated_at
              FROM auth_api_keys
             WHERE owner_id = $1
               AND ($2::text IS NULL OR caller_name ILIKE '%' || $2 || '%' OR purpose ILIKE '%' || $2 || '%')
               AND ($3::text IS NULL OR status = $3)
             ORDER BY created_at DESC, id DESC
             LIMIT $4 OFFSET $5
            "#,
        )
        .bind(owner_id)
        .bind(query.keyword.as_deref())
        .bind(query.status.as_deref())
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let data = rows
            .into_iter()
            .map(|row| row.into_api_key(None))
            .collect::<Vec<_>>();
        Ok((data, total))
    }

    pub async fn create(
        &self,
        ctx: &AuthContext,
        request: &CreateApiKeyRequest,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        scopes: &[String],
        idempotency_key: &str,
        request_hash: &str,
        key_id: Uuid,
        secret: &str,
    ) -> Result<ApiKey, ApiKeyRepositoryError> {
        let path = "/api/v1/auth/api-keys";
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(response) = replay_idempotency::<ApiKey>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            request_hash,
            "POST",
            path,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(db_error)?;
            return Ok(response);
        }
        validate_owner_references(&mut tx, ctx.owner_id, request).await?;
        let key_hash = hash_secret(secret);
        let row = sqlx::query_as::<_, ApiKeyRow>(
            r#"INSERT INTO auth_api_keys (
                   id, owner_id, caller_name, purpose, warehouse_ids, scopes,
                   responsible_user_id, key_hash, expires_at, created_at, updated_at
               ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10)
               RETURNING id, owner_id, caller_name, purpose, warehouse_ids, scopes,
                         responsible_user_id, key_hash, status, expires_at, grace_expires_at,
                         revoked_at, temporarily_disabled_until,
                         failed_auth_count, failed_auth_window_started_at,
                         rate_limit_window_started_at, rate_limit_count, last_used_at,
                         created_at, updated_at"#,
        )
        .bind(key_id)
        .bind(ctx.owner_id)
        .bind(request.caller_name.trim())
        .bind(request.purpose.trim())
        .bind(&request.warehouse_ids)
        .bind(scopes)
        .bind(request.responsible_user_id)
        .bind(key_hash)
        .bind(expires_at)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        let stored = row.clone().into_api_key(None);
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            request_hash,
            &stored,
            now,
            "POST",
            path,
            "api_key",
            key_id,
        )
        .await?;
        append_context_audit(
            &mut tx,
            ctx,
            "auth.api_key.create",
            key_id,
            None,
            serde_json::to_value(&stored).map_err(serialize_error)?,
            now,
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(row.into_api_key(Some(secret.to_string())))
    }

    pub async fn rotate(
        &self,
        ctx: &AuthContext,
        key_id: Uuid,
        request: &RotateApiKeyRequest,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        idempotency_key: &str,
        request_hash: &str,
        new_key_id: Uuid,
        secret: &str,
    ) -> Result<ApiKeyRotationResponse, ApiKeyRepositoryError> {
        let path = format!("/api/v1/auth/api-keys/{key_id}/rotate");
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(response) = replay_idempotency::<ApiKeyRotationResponse>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            request_hash,
            "POST",
            &path,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(db_error)?;
            return Ok(response);
        }
        let old = load_for_update(&mut tx, ctx.owner_id, key_id)
            .await?
            .ok_or(ApiKeyRepositoryError::NotFound)?;
        if old.status == "revoked" {
            return Err(ApiKeyRepositoryError::Revoked);
        }
        let grace_expires_at = now + Duration::days(request.grace_period_days.unwrap_or(7));
        let new_hash = hash_secret(secret);
        let new_row = sqlx::query_as::<_, ApiKeyRow>(
            r#"INSERT INTO auth_api_keys (
                   id, owner_id, caller_name, purpose, warehouse_ids, scopes,
                   responsible_user_id, key_hash, expires_at, created_at, updated_at
               ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10)
               RETURNING id, owner_id, caller_name, purpose, warehouse_ids, scopes,
                         responsible_user_id, key_hash, status, expires_at, grace_expires_at,
                         revoked_at, temporarily_disabled_until,
                         failed_auth_count, failed_auth_window_started_at,
                         rate_limit_window_started_at, rate_limit_count, last_used_at,
                         created_at, updated_at"#,
        )
        .bind(new_key_id)
        .bind(ctx.owner_id)
        .bind(&old.caller_name)
        .bind(&old.purpose)
        .bind(&old.warehouse_ids)
        .bind(&old.scopes)
        .bind(old.responsible_user_id)
        .bind(new_hash)
        .bind(expires_at)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        let updated_old = sqlx::query_as::<_, ApiKeyRow>(
            r#"UPDATE auth_api_keys
                  SET grace_expires_at = $3, replaced_by_key_id = $4, updated_at = $5, version = version + 1
                WHERE id = $1 AND owner_id = $2
                RETURNING id, owner_id, caller_name, purpose, warehouse_ids, scopes,
                          responsible_user_id, key_hash, status, expires_at, grace_expires_at,
                          revoked_at, temporarily_disabled_until,
                          failed_auth_count, failed_auth_window_started_at,
                          rate_limit_window_started_at, rate_limit_count, last_used_at,
                          created_at, updated_at"#,
        )
        .bind(key_id).bind(ctx.owner_id).bind(grace_expires_at).bind(new_key_id).bind(now)
        .fetch_one(&mut *tx).await.map_err(db_error)?;
        let new_key = new_row.clone().into_api_key(None);
        let response = ApiKeyRotationResponse {
            previous_key_id: key_id,
            previous_grace_expires_at: grace_expires_at,
            new_key: new_key.clone(),
        };
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            request_hash,
            &response,
            now,
            "POST",
            &path,
            "api_key",
            new_key_id,
        )
        .await?;
        let before =
            serde_json::to_value(updated_old.into_api_key(None)).map_err(serialize_error)?;
        let after = serde_json::to_value(&response).map_err(serialize_error)?;
        append_context_audit(
            &mut tx,
            ctx,
            "auth.api_key.rotate",
            key_id,
            Some(AuditDiff::compute(before, after)),
            serde_json::json!({}),
            now,
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(ApiKeyRotationResponse {
            new_key: ApiKey {
                secret: Some(secret.to_string()),
                ..new_key
            },
            ..response
        })
    }

    pub async fn revoke(
        &self,
        ctx: &AuthContext,
        key_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        request_hash: &str,
    ) -> Result<ApiKey, ApiKeyRepositoryError> {
        let path = format!("/api/v1/auth/api-keys/{key_id}/revoke");
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(response) = replay_idempotency::<ApiKey>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            request_hash,
            "POST",
            &path,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(db_error)?;
            return Ok(response);
        }
        let current = load_for_update(&mut tx, ctx.owner_id, key_id)
            .await?
            .ok_or(ApiKeyRepositoryError::NotFound)?;
        let was_revoked = current.status == "revoked";
        let row = if was_revoked {
            current
        } else {
            sqlx::query_as::<_, ApiKeyRow>(
                r#"UPDATE auth_api_keys SET status = 'revoked', revoked_at = $3, updated_at = $3, version = version + 1
                    WHERE id = $1 AND owner_id = $2
                    RETURNING id, owner_id, caller_name, purpose, warehouse_ids, scopes,
                              responsible_user_id, key_hash, status, expires_at, grace_expires_at,
                              revoked_at, temporarily_disabled_until,
                              failed_auth_count, failed_auth_window_started_at,
                              rate_limit_window_started_at, rate_limit_count, last_used_at,
                              created_at, updated_at"#,
            ).bind(key_id).bind(ctx.owner_id).bind(now).fetch_one(&mut *tx).await.map_err(db_error)?
        };
        let response = row.into_api_key(None);
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            request_hash,
            &response,
            now,
            "POST",
            &path,
            "api_key",
            key_id,
        )
        .await?;
        if !was_revoked {
            append_context_audit(
                &mut tx,
                ctx,
                "auth.api_key.revoke",
                key_id,
                None,
                serde_json::json!({}),
                now,
            )
            .await?;
        }
        tx.commit().await.map_err(db_error)?;
        Ok(response)
    }

    pub async fn authenticate(
        &self,
        raw_key: &str,
        target_owner_id: Uuid,
        required_scope: &str,
        now: DateTime<Utc>,
        ip: Option<&str>,
        user_agent: Option<&str>,
        policy: ApiKeyAuthPolicy,
    ) -> Result<ApiKeyContext, ApiKeyRepositoryError> {
        self.authenticate_internal(
            raw_key,
            Some(target_owner_id),
            required_scope,
            now,
            ip,
            user_agent,
            policy,
        )
        .await
    }

    pub async fn authenticate_any_owner(
        &self,
        raw_key: &str,
        required_scope: &str,
        now: DateTime<Utc>,
        ip: Option<&str>,
        user_agent: Option<&str>,
        policy: ApiKeyAuthPolicy,
    ) -> Result<ApiKeyContext, ApiKeyRepositoryError> {
        self.authenticate_internal(raw_key, None, required_scope, now, ip, user_agent, policy)
            .await
    }

    async fn authenticate_internal(
        &self,
        raw_key: &str,
        target_owner_id: Option<Uuid>,
        required_scope: &str,
        now: DateTime<Utc>,
        ip: Option<&str>,
        user_agent: Option<&str>,
        policy: ApiKeyAuthPolicy,
    ) -> Result<ApiKeyContext, ApiKeyRepositoryError> {
        let key_id = parse_key_id(raw_key).ok_or(ApiKeyRepositoryError::Invalid)?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let row = if let Some(owner_id) = target_owner_id {
            let Some(row) = load_for_update(&mut tx, owner_id, key_id).await? else {
                let Some(row) = load_for_update_any_owner(&mut tx, key_id).await? else {
                    return Err(ApiKeyRepositoryError::Invalid);
                };
                tx.commit().await.map_err(db_error)?;
                return Err(if row.owner_id == owner_id {
                    ApiKeyRepositoryError::Invalid
                } else {
                    ApiKeyRepositoryError::CrossOwner
                });
            };
            row
        } else {
            load_for_update_any_owner(&mut tx, key_id)
                .await?
                .ok_or(ApiKeyRepositoryError::Invalid)?
        };
        if row.status == "revoked" {
            tx.commit().await.map_err(db_error)?;
            return Err(ApiKeyRepositoryError::Invalid);
        }
        if row
            .temporarily_disabled_until
            .is_some_and(|until| until > now)
        {
            tx.commit().await.map_err(db_error)?;
            return Err(ApiKeyRepositoryError::TemporarilyDisabled);
        }
        if row.expires_at <= now || row.grace_expires_at.is_some_and(|until| until <= now) {
            tx.commit().await.map_err(db_error)?;
            return Err(ApiKeyRepositoryError::Expired);
        }
        if hash_secret(raw_key) != key_hash_for_row(&row) {
            let failure_count = if row
                .failed_auth_window_started_at
                .is_some_and(|started| started + Duration::minutes(15) > now)
            {
                row.failed_auth_count + 1
            } else {
                1
            };
            let disabled = failure_count >= policy.failure_threshold;
            let disabled_until = disabled.then(|| now + Duration::minutes(policy.disable_minutes));
            sqlx::query("UPDATE auth_api_keys SET failed_auth_count=$2, failed_auth_window_started_at=$3, status=CASE WHEN $4 THEN 'temporarily_disabled' ELSE status END, temporarily_disabled_until=$5, updated_at=$6, version=version+1 WHERE id=$1 AND owner_id=$7")
                .bind(key_id).bind(failure_count).bind(now).bind(disabled).bind(disabled_until).bind(now).bind(row.owner_id)
                .execute(&mut *tx).await.map_err(db_error)?;
            append_key_audit(
                &mut tx,
                &row,
                if disabled {
                    "auth.api_key.temporarily_disabled"
                } else {
                    "auth.api_key.authentication_failed"
                },
                ip,
                user_agent,
                now,
            )
            .await?;
            tx.commit().await.map_err(db_error)?;
            return Err(if disabled {
                ApiKeyRepositoryError::TemporarilyDisabled
            } else {
                ApiKeyRepositoryError::Invalid
            });
        }
        if !row.scopes.iter().any(|scope| scope == required_scope) {
            tx.commit().await.map_err(db_error)?;
            return Err(ApiKeyRepositoryError::InvalidScope);
        }
        let (window_started, count) = match row.rate_limit_window_started_at {
            Some(started) if started + Duration::seconds(1) > now => {
                if row.rate_limit_count >= policy.qps {
                    let disabled_until = now + Duration::minutes(policy.disable_minutes);
                    sqlx::query("UPDATE auth_api_keys SET status='temporarily_disabled', temporarily_disabled_until=$2, updated_at=$3, version=version+1 WHERE id=$1 AND owner_id=$4")
                        .bind(key_id).bind(disabled_until).bind(now).bind(row.owner_id).execute(&mut *tx).await.map_err(db_error)?;
                    append_key_audit(
                        &mut tx,
                        &row,
                        "auth.api_key.rate_limited",
                        ip,
                        user_agent,
                        now,
                    )
                    .await?;
                    tx.commit().await.map_err(db_error)?;
                    return Err(ApiKeyRepositoryError::RateLimited);
                }
                (started, row.rate_limit_count + 1)
            }
            _ => (now, 1),
        };
        sqlx::query("UPDATE auth_api_keys SET status='active', temporarily_disabled_until=NULL, failed_auth_count=0, failed_auth_window_started_at=NULL, rate_limit_window_started_at=$2, rate_limit_count=$3, last_used_at=$4, updated_at=$4, version=version+1 WHERE id=$1 AND owner_id=$5")
            .bind(key_id).bind(window_started).bind(count).bind(now).bind(row.owner_id).execute(&mut *tx).await.map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(ApiKeyContext {
            key_id,
            owner_id: row.owner_id,
            caller_name: row.caller_name,
            warehouse_ids: row.warehouse_ids,
            scopes: row.scopes,
        })
    }

    pub async fn append_request_audit(
        &self,
        context: &ApiKeyContext,
        method: &str,
        path: &str,
        status_code: u16,
        ip: Option<&str>,
        user_agent: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<(), ApiKeyRepositoryError> {
        let mut request = AuditWriteRequest {
            occurred_at: now,
            actor_id: context.key_id,
            actor_name: format!("api-key:{}", context.caller_name),
            owner_id: context.owner_id,
            jti: format!("api-key:{}", context.key_id),
            action: "auth.api_key.request".to_string(),
            module: "H1".to_string(),
            resource_type: "api_key_request".to_string(),
            resource_id: context.key_id.to_string(),
            diff: Some(AuditDiff::compute(
                serde_json::json!({}),
                serde_json::json!({
                    "method": method,
                    "path": path,
                    "status_code": status_code,
                    "scopes": &context.scopes,
                }),
            )),
            request_id: None,
            ip: ip.map(str::to_string),
            user_agent: user_agent.map(str::to_string),
        };
        request.occurred_at = now;
        append_event(&self.pool, &request)
            .await
            .map(|_| ())
            .map_err(|error| ApiKeyRepositoryError::Audit(format!("{error:?}")))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ApiKeyListQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    /// 页码，从 1 起；缺省 1。
    pub page: Option<u32>,
    /// 每页条数；缺省 20，上限 200。
    pub page_size: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct ApiKeyAuthPolicy {
    pub qps: i32,
    pub failure_threshold: i32,
    pub disable_minutes: i64,
}

impl Default for ApiKeyAuthPolicy {
    fn default() -> Self {
        Self {
            qps: 100,
            failure_threshold: 10,
            disable_minutes: 15,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiKeyContext {
    pub key_id: Uuid,
    pub owner_id: Uuid,
    pub caller_name: String,
    pub warehouse_ids: Vec<Uuid>,
    pub scopes: Vec<String>,
}

impl ApiKeyContext {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|candidate| candidate == scope)
    }
    pub fn require_owner(&self, owner_id: Uuid) -> Result<(), ApiKeyRepositoryError> {
        (self.owner_id == owner_id)
            .then_some(())
            .ok_or(ApiKeyRepositoryError::CrossOwner)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiKeyRepositoryError {
    Database(String),
    Audit(String),
    Serialize(String),
    NotFound,
    IdempotencyConflict,
    Invalid,
    InvalidScope,
    Expired,
    Revoked,
    TemporarilyDisabled,
    RateLimited,
    CrossOwner,
    WarehouseScope,
    ResponsibleUser,
}

impl From<crate::idempotency::IdempotencyError> for ApiKeyRepositoryError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(error) => {
                Self::Database(error.to_string())
            }
            crate::idempotency::IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}

#[derive(Clone, Debug, FromRow)]
struct ApiKeyRow {
    id: Uuid,
    owner_id: Uuid,
    caller_name: String,
    purpose: String,
    warehouse_ids: Vec<Uuid>,
    scopes: Vec<String>,
    responsible_user_id: Uuid,
    key_hash: String,
    status: String,
    expires_at: DateTime<Utc>,
    grace_expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    temporarily_disabled_until: Option<DateTime<Utc>>,
    failed_auth_count: i32,
    failed_auth_window_started_at: Option<DateTime<Utc>>,
    rate_limit_window_started_at: Option<DateTime<Utc>>,
    rate_limit_count: i32,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ApiKeyRow {
    fn into_api_key(self, secret: Option<String>) -> ApiKey {
        ApiKey {
            key_id: self.id,
            owner_id: self.owner_id,
            caller_name: self.caller_name,
            purpose: self.purpose,
            warehouse_ids: self.warehouse_ids,
            scopes: self.scopes,
            responsible_user_id: self.responsible_user_id,
            expires_at: self.expires_at,
            status: self.status,
            grace_expires_at: self.grace_expires_at,
            revoked_at: self.revoked_at,
            temporarily_disabled_until: self.temporarily_disabled_until,
            last_used_at: self.last_used_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            secret,
        }
    }
}

async fn load_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key_id: Uuid,
) -> Result<Option<ApiKeyRow>, ApiKeyRepositoryError> {
    load_key(tx, key_id, Some(owner_id)).await
}

async fn load_for_update_any_owner(
    tx: &mut Transaction<'_, Postgres>,
    key_id: Uuid,
) -> Result<Option<ApiKeyRow>, ApiKeyRepositoryError> {
    load_key(tx, key_id, None).await
}

async fn load_key(
    tx: &mut Transaction<'_, Postgres>,
    key_id: Uuid,
    owner_id: Option<Uuid>,
) -> Result<Option<ApiKeyRow>, ApiKeyRepositoryError> {
    sqlx::query_as::<_, ApiKeyRow>(
        "SELECT id, owner_id, caller_name, purpose, warehouse_ids, scopes, responsible_user_id, key_hash, status, expires_at, grace_expires_at, revoked_at, temporarily_disabled_until, failed_auth_count, failed_auth_window_started_at, rate_limit_window_started_at, rate_limit_count, last_used_at, created_at, updated_at FROM auth_api_keys WHERE id=$1 AND ($2::uuid IS NULL OR owner_id=$2) FOR UPDATE",
    )
    .bind(key_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)
}

async fn validate_owner_references(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &CreateApiKeyRequest,
) -> Result<(), ApiKeyRepositoryError> {
    let responsible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_user_owner_bindings WHERE user_id=$1 AND owner_id=$2 AND is_active)")
        .bind(request.responsible_user_id).bind(owner_id).fetch_one(&mut **tx).await.map_err(db_error)?;
    if !responsible {
        return Err(ApiKeyRepositoryError::ResponsibleUser);
    }
    if !request.warehouse_ids.is_empty() {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM warehouses WHERE owner_id=$1 AND id=ANY($2)")
                .bind(owner_id)
                .bind(&request.warehouse_ids)
                .fetch_one(&mut **tx)
                .await
                .map_err(db_error)?;
        if count
            != i64::try_from(request.warehouse_ids.len())
                .map_err(|_| ApiKeyRepositoryError::WarehouseScope)?
        {
            return Err(ApiKeyRepositoryError::WarehouseScope);
        }
    }
    Ok(())
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
) -> Result<(), ApiKeyRepositoryError> {
    idempotency::lock_key(tx, "api-key", owner_id, key)
        .await
        .map_err(Into::into)
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, ApiKeyRepositoryError> {
    idempotency::replay(tx, owner_id, key, request_hash, method, path, now)
        .await
        .map_err(Into::into)
}

async fn store_idempotency<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    response: &T,
    now: DateTime<Utc>,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: Uuid,
) -> Result<(), ApiKeyRepositoryError> {
    idempotency::store_success(
        tx,
        owner_id,
        key,
        request_hash,
        method,
        path,
        resource_type,
        &resource_id.to_string(),
        response,
        now,
    )
    .await
    .map_err(Into::into)
}

async fn append_context_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_id: Uuid,
    diff: Option<AuditDiff>,
    after: serde_json::Value,
    now: DateTime<Utc>,
) -> Result<(), ApiKeyRepositoryError> {
    let diff = diff.or_else(|| Some(AuditDiff::compute(serde_json::json!({}), after)));
    let mut request = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H1",
        "api_key",
        resource_id.to_string(),
        diff,
    );
    request.occurred_at = now;
    append_event_in_tx(tx, &request)
        .await
        .map(|_| ())
        .map_err(|error| ApiKeyRepositoryError::Audit(format!("{error:?}")))
}

async fn append_key_audit(
    tx: &mut Transaction<'_, Postgres>,
    row: &ApiKeyRow,
    action: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), ApiKeyRepositoryError> {
    let mut request = AuditWriteRequest {
        occurred_at: now,
        actor_id: row.id,
        actor_name: format!("api-key:{}", row.caller_name),
        owner_id: row.owner_id,
        jti: format!("api-key:{}", row.id),
        action: action.to_string(),
        module: "H1".to_string(),
        resource_type: "api_key".to_string(),
        resource_id: row.id.to_string(),
        diff: None,
        request_id: None,
        ip: ip.map(str::to_string),
        user_agent: user_agent.map(str::to_string),
    };
    request.occurred_at = now;
    append_event_in_tx(tx, &request)
        .await
        .map(|_| ())
        .map_err(|error| ApiKeyRepositoryError::Audit(format!("{error:?}")))
}

fn key_hash_for_row(row: &ApiKeyRow) -> String {
    // The hash is loaded only for comparison by the authentication query below.
    row.key_hash.clone()
}

fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

fn parse_key_id(raw_key: &str) -> Option<Uuid> {
    let mut parts = raw_key.strip_prefix("wms_")?.splitn(3, '_');
    Uuid::parse_str(parts.next()?)
        .ok()
        .filter(|_| parts.next().is_some())
}

fn serialize_error(error: serde_json::Error) -> ApiKeyRepositoryError {
    ApiKeyRepositoryError::Serialize(error.to_string())
}
fn db_error(error: sqlx::Error) -> ApiKeyRepositoryError {
    ApiKeyRepositoryError::Database(error.to_string())
}
