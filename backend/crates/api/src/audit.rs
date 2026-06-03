//! Wave 1 H2 append-only audit runtime contract.
//!
//! The PostgreSQL enforcement lives in `backend/migrations/*_audit_event.sql`.
//! This module provides the shared write helper every mutation handler should
//! call before returning success.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::auth::AuthContext;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuditDiff {
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub changed_keys: Vec<String>,
}

impl AuditDiff {
    pub fn compute(before: serde_json::Value, after: serde_json::Value) -> Self {
        let mut changed_keys = Vec::new();
        if let (Some(before), Some(after)) = (before.as_object(), after.as_object()) {
            for (key, after_value) in after {
                match before.get(key) {
                    Some(before_value) if before_value == after_value => {}
                    _ => changed_keys.push(key.clone()),
                }
            }
            for key in before.keys() {
                if !after.contains_key(key) {
                    changed_keys.push(key.clone());
                }
            }
        }
        changed_keys.sort();
        changed_keys.dedup();

        Self {
            before,
            after,
            changed_keys,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditWriteRequest {
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub owner_id: Uuid,
    pub jti: String,
    pub action: String,
    pub module: String,
    pub resource_type: String,
    pub resource_id: String,
    pub diff: Option<AuditDiff>,
    pub request_id: Option<Uuid>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEventRecord {
    pub id: i64,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub owner_id: Uuid,
    pub jti: String,
    pub action: String,
    pub module: String,
    pub resource_type: String,
    pub resource_id: String,
    pub diff: Option<AuditDiff>,
    pub request_id: Option<Uuid>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub prev_hash: Option<String>,
    pub self_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditChainSeal {
    pub seal_date: chrono::NaiveDate,
    pub last_id: i64,
    pub last_self_hash: String,
    pub sealed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditError {
    HashChainBroken {
        at_id: i64,
        expected: String,
        actual: String,
    },
    EmptyChain,
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, Default)]
pub struct AuditLog {
    events: Vec<AuditEventRecord>,
}

impl AuditWriteRequest {
    pub fn from_auth_context(
        ctx: &AuthContext,
        action: impl Into<String>,
        module: impl Into<String>,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        diff: Option<AuditDiff>,
    ) -> Self {
        Self {
            occurred_at: Utc::now(),
            actor_id: ctx.user_id,
            actor_name: ctx.actor_name.clone(),
            owner_id: ctx.owner_id,
            jti: ctx.jti.clone(),
            action: action.into(),
            module: module.into(),
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            diff,
            request_id: None,
            ip: None,
            user_agent: None,
        }
    }

    pub fn compute_self_hash(&self, prev_hash: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        let canonical = serde_json::json!({
            "prev_hash": prev_hash,
            "occurred_at_micros": self.occurred_at.timestamp_micros(),
            "actor_id": self.actor_id,
            "actor_name": self.actor_name,
            "owner_id": self.owner_id,
            "jti": self.jti,
            "action": self.action,
            "module": self.module,
            "resource_type": self.resource_type,
            "resource_id": self.resource_id,
            "diff": self.diff,
            "request_id": self.request_id,
            "ip": self.ip,
            "user_agent": self.user_agent,
        });
        hasher.update(canonical.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }
}

impl AuditLog {
    pub fn append_event(&mut self, req: AuditWriteRequest) -> AuditEventRecord {
        let id = self.events.len() as i64 + 1;
        let prev_hash = self.events.last().map(|event| event.self_hash.clone());
        let self_hash = req.compute_self_hash(prev_hash.as_deref());
        let record = AuditEventRecord {
            id,
            occurred_at: req.occurred_at,
            actor_id: req.actor_id,
            actor_name: req.actor_name,
            owner_id: req.owner_id,
            jti: req.jti,
            action: req.action,
            module: req.module,
            resource_type: req.resource_type,
            resource_id: req.resource_id,
            diff: req.diff,
            request_id: req.request_id,
            ip: req.ip,
            user_agent: req.user_agent,
            prev_hash,
            self_hash,
        };
        self.events.push(record.clone());
        record
    }

    pub fn verify_hash_chain(&self) -> Result<(), AuditError> {
        let mut prev_hash: Option<&str> = None;
        for event in &self.events {
            if event.prev_hash.as_deref() != prev_hash {
                return Err(AuditError::HashChainBroken {
                    at_id: event.id,
                    expected: prev_hash.unwrap_or("").to_string(),
                    actual: event.prev_hash.clone().unwrap_or_default(),
                });
            }
            let req = event.as_write_request();
            let expected = req.compute_self_hash(event.prev_hash.as_deref());
            if expected != event.self_hash {
                return Err(AuditError::HashChainBroken {
                    at_id: event.id,
                    expected,
                    actual: event.self_hash.clone(),
                });
            }
            prev_hash = Some(&event.self_hash);
        }
        Ok(())
    }

    pub fn seal_latest_chain(
        &self,
        sealed_at: DateTime<Utc>,
    ) -> Result<AuditChainSeal, AuditError> {
        let Some(last) = self.events.last() else {
            return Err(AuditError::EmptyChain);
        };
        Ok(AuditChainSeal {
            seal_date: sealed_at.date_naive(),
            last_id: last.id,
            last_self_hash: last.self_hash.clone(),
            sealed_at,
        })
    }

    pub fn events(&self) -> &[AuditEventRecord] {
        &self.events
    }

    #[cfg(test)]
    fn tamper_self_hash_for_test(&mut self, id: i64, value: &str) {
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
            event.self_hash = value.to_string();
        }
    }

    #[cfg(test)]
    fn tamper_diff_for_test(&mut self, id: i64, diff: AuditDiff) {
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
            event.diff = Some(diff);
        }
    }
}

impl AuditEventRecord {
    fn as_write_request(&self) -> AuditWriteRequest {
        AuditWriteRequest {
            occurred_at: self.occurred_at,
            actor_id: self.actor_id,
            actor_name: self.actor_name.clone(),
            owner_id: self.owner_id,
            jti: self.jti.clone(),
            action: self.action.clone(),
            module: self.module.clone(),
            resource_type: self.resource_type.clone(),
            resource_id: self.resource_id.clone(),
            diff: self.diff.clone(),
            request_id: self.request_id,
            ip: self.ip.clone(),
            user_agent: self.user_agent.clone(),
        }
    }
}

#[derive(Debug, FromRow)]
struct AuditChainHeadRow {
    self_hash: String,
}

#[derive(Debug, FromRow)]
struct AuditEventDbRow {
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
    let chain_lock_key = format!("audit_event:{}", req.occurred_at.date_naive());

    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(chain_lock_key)
        .execute(&mut *tx)
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
    .fetch_optional(&mut *tx)
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
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| AuditError::Database(error.to_string()))?;

    tx.commit()
        .await
        .map_err(|error| AuditError::Database(error.to_string()))?;

    inserted.into_record()
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
    let next_date = seal_date
        .succ_opt()
        .ok_or_else(|| AuditError::Database("seal_date overflow".to_string()))?;

    let events = sqlx::query_as::<_, AuditEventDbRow>(
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
         ORDER BY id ASC
         FOR UPDATE
        "#,
    )
    .bind(seal_date)
    .bind(next_date)
    .fetch_all(&mut *tx)
    .await
    .map_err(|error| AuditError::Database(error.to_string()))?;

    if events.is_empty() {
        return Err(AuditError::EmptyChain);
    }

    let mut prev_hash: Option<String> = None;
    let mut records = Vec::with_capacity(events.len());
    for row in events {
        let record = row.into_record()?;
        if record.prev_hash.as_deref() != prev_hash.as_deref() {
            return Err(AuditError::HashChainBroken {
                at_id: record.id,
                expected: prev_hash.unwrap_or_default(),
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
        prev_hash = Some(record.self_hash.clone());
        records.push(record);
    }

    let last = records.last().ok_or(AuditError::EmptyChain)?;
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
    .bind(last.id)
    .bind(&last.self_hash)
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
    F: FnOnce() -> (T, String, Option<AuditDiff>),
{
    let (result, resource_id, diff) = mutation();
    let req =
        AuditWriteRequest::from_auth_context(ctx, action, module, resource_type, resource_id, diff);
    audit_log.append_event(req);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{commit_with_audit, AuditDiff, AuditLog};
    use crate::auth::AuthContext;
    use serde_json::json;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct DemoItem {
        id: String,
        owner_id: Uuid,
        name: String,
    }

    fn ctx() -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "alice".to_string(),
            permissions: vec!["demo:write".to_string()],
            jti: "jti-1".to_string(),
        }
    }

    fn create_demo_item_handler(
        items: &mut BTreeMap<String, DemoItem>,
        audit_log: &mut AuditLog,
        ctx: &AuthContext,
        id: &str,
    ) -> DemoItem {
        commit_with_audit(audit_log, ctx, "create", "DEMO", "demo_item", || {
            let item = DemoItem {
                id: id.to_string(),
                owner_id: ctx.owner_id,
                name: "item-a".to_string(),
            };
            items.insert(id.to_string(), item.clone());
            (item, id.to_string(), None)
        })
        .expect("audit commit should succeed")
    }

    fn update_demo_item_handler(
        items: &mut BTreeMap<String, DemoItem>,
        audit_log: &mut AuditLog,
        ctx: &AuthContext,
        id: &str,
        name: &str,
    ) -> DemoItem {
        commit_with_audit(audit_log, ctx, "update", "DEMO", "demo_item", || {
            let before = items.get(id).expect("item exists").clone();
            let after = DemoItem {
                name: name.to_string(),
                ..before.clone()
            };
            items.insert(id.to_string(), after.clone());
            let diff = AuditDiff::compute(
                json!({"name": before.name, "owner_id": before.owner_id}),
                json!({"name": after.name, "owner_id": after.owner_id}),
            );
            (after, id.to_string(), Some(diff))
        })
        .expect("audit commit should succeed")
    }

    #[test]
    fn two_mutation_handlers_reuse_commit_with_audit() {
        let ctx = ctx();
        let mut items = BTreeMap::new();
        let mut audit_log = AuditLog::default();

        let created = create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
        let updated =
            update_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1", "item-b");

        assert_eq!(created.owner_id, ctx.owner_id);
        assert_eq!(updated.name, "item-b");
        assert_eq!(audit_log.events().len(), 2);
        assert_eq!(audit_log.events()[0].action, "create");
        assert_eq!(audit_log.events()[1].action, "update");
        assert_eq!(audit_log.events()[0].actor_id, ctx.user_id);
        assert_eq!(audit_log.events()[0].owner_id, ctx.owner_id);
        assert_eq!(audit_log.events()[0].jti, ctx.jti);
        assert_eq!(
            audit_log.events()[1]
                .diff
                .as_ref()
                .expect("diff should exist")
                .changed_keys,
            vec!["name".to_string()]
        );
        audit_log
            .verify_hash_chain()
            .expect("hash chain should verify");
    }

    #[test]
    fn hash_chain_detects_tampering_and_can_be_sealed() {
        let ctx = ctx();
        let mut items = BTreeMap::new();
        let mut audit_log = AuditLog::default();

        create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
        let seal = audit_log
            .seal_latest_chain(chrono::Utc::now())
            .expect("non-empty chain should seal");

        assert_eq!(seal.last_id, 1);
        assert_eq!(seal.last_self_hash, audit_log.events()[0].self_hash);

        audit_log.tamper_self_hash_for_test(1, "tampered");
        assert!(audit_log.verify_hash_chain().is_err());
    }

    #[test]
    fn hash_chain_detects_diff_value_tampering() {
        let ctx = ctx();
        let mut items = BTreeMap::new();
        let mut audit_log = AuditLog::default();

        create_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1");
        update_demo_item_handler(&mut items, &mut audit_log, &ctx, "ITEM-1", "item-b");

        audit_log.tamper_diff_for_test(
            2,
            AuditDiff::compute(
                json!({"name": "item-a", "owner_id": ctx.owner_id}),
                json!({"name": "item-c", "owner_id": ctx.owner_id}),
            ),
        );

        assert!(audit_log.verify_hash_chain().is_err());
    }
}
