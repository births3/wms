//! M-RC 查询与频率配置。

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use wms_domain::PageMeta;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    reconciliation::{
        db, lock_idempotency, replay_idempotency, request_hash, store_idempotency,
        IdempotentMutation, PgReconciliationRepository, ReconciliationError, ReconciliationItem,
    },
};

#[derive(Clone, Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ReconciliationItemQuery {
    pub product_code: Option<String>,
    pub batch_no: Option<String>,
    /// 逗号分隔差异类型；管理端多选控件提交 canonical CSV。
    pub difference_type: Option<String>,
    /// 逗号分隔处理状态；管理端多选控件提交 canonical CSV。
    pub resolution_status: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReconciliationItemListResponse {
    pub data: Vec<ReconciliationItem>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconciliationRule {
    pub interval_hours: i32,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertReconciliationRuleRequest {
    pub interval_hours: i32,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReconciliationDueOwner {
    pub owner_id: uuid::Uuid,
    pub interval_hours: i32,
    pub next_due_at: DateTime<Utc>,
    pub window_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ClaimReconciliationRequest {
    pub worker_id: String,
    pub lease_seconds: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconciliationScheduleClaim {
    pub id: Uuid,
    pub claim_token: Uuid,
    pub owner_id: Uuid,
    pub window_key: String,
    pub worker_id: String,
    pub attempt_no: i32,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconciliationClaimResponse {
    pub claim: Option<ReconciliationScheduleClaim>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RenewReconciliationClaimRequest {
    pub claim_token: Uuid,
    pub worker_id: String,
    pub lease_seconds: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationClaimFailureStage {
    Pull,
    Submit,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationClaimFailureCode {
    ErpPullFailed,
    SnapshotSubmitFailed,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FailReconciliationClaimRequest {
    pub claim_token: Uuid,
    pub stage: ReconciliationClaimFailureStage,
    pub error_code: ReconciliationClaimFailureCode,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconciliationClaimMutation {
    pub id: Uuid,
    pub status: String,
    pub lease_expires_at: DateTime<Utc>,
    pub run_id: Option<Uuid>,
}

#[derive(FromRow)]
struct ReconciliationItemRow {
    id: uuid::Uuid,
    product_code: String,
    batch_no: String,
    wms_qty: wms_domain::Quantity,
    erp_qty: wms_domain::Quantity,
    difference_qty: wms_domain::Quantity,
    difference_type: String,
    resolution_status: String,
    stock_adjustment_order_ids: Vec<uuid::Uuid>,
    created_at: DateTime<Utc>,
}

impl From<ReconciliationItemRow> for ReconciliationItem {
    fn from(row: ReconciliationItemRow) -> Self {
        Self {
            id: row.id,
            product_code: row.product_code,
            batch_no: row.batch_no,
            wms_qty: row.wms_qty,
            erp_qty: row.erp_qty,
            difference_qty: row.difference_qty,
            difference_type: row.difference_type,
            resolution_status: row.resolution_status,
            stock_adjustment_order_ids: row.stock_adjustment_order_ids,
            created_at: row.created_at,
        }
    }
}

impl PgReconciliationRepository {
    pub async fn list_items(
        &self,
        ctx: &AuthContext,
        query: ReconciliationItemQuery,
    ) -> Result<ReconciliationItemListResponse, ReconciliationError> {
        let limit = query.limit.unwrap_or(50);
        if !(1..=200).contains(&limit) {
            return Err(ReconciliationError::InvalidRequest);
        }
        let cursor = query.cursor.as_deref().map(parse_cursor).transpose()?;
        let rows = sqlx::query_as::<_, ReconciliationItemRow>(
            "SELECT item.id, item.product_code, item.batch_no, item.wms_qty, item.erp_qty,
                    item.difference_qty, item.difference_type, item.resolution_status,
                    ARRAY(SELECT link.adjustment_order_id
                            FROM reconciliation_item_adjustments link
                           WHERE link.item_id = item.id
                           ORDER BY link.adjustment_order_id) AS stock_adjustment_order_ids,
                    item.created_at
               FROM reconciliation_items item
              WHERE item.owner_id = $1
                AND ($2::TEXT IS NULL OR item.product_code ILIKE '%' || $2 || '%')
                AND ($3::TEXT IS NULL OR item.batch_no ILIKE '%' || $3 || '%')
                AND ($4::TEXT IS NULL OR item.difference_type = ANY(string_to_array($4, ',')))
                AND ($5::TEXT IS NULL OR item.resolution_status = ANY(string_to_array($5, ',')))
                AND ($6::TIMESTAMPTZ IS NULL OR (item.created_at, item.id) < ($6, $7))
              ORDER BY item.created_at DESC, item.id DESC
              LIMIT $8",
        )
        .bind(ctx.owner_id)
        .bind(non_empty(query.product_code))
        .bind(non_empty(query.batch_no))
        .bind(non_empty(query.difference_type))
        .bind(non_empty(query.resolution_status))
        .bind(cursor.map(|value| value.0))
        .bind(cursor.map(|value| value.1))
        .bind(i64::from(limit) + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        let mut data = rows
            .into_iter()
            .map(ReconciliationItem::from)
            .collect::<Vec<ReconciliationItem>>();
        let next_cursor = if data.len() > limit as usize {
            data.pop();
            data.last().map(|item| {
                format!(
                    "{},{}",
                    item.created_at.to_rfc3339_opts(SecondsFormat::Nanos, true),
                    item.id
                )
            })
        } else {
            None
        };
        Ok(ReconciliationItemListResponse {
            page: PageMeta {
                count: data.len() as u32,
                next_cursor,
            },
            data,
        })
    }

    pub async fn get_rule(
        &self,
        ctx: &AuthContext,
        now: DateTime<Utc>,
    ) -> Result<ReconciliationRule, ReconciliationError> {
        sqlx::query_as::<_, (i32, bool, DateTime<Utc>)>(
            "SELECT interval_hours, enabled, updated_at
               FROM reconciliation_rules WHERE owner_id = $1",
        )
        .bind(ctx.owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)
        .map(|row| {
            row.map_or(
                ReconciliationRule {
                    interval_hours: 24,
                    enabled: true,
                    updated_at: now,
                },
                |(interval_hours, enabled, updated_at)| ReconciliationRule {
                    interval_hours,
                    enabled,
                    updated_at,
                },
            )
        })
    }

    pub async fn upsert_rule(
        &self,
        ctx: &AuthContext,
        req: UpsertReconciliationRuleRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<ReconciliationRule>, ReconciliationError> {
        if !(1..=168).contains(&req.interval_hours) {
            return Err(ReconciliationError::InvalidRequest);
        }
        let hash = request_hash(&req)?;
        let path = "/api/v1/reconciliation/rule";
        let mut tx = self.pool.begin().await.map_err(db)?;
        lock_idempotency(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "PUT",
            path,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let value = sqlx::query_as::<_, (i32, bool, DateTime<Utc>)>(
            "INSERT INTO reconciliation_rules
             (owner_id, interval_hours, enabled, updated_by, updated_at)
             VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (owner_id) DO UPDATE
                 SET interval_hours = EXCLUDED.interval_hours,
                     enabled = EXCLUDED.enabled,
                     updated_by = EXCLUDED.updated_by,
                     updated_at = EXCLUDED.updated_at
             RETURNING interval_hours, enabled, updated_at",
        )
        .bind(ctx.owner_id)
        .bind(req.interval_hours)
        .bind(req.enabled)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)
        .map(|(interval_hours, enabled, updated_at)| ReconciliationRule {
            interval_hours,
            enabled,
            updated_at,
        })?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "upsert_reconciliation_rule",
            "M-RC",
            "reconciliation_rule",
            ctx.owner_id.to_string(),
            Some(AuditDiff::compute(
                json!({}),
                serde_json::to_value(&value)
                    .map_err(|e| ReconciliationError::Serialize(e.to_string()))?,
            )),
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|e| ReconciliationError::Audit(format!("{e:?}")))?;
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "PUT",
            path,
            "reconciliation_rule",
            ctx.owner_id.to_string(),
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }
}

fn parse_cursor(value: &str) -> Result<(DateTime<Utc>, Uuid), ReconciliationError> {
    let (created_at, id) = value
        .split_once(',')
        .ok_or(ReconciliationError::InvalidRequest)?;
    let created_at = DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| ReconciliationError::InvalidRequest)?
        .with_timezone(&Utc);
    let id = id
        .parse::<Uuid>()
        .map_err(|_| ReconciliationError::InvalidRequest)?;
    Ok((created_at, id))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|v| !v.is_empty())
}
