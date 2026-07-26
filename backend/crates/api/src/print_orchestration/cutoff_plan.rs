use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreateCutoffPlanRequest, CutoffDateException, CutoffPlan, CutoffPlanListResponse,
    CutoffPlanScope, WeeklyCutoffSlot,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::{
    repository::{
        json_request_hash, lock_idempotency_key, map_db_error, replay_idempotency,
        store_idempotency_success, PgPrintOrchestrationRepository,
    },
    IdempotentMutation, PrintOrchestrationError,
};

#[derive(Debug, FromRow)]
struct CutoffPlanRow {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    warehouse_id: Uuid,
    scope_type: String,
    customer_id: Option<Uuid>,
    route_code: Option<String>,
    utc_offset_minutes: i16,
    weekly_schedule: Value,
    exceptions: Value,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgPrintOrchestrationRepository {
    pub(super) async fn list_cutoff_plans(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<Uuid>,
    ) -> Result<CutoffPlanListResponse, PrintOrchestrationError> {
        let rows = sqlx::query_as::<_, CutoffPlanRow>(
            r#"
            SELECT id, owner_id, name, warehouse_id, scope_type,
                   customer_id, route_code, utc_offset_minutes,
                   weekly_schedule, exceptions, effective_from,
                   effective_to, status, created_at, updated_at
              FROM h9_cutoff_plans
             WHERE owner_id = $1
               AND ($2::uuid IS NULL OR warehouse_id = $2)
             ORDER BY updated_at DESC, id
            "#,
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let data = rows
            .into_iter()
            .map(map_cutoff_plan)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CutoffPlanListResponse { data })
    }

    pub(super) async fn create_cutoff_plan(
        &self,
        ctx: &AuthContext,
        request: CreateCutoffPlanRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<CutoffPlan>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&request)?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(plan) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: plan,
                replayed: true,
            });
        }
        ensure_cutoff_scope_exists(&mut tx, ctx.owner_id, &request).await?;
        let weekly_schedule = serde_json::to_value(&request.weekly_schedule)
            .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
        let exceptions = serde_json::to_value(&request.exceptions)
            .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
        let row = sqlx::query_as::<_, CutoffPlanRow>(
            r#"
            INSERT INTO h9_cutoff_plans (
                id, owner_id, name, warehouse_id, scope_type, customer_id,
                route_code, utc_offset_minutes, weekly_schedule, exceptions,
                effective_from, effective_to, status, created_by,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, 'draft', $13, $14, $14
            )
            RETURNING id, owner_id, name, warehouse_id, scope_type,
                      customer_id, route_code, utc_offset_minutes,
                      weekly_schedule, exceptions, effective_from,
                      effective_to, status, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(request.name.trim())
        .bind(request.warehouse_id)
        .bind(scope_code(request.scope))
        .bind(request.customer_id)
        .bind(request.route_code.as_deref().map(str::trim))
        .bind(request.utc_offset_minutes)
        .bind(weekly_schedule)
        .bind(exceptions)
        .bind(request.effective_from)
        .bind(request.effective_to)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let plan = map_cutoff_plan(row)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "/api/v1/print-orchestration/cutoff-plans",
            "cutoff_plan",
            &plan,
            now,
        )
        .await?;
        append_cutoff_plan_audit(&mut tx, ctx, "create_cutoff_plan", &plan, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: plan,
            replayed: false,
        })
    }

    pub(super) async fn publish_cutoff_plan(
        &self,
        ctx: &AuthContext,
        plan_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<CutoffPlan>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&json!({ "plan_id": plan_id }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(plan) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: plan,
                replayed: true,
            });
        }
        let draft = load_cutoff_plan_for_update(&mut tx, ctx.owner_id, plan_id).await?;
        if draft.status != "draft" {
            return Err(PrintOrchestrationError::InvalidState);
        }
        lock_cutoff_scope(&mut tx, &draft).await?;
        if cutoff_period_overlaps(&mut tx, &draft).await? {
            return Err(PrintOrchestrationError::EffectivePeriodOverlap);
        }
        let row = sqlx::query_as::<_, CutoffPlanRow>(
            r#"
            UPDATE h9_cutoff_plans
               SET status = 'published',
                   published_by = $3,
                   published_at = $4,
                   updated_at = $4
             WHERE owner_id = $1 AND id = $2 AND status = 'draft'
            RETURNING id, owner_id, name, warehouse_id, scope_type,
                      customer_id, route_code, utc_offset_minutes,
                      weekly_schedule, exceptions, effective_from,
                      effective_to, status, created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(plan_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintOrchestrationError::InvalidState)?;
        let plan = map_cutoff_plan(row)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-orchestration/cutoff-plans/{plan_id}/publish"),
            "cutoff_plan",
            &plan,
            now,
        )
        .await?;
        append_cutoff_plan_audit(&mut tx, ctx, "publish_cutoff_plan", &plan, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: plan,
            replayed: false,
        })
    }

    pub(super) async fn resolve_cutoff_plan(
        &self,
        ctx: &AuthContext,
        warehouse_id: Uuid,
        customer_id: Uuid,
        route_code: &str,
        effective_at: DateTime<Utc>,
    ) -> Result<CutoffPlan, PrintOrchestrationError> {
        let row = sqlx::query_as::<_, CutoffPlanRow>(
            r#"
            SELECT id, owner_id, name, warehouse_id, scope_type,
                   customer_id, route_code, utc_offset_minutes,
                   weekly_schedule, exceptions, effective_from,
                   effective_to, status, created_at, updated_at
              FROM h9_cutoff_plans
             WHERE owner_id = $1
               AND warehouse_id = $2
               AND status = 'published'
               AND effective_from <= $5
               AND (effective_to IS NULL OR effective_to > $5)
               AND (
                    (scope_type = 'customer' AND customer_id = $3)
                    OR (scope_type = 'route' AND route_code = $4)
                    OR scope_type = 'owner_warehouse'
               )
             ORDER BY CASE scope_type
                          WHEN 'customer' THEN 3
                          WHEN 'route' THEN 2
                          ELSE 1
                      END DESC
             LIMIT 1
            "#,
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .bind(customer_id)
        .bind(route_code.trim())
        .bind(effective_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintOrchestrationError::CutoffPlanNotFound)?;
        map_cutoff_plan(row)
    }
}

