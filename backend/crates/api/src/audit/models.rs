use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::AuthContext;

pub const AUDIT_SEAL_BATCH_SIZE: i64 = 10_000;
pub const DEFAULT_AUDIT_EVENT_QUERY_LIMIT: u32 = 100;
pub const MAX_AUDIT_EVENT_QUERY_LIMIT: u32 = 100;

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
    pub actor_id: uuid::Uuid,
    pub actor_name: String,
    pub owner_id: uuid::Uuid,
    pub jti: String,
    pub action: String,
    pub module: String,
    pub resource_type: String,
    pub resource_id: String,
    pub diff: Option<AuditDiff>,
    pub request_id: Option<uuid::Uuid>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEventRecord {
    pub id: i64,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: uuid::Uuid,
    pub actor_name: String,
    pub owner_id: uuid::Uuid,
    pub jti: String,
    pub action: String,
    pub module: String,
    pub resource_type: String,
    pub resource_id: String,
    pub diff: Option<AuditDiff>,
    pub request_id: Option<uuid::Uuid>,
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
pub struct AuditEventQueryCursor {
    pub occurred_at: DateTime<Utc>,
    pub id: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEventQuery {
    pub owner_id: uuid::Uuid,
    pub resource_type: Option<String>,
    pub actor_id: Option<uuid::Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub cursor: Option<AuditEventQueryCursor>,
    pub limit: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEventPage {
    pub events: Vec<AuditEventRecord>,
    pub next_cursor: Option<AuditEventQueryCursor>,
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

impl AuditEventRecord {
    pub(crate) fn as_write_request(&self) -> AuditWriteRequest {
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
    pub(crate) fn tamper_self_hash_for_test(&mut self, id: i64, value: &str) {
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
            event.self_hash = value.to_string();
        }
    }

    #[cfg(test)]
    pub(crate) fn tamper_diff_for_test(&mut self, id: i64, diff: AuditDiff) {
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
            event.diff = Some(diff);
        }
    }
}
