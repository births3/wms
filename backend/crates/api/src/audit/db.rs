use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::operation_context::OperationContext as AuthContext;

use super::{
    AuditChainSeal, AuditError, AuditEventPage, AuditEventQuery, AuditEventQueryCursor,
    AuditEventRecord, AuditLog, AuditWriteRequest, AUDIT_SEAL_BATCH_SIZE,
    MAX_AUDIT_EVENT_QUERY_LIMIT,
};

#[derive(Debug, FromRow)]
struct AuditChainHeadRow {
    self_hash: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct AuditEventDbRow {
    id: i64,
    occurred_at: DateTime<Utc>,
    actor_id: Uuid,
    actor_name: String,
    owner_id: Uuid,
    jti: String,
    action: String,
    module: String,
    resource_type: Option<String>,
    resource_id: Option<String>,
    diff: Option<serde_json::Value>,
    request_id: Option<Uuid>,
    ip: Option<String>,
    user_agent: Option<String>,
    prev_hash: Option<String>,
    self_hash: String,
}

#[derive(Debug, FromRow)]
struct AuditChainSealRow {
    seal_date: chrono::NaiveDate,
    last_id: i64,
    last_self_hash: String,
    sealed_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub(crate) struct AuditSealProgress {
    prev_hash: Option<String>,
    last_id: Option<i64>,
    last_self_hash: Option<String>,
    pub(crate) records_seen: i64,
}

impl AuditSealProgress {
    pub(crate) fn new() -> Self {
        Self {
            prev_hash: None,
            last_id: None,
            last_self_hash: None,
            records_seen: 0,
        }
    }

    pub(crate) fn observe(&mut self, record: AuditEventRecord) -> Result<(), AuditError> {
        if record.prev_hash.as_deref() != self.prev_hash.as_deref() {
            return Err(AuditError::HashChainBroken {
                at_id: record.id,
                expected: self.prev_hash.clone().unwrap_or_default(),
                actual: record.prev_hash.clone().unwrap_or_default(),
            });
        }
        let expected = record
            .as_write_request()
            .compute_self_hash(record.prev_hash.as_deref());
        if expected != record.self_hash {
            return Err(AuditError::HashChainBroken {
                at_id: record.id,
                expected,
                actual: record.self_hash,
            });
        }
        self.prev_hash = Some(record.self_hash.clone());
        self.last_id = Some(record.id);
        self.last_self_hash = Some(record.self_hash);
        self.records_seen += 1;
        Ok(())
    }

    pub(crate) fn last(&self) -> Result<(i64, &str), AuditError> {
        match (self.last_id, self.last_self_hash.as_deref()) {
            (Some(id), Some(hash)) => Ok((id, hash)),
            _ => Err(AuditError::EmptyChain),
        }
    }
}

impl AuditEventDbRow {
    fn into_record(self) -> Result<AuditEventRecord, AuditError> {
        let diff = self
            .diff
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| AuditError::Serialize(error.to_string()))?;
        Ok(AuditEventRecord {
            id: self.id,
            occurred_at: self.occurred_at,
            actor_id: self.actor_id,
            actor_name: self.actor_name,
            owner_id: self.owner_id,
            jti: self.jti,
            action: self.action,
            module: self.module,
            resource_type: self.resource_type.unwrap_or_default(),
            resource_id: self.resource_id.unwrap_or_default(),
            diff,
            request_id: self.request_id,
            ip: self.ip,
            user_agent: self.user_agent,
            prev_hash: self.prev_hash,
            self_hash: self.self_hash,
        })
    }
}

impl From<AuditChainSealRow> for AuditChainSeal {
    fn from(row: AuditChainSealRow) -> Self {
        Self {
            seal_date: row.seal_date,
            last_id: row.last_id,
            last_self_hash: row.last_self_hash,
            sealed_at: row.sealed_at,
        }
    }
}

pub async fn append_event(
    pool: &PgPool,
    req: &AuditWriteRequest,
) -> Result<AuditEventRecord, AuditError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;

