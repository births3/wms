//! SPIKE-002 H2 append-only 审计 — repo + diff 计算 + hash chain
//!
//! 验证：
//! - H1: trigger + 角色权限阻止 UPDATE/DELETE
//! - H2: 按月 RANGE 分区，partition pruning 生效（EXPLAIN ANALYZE 验证）
//! - H3: JSONB diff (before/after/changed_keys) + jsonb_path_ops 索引
//! - H4: 写入吞吐（spike 用 100 条 timing 而非 60M 行 wrk）
//! - H5: hash chain 完整性自检

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, types::Json, PgPool};
use uuid::Uuid;

// ============================================================
// Domain types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: i64,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub owner_id: Uuid,
    pub action: String,
    pub module: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub diff: Option<serde_json::Value>,
    pub prev_hash: Option<String>,
    pub self_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDiff {
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub changed_keys: Vec<String>,
}

impl AuditDiff {
    /// 计算两个 JSON object 的 diff
    pub fn compute(
        before: &serde_json::Value,
        after: &serde_json::Value,
    ) -> Self {
        let mut changed = Vec::new();
        if let (Some(b_obj), Some(a_obj)) = (before.as_object(), after.as_object()) {
            // 遍历 after 的所有 key，检查 before 是否相同
            for (k, a_v) in a_obj {
                match b_obj.get(k) {
                    Some(b_v) if b_v == a_v => {} // 未变
                    _ => changed.push(k.clone()),
                }
            }
            // 遍历 before 中 after 不存在的 key
            for k in b_obj.keys() {
                if !a_obj.contains_key(k) {
                    changed.push(k.clone());
                }
            }
        }
        changed.sort();
        changed.dedup();
        Self {
            before: before.clone(),
            after: after.clone(),
            changed_keys: changed,
        }
    }
}

// ============================================================
// Hash chain 计算
//
// self_hash = sha256(prev_hash || actor_id || occurred_at_unix || action || diff_canonical)
// prev_hash 由插入时 RETURNING 获取上一条
// ============================================================

#[derive(Debug, Clone)]
pub struct AuditWriteRequest {
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub owner_id: Uuid,
    pub action: String,
    pub module: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub diff: Option<AuditDiff>,
}

impl AuditWriteRequest {
    /// 计算 self_hash（确定性算法，可独立重算用于完整性校验）
    pub fn compute_self_hash(&self, prev_hash: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.unwrap_or(""));
        hasher.update("|");
        hasher.update(self.actor_id.as_bytes());
        hasher.update("|");
        hasher.update(self.occurred_at.timestamp_micros().to_be_bytes());
        hasher.update("|");
        hasher.update(self.action.as_bytes());
        hasher.update("|");
        hasher.update(self.module.as_bytes());
        hasher.update("|");
        if let Some(d) = &self.diff {
            // 用 changed_keys 串接（确定性，不受 JSON 字段顺序影响）
            for k in &d.changed_keys {
                hasher.update(k.as_bytes());
                hasher.update(",");
            }
        }
        hex::encode(hasher.finalize())
    }
}

// ============================================================
// Repository
// ============================================================

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("数据库错误：{0}")]
    Db(#[from] sqlx::Error),
    #[error("hash chain 不完整：第 {at_id} 条 self_hash 期望 {expected} 实际 {actual}")]
    HashChainBroken {
        at_id: i64,
        expected: String,
        actual: String,
    },
}

pub type AuditResult<T> = Result<T, AuditError>;

pub async fn create_pool(database_url: &str) -> AuditResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// 写入一条审计事件
///
/// 流程：
/// 1. 查最新一条的 self_hash 作为本条 prev_hash
/// 2. 计算本条 self_hash
/// 3. INSERT 一行（必须含 partition key occurred_at）
///
/// 性能注：步骤 1 是 SELECT，与 INSERT 不在同一事务则可能有并发漂移
/// （A B 两条同时插入会拿到相同 prev_hash）。
/// 生产化方案：用同一事务 + SELECT FOR UPDATE 或专用 sequence；spike-002 不深入
/// （H5 单测验证逻辑正确即可，吞吐数据用单线程跑）。
pub async fn append_event(
    pool: &PgPool,
    req: &AuditWriteRequest,
) -> AuditResult<i64> {
    let prev_hash: Option<String> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_event ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let self_hash = req.compute_self_hash(prev_hash.as_deref());

    let diff_json: Option<Json<&AuditDiff>> = req.diff.as_ref().map(Json);

    let id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO audit_event (
            occurred_at, actor_id, actor_name, owner_id,
            action, module, resource_type, resource_id,
            diff, prev_hash, self_hash
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id
        "#,
    )
    .bind(req.occurred_at)
    .bind(req.actor_id)
    .bind(&req.actor_name)
    .bind(req.owner_id)
    .bind(&req.action)
    .bind(&req.module)
    .bind(req.resource_type.as_deref())
    .bind(req.resource_id.as_deref())
    .bind(diff_json.as_ref().map(|j| serde_json::to_value(j.0).unwrap()))
    .bind(prev_hash.as_deref())
    .bind(&self_hash)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// H5：扫全表验证 hash chain 完整性
///
/// 返回断裂位置（如有）。生产化部署应每日跑一次 cron。
pub async fn verify_hash_chain(pool: &PgPool) -> AuditResult<()> {
    let rows = sqlx::query(
        r#"
        SELECT id, occurred_at, actor_id, actor_name, owner_id,
               action, module, resource_type, resource_id,
               diff, prev_hash, self_hash
        FROM audit_event
        ORDER BY id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut last_hash: Option<String> = None;
    for row in rows {
        use sqlx::Row;
        let id: i64 = row.get("id");
        let occurred_at: DateTime<Utc> = row.get("occurred_at");
        let actor_id: Uuid = row.get("actor_id");
        let actor_name: String = row.get("actor_name");
        let owner_id: Uuid = row.get("owner_id");
        let action: String = row.get("action");
        let module: String = row.get("module");
        let resource_type: Option<String> = row.get("resource_type");
        let resource_id: Option<String> = row.get("resource_id");
        let diff_value: Option<serde_json::Value> = row.get("diff");
        let prev_hash: Option<String> = row.get("prev_hash");
        let self_hash: String = row.get("self_hash");

        // 重算 expected hash
        let diff = diff_value.and_then(|v| serde_json::from_value::<AuditDiff>(v).ok());
        let req = AuditWriteRequest {
            occurred_at,
            actor_id,
            actor_name,
            owner_id,
            action,
            module,
            resource_type,
            resource_id,
            diff,
        };
        let expected = req.compute_self_hash(prev_hash.as_deref());

        if expected != self_hash {
            return Err(AuditError::HashChainBroken {
                at_id: id,
                expected,
                actual: self_hash,
            });
        }
        // 链：本行 prev_hash 必须等于上一行 self_hash
        if let Some(last) = &last_hash {
            if prev_hash.as_deref() != Some(last.as_str()) {
                return Err(AuditError::HashChainBroken {
                    at_id: id,
                    expected: last.clone(),
                    actual: prev_hash.unwrap_or_default(),
                });
            }
        }
        last_hash = Some(self_hash);
    }
    Ok(())
}