async fn ensure_cutoff_scope_exists(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &CreateCutoffPlanRequest,
) -> Result<(), PrintOrchestrationError> {
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM warehouses
             WHERE owner_id = $1 AND id = $2
        )
        AND (
            $3::uuid IS NULL
            OR EXISTS (
                SELECT 1
                  FROM customers
                 WHERE owner_id = $1 AND id = $3
            )
        )
        "#,
    )
    .bind(owner_id)
    .bind(request.warehouse_id)
    .bind(request.customer_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if valid {
        Ok(())
    } else {
        Err(PrintOrchestrationError::InvalidRequest)
    }
}

async fn load_cutoff_plan_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    plan_id: Uuid,
) -> Result<CutoffPlanRow, PrintOrchestrationError> {
    sqlx::query_as::<_, CutoffPlanRow>(
        r#"
        SELECT id, owner_id, name, warehouse_id, scope_type,
               customer_id, route_code, utc_offset_minutes,
               weekly_schedule, exceptions, effective_from,
               effective_to, status, created_at, updated_at
          FROM h9_cutoff_plans
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(plan_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintOrchestrationError::CutoffPlanNotFound)
}

async fn lock_cutoff_scope(
    tx: &mut Transaction<'_, Postgres>,
    plan: &CutoffPlanRow,
) -> Result<(), PrintOrchestrationError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('h9-cutoff-plan'), hashtext($1))")
        .bind(format!(
            "{}:{}:{}:{}",
            plan.owner_id,
            plan.warehouse_id,
            plan.scope_type,
            plan.customer_id
                .map(|id| id.to_string())
                .or_else(|| plan.route_code.clone())
                .unwrap_or_else(|| "default".to_string())
        ))
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn cutoff_period_overlaps(
    tx: &mut Transaction<'_, Postgres>,
    plan: &CutoffPlanRow,
) -> Result<bool, PrintOrchestrationError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM h9_cutoff_plans
             WHERE owner_id = $1
               AND warehouse_id = $2
               AND scope_type = $3
               AND customer_id IS NOT DISTINCT FROM $4
               AND route_code IS NOT DISTINCT FROM $5
               AND status = 'published'
               AND id <> $6
               AND effective_from < COALESCE($8, 'infinity'::timestamptz)
               AND COALESCE(effective_to, 'infinity'::timestamptz) > $7
        )
        "#,
    )
    .bind(plan.owner_id)
    .bind(plan.warehouse_id)
    .bind(&plan.scope_type)
    .bind(plan.customer_id)
    .bind(&plan.route_code)
    .bind(plan.id)
    .bind(plan.effective_from)
    .bind(plan.effective_to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn append_cutoff_plan_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    plan: &CutoffPlan,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H9",
        "cutoff_plan",
        plan.id.to_string(),
        Some(AuditDiff::compute(Value::Null, json!(plan))),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintOrchestrationError::Audit(format!("{error:?}")))?;
    Ok(())
}

fn map_cutoff_plan(row: CutoffPlanRow) -> Result<CutoffPlan, PrintOrchestrationError> {
    let weekly_schedule: Vec<WeeklyCutoffSlot> = serde_json::from_value(row.weekly_schedule)
        .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
    let exceptions: Vec<CutoffDateException> = serde_json::from_value(row.exceptions)
        .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
    Ok(CutoffPlan {
        id: row.id,
        owner_id: row.owner_id,
        name: row.name,
        warehouse_id: row.warehouse_id,
        scope: parse_scope(&row.scope_type)?,
        customer_id: row.customer_id,
        route_code: row.route_code,
        utc_offset_minutes: row.utc_offset_minutes,
        weekly_schedule,
        exceptions,
        effective_from: row.effective_from,
        effective_to: row.effective_to,
        status: row.status,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn scope_code(scope: CutoffPlanScope) -> &'static str {
    match scope {
        CutoffPlanScope::Customer => "customer",
        CutoffPlanScope::Route => "route",
        CutoffPlanScope::OwnerWarehouse => "owner_warehouse",
    }
}

fn parse_scope(value: &str) -> Result<CutoffPlanScope, PrintOrchestrationError> {
    match value {
        "customer" => Ok(CutoffPlanScope::Customer),
        "route" => Ok(CutoffPlanScope::Route),
        "owner_warehouse" => Ok(CutoffPlanScope::OwnerWarehouse),
        _ => Err(PrintOrchestrationError::Serialize(
            "unknown cutoff plan scope".to_string(),
        )),
    }
}