    let inserted = append_event_in_tx(&mut tx, req).await?;

    tx.commit()
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;

    Ok(inserted)
}

pub async fn list_events(
    pool: &PgPool,
    query: &AuditEventQuery,
) -> Result<AuditEventPage, AuditError> {
    let limit = query.limit.clamp(1, MAX_AUDIT_EVENT_QUERY_LIMIT);
    let fetch_limit = i64::from(limit) + 1;
    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            id,
            occurred_at,
            actor_id,
            actor_name,
            owner_id,
            jti,
            action,
            module,
            resource_type,
            resource_id,
            diff,
            request_id,
            host(ip) AS ip,
            user_agent,
            prev_hash,
            self_hash
          FROM audit_event
         WHERE owner_id = "#,
    );
    builder.push_bind(query.owner_id);

    if let Some(resource_type) = query
        .resource_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" AND resource_type = ");
        builder.push_bind(resource_type);
    }
    if let Some(action) = query
        .action
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" AND action = ");
        builder.push_bind(action);
    }
    if let Some(resource_id) = query
        .resource_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" AND resource_id = ");
        builder.push_bind(resource_id);
    }
    if let Some(product_code) = query
        .product_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" AND (COALESCE(diff->'before'->>'product_code', '') ILIKE '%' || ");
        builder.push_bind(product_code);
        builder.push(" || '%' OR COALESCE(diff->'after'->>'product_code', '') ILIKE '%' || ");
        builder.push_bind(product_code);
        builder.push(" || '%')");
    }
    if let Some(batch_no) = query
        .batch_no
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder.push(" AND (COALESCE(diff->'before'->>'batch_no', '') ILIKE '%' || ");
        builder.push_bind(batch_no);
        builder.push(" || '%' OR COALESCE(diff->'after'->>'batch_no', '') ILIKE '%' || ");
        builder.push_bind(batch_no);
        builder.push(" || '%')");
    }
    if let Some(actor_id) = query.actor_id {
        builder.push(" AND actor_id = ");
        builder.push_bind(actor_id);
    }
    if let Some(from) = query.from {
        builder.push(" AND occurred_at >= ");
        builder.push_bind(from);
    }
    if let Some(to) = query.to {
        builder.push(" AND occurred_at <= ");
        builder.push_bind(to);
    }
    if let Some(cursor) = &query.cursor {
        builder.push(" AND (occurred_at < ");
        builder.push_bind(cursor.occurred_at);
        builder.push(" OR (occurred_at = ");
        builder.push_bind(cursor.occurred_at);
        builder.push(" AND id < ");
        builder.push_bind(cursor.id);
        builder.push(") )");
    }
    builder.push(" ORDER BY occurred_at DESC, id DESC LIMIT ");
    builder.push_bind(fetch_limit);

    let mut events: Vec<AuditEventRecord> = builder
        .build_query_as::<AuditEventDbRow>()
        .fetch_all(pool)
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?
        .into_iter()
        .map(AuditEventDbRow::into_record)
        .collect::<Result<Vec<_>, _>>()?;

    let next_cursor = if events.len() > limit as usize {
        events.pop();
        events.last().map(|event| AuditEventQueryCursor {
            occurred_at: event.occurred_at,
            id: event.id,
        })
    } else {
        None
    };

    Ok(AuditEventPage {
        events,
        next_cursor,
    })
}

pub async fn export_events(
    pool: &PgPool,
    query: &AuditEventQuery,
) -> Result<Vec<AuditEventRecord>, AuditError> {
    // ponytail: bounded in-memory export; move to a DB cursor stream if exports exceed 100k rows.
    let mut page_query = query.clone();
    page_query.limit = MAX_AUDIT_EVENT_QUERY_LIMIT;
    page_query.cursor = None;
    let mut events = Vec::new();

    loop {
        let page = list_events(pool, &page_query).await?;
        if events.len() + page.events.len() > super::models::MAX_AUDIT_EXPORT_EVENTS {
            return Err(AuditError::ExportTooLarge);
        }
        let next_cursor = page.next_cursor;
        events.extend(page.events);
        match next_cursor {
            Some(cursor) => {
                if events.len() >= super::models::MAX_AUDIT_EXPORT_EVENTS {
                    return Err(AuditError::ExportTooLarge);
                }
                page_query.cursor = Some(cursor);
            }
            None => return Ok(events),
        }
    }
}

pub async fn append_event_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    req: &AuditWriteRequest,
) -> Result<AuditEventRecord, AuditError> {
    let chain_lock_key = format!("audit_event:{}", req.occurred_at.date_naive());

    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(chain_lock_key)
        .execute(&mut **tx)
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;

    let head = sqlx::query_as::<_, AuditChainHeadRow>(
        r#"
        SELECT self_hash
          FROM audit_event
         WHERE occurred_at >= date_trunc('day', $1::timestamptz)
           AND occurred_at < date_trunc('day', $1::timestamptz) + interval '1 day'
         ORDER BY id DESC
         LIMIT 1
         FOR UPDATE
        "#,
    )
    .bind(req.occurred_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| AuditError::Database(error.to_string()))?;

    let prev_hash = head.map(|row| row.self_hash);
    let self_hash = req.compute_self_hash(prev_hash.as_deref());
    let diff = req
        .diff
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| AuditError::Serialize(error.to_string()))?;

    let inserted = sqlx::query_as::<_, AuditEventDbRow>(
        r#"
        INSERT INTO audit_event (
            occurred_at,
            actor_id,
            actor_name,
            owner_id,
            jti,
            action,
            module,
            resource_type,
            resource_id,
            diff,
            request_id,
            ip,
            user_agent,
            prev_hash,
            self_hash
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12::inet, $13, $14, $15
        )
        RETURNING
            id,
            occurred_at,
            actor_id,
            actor_name,
            owner_id,
            jti,
            action,
            module,
            resource_type,
            resource_id,
            diff,
            request_id,
            host(ip) AS ip,
            user_agent,
            prev_hash,
            self_hash
        "#,
    )
    .bind(req.occurred_at)
    .bind(req.actor_id)
    .bind(&req.actor_name)
    .bind(req.owner_id)
    .bind(&req.jti)
    .bind(&req.action)
    .bind(&req.module)
    .bind(&req.resource_type)
    .bind(&req.resource_id)
    .bind(diff)
    .bind(req.request_id)
    .bind(&req.ip)
    .bind(&req.user_agent)
    .bind(&prev_hash)
    .bind(&self_hash)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| AuditError::Database(error.to_string()))?;

    let record = inserted.into_record()?;
    append_audit_event_bus_outbox_in_tx(tx, &record).await?;
    Ok(record)
}

async fn append_audit_event_bus_outbox_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    record: &AuditEventRecord,
) -> Result<(), AuditError> {
    let event_id = Uuid::new_v4();
    let event_type = format!("audit.{}.{}", record.resource_type, record.action);
    let idempotency_key = format!("audit_event:{}", record.id);
    sqlx::query(
        r#"
        INSERT INTO event_bus_event (
            id, owner_id, idempotency_key, event_type, source_module,
            resource_type, resource_id, payload, created_at
        )
        VALUES ($1, $2, $3, $4, 'H2', $5, $6, $7, $8)
        ON CONFLICT (owner_id, idempotency_key) DO NOTHING
        "#,
    )
    .bind(event_id)
    .bind(record.owner_id)
    .bind(&idempotency_key)
    .bind(&event_type)
    .bind(&record.resource_type)
    .bind(&record.resource_id)
    .bind(serde_json::json!({
        "audit_event_id": record.id,
        "action": record.action,
        "module": record.module,
        "resource_type": record.resource_type,
        "resource_id": record.resource_id,
        "occurred_at": record.occurred_at,
    }))
    .bind(record.occurred_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| AuditError::Database(error.to_string()))?;

    let subscriptions: Vec<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT id, event_pattern
          FROM event_bus_subscription
         WHERE owner_id = $1 AND active = TRUE
        "#,
    )
    .bind(record.owner_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| AuditError::Database(error.to_string()))?;

    for (subscription_id, event_pattern) in subscriptions {
        if !matches_event_pattern(&event_pattern, &event_type) {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO event_bus_delivery (
                id, owner_id, event_id, subscription_id, status, attempt_count, next_attempt_at
            )
            VALUES ($1, $2, $3, $4, 'pending', 0, $5)
            ON CONFLICT (event_id, subscription_id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(record.owner_id)
        .bind(event_id)
        .bind(subscription_id)
        .bind(record.occurred_at)
        .execute(&mut **tx)
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;
    }

    Ok(())
}

fn matches_event_pattern(pattern: &str, event_type: &str) -> bool {
    pattern == "*"
        || pattern == event_type
        || pattern
            .strip_suffix(".*")
            .is_some_and(|prefix| event_type.starts_with(&format!("{prefix}.")))
}

pub async fn seal_audit_chain(
    pool: &PgPool,
    seal_date: chrono::NaiveDate,
    sealed_at: DateTime<Utc>,
) -> Result<AuditChainSeal, AuditError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;
    let chain_lock_key = format!("audit_event:{}", seal_date);

    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(chain_lock_key)
        .execute(&mut *tx)
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;

    let next_date = seal_date
        .succ_opt()
        .ok_or_else(|| AuditError::Database("seal_date overflow".to_string()))?;

    let mut progress = AuditSealProgress::new();
    loop {
        let last_seen_id = progress.last_id.unwrap_or(0);
        let rows = sqlx::query_as::<_, AuditEventDbRow>(
            r#"
            SELECT
                id,
                occurred_at,
                actor_id,
                actor_name,
                owner_id,
                jti,
                action,
                module,
                resource_type,
                resource_id,
                diff,
                request_id,
                host(ip) AS ip,
                user_agent,
                prev_hash,
                self_hash
              FROM audit_event
             WHERE occurred_at >= $1::date
               AND occurred_at < $2::date
               AND id > $3
             ORDER BY id ASC
             LIMIT $4
             FOR UPDATE
            "#,
        )
        .bind(seal_date)
        .bind(next_date)
        .bind(last_seen_id)
        .bind(AUDIT_SEAL_BATCH_SIZE)
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;

        if rows.is_empty() {
            break;
        }

        for row in rows {
            progress.observe(row.into_record()?)?;
        }
    }

    let (last_id, last_self_hash) = progress.last()?;
    let seal = sqlx::query_as::<_, AuditChainSealRow>(
        r#"
        INSERT INTO audit_chain_seal (
            seal_date,
            last_id,
            last_self_hash,
            sealed_at
        )
        VALUES ($1, $2, $3, $4)
        RETURNING seal_date, last_id, last_self_hash, sealed_at
        "#,
    )
    .bind(seal_date)
    .bind(last_id)
    .bind(last_self_hash)
    .bind(sealed_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| AuditError::Database(error.to_string()))?;

    tx.commit()
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;

    Ok(seal.into())
}

pub fn commit_with_audit<T, F>(
    audit_log: &mut AuditLog,
    ctx: &AuthContext,
    action: &'static str,
    module: &'static str,
    resource_type: &'static str,
    mutation: F,
) -> Result<T, AuditError>
where
    F: FnOnce() -> (T, String, Option<super::AuditDiff>),
{
    let (result, resource_id, diff) = mutation();
    let req =
        AuditWriteRequest::from_auth_context(ctx, action, module, resource_type, resource_id, diff);
    audit_log.append_event(req);
    Ok(result)
}
