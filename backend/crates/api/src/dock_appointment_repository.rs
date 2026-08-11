// @governance: skip-page-size - 预约状态、审计和幂等保持同一事务边界，迁移不拆行为链。
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    ArriveDockAppointmentRequest, CancelDockAppointmentRequest, CreateDockAppointmentRequest,
    DockAppointment, UpdateDockAppointmentRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    idempotency,
    operation_context::OperationContext as AuthContext,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DockAppointmentRepositoryError {
    NotFound,
    AppointmentNotFound,
    WindowInvalid,
    WindowEnded,
    OwnerWarehouseMismatch,
    AppointmentNoConflict,
    ActiveAppointmentConflict,
    TimeConflict,
    StatusNotEditable,
    StatusNotCancellable,
    StatusNotArrivable,
    ArrivalCheckMismatch,
    TemperatureMismatch,
    IdempotencyConflict,
    Invalid(String),
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<crate::idempotency::IdempotencyError> for DockAppointmentRepositoryError {
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

#[derive(Clone, Debug)]
pub struct PgDockAppointmentRepository {
    pool: PgPool,
}

#[derive(Clone, FromRow)]
struct DockAppointmentRow {
    id: Uuid,
    dock_id: Uuid,
    owner_id: Uuid,
    warehouse_id: Uuid,
    status: String,
    appointment_no: String,
    document_type: String,
    document_no: String,
    window_start_at: DateTime<Utc>,
    window_end_at: DateTime<Utc>,
    vehicle_plate_no: Option<String>,
    vehicle_type: String,
    driver_name: String,
    driver_phone: String,
    supersedes_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
    arrived_at: Option<DateTime<Utc>>,
    arrival_deviation_minutes: Option<i64>,
}

impl PgDockAppointmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 查询当前货主在指定仓库范围内的月台预约（offset 分页）。
    pub async fn list(
        &self,
        ctx: &AuthContext,
        warehouse_id: Uuid,
        dock_id: Option<Uuid>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        status: Option<String>,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<DockAppointment>, i64), DockAppointmentRepositoryError> {
        if let (Some(from), Some(to)) = (from, to) {
            if from > to {
                return Err(DockAppointmentRepositoryError::WindowInvalid);
            }
        }

        let scope_owned = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM warehouses w WHERE w.id=$1 AND w.owner_id=$2 AND w.status='active' AND ($3::UUID IS NULL OR EXISTS (SELECT 1 FROM warehouse_docks d WHERE d.id=$3 AND d.warehouse_id=w.id)))",
        )
        .bind(warehouse_id)
        .bind(ctx.owner_id)
        .bind(dock_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        if !scope_owned {
            return Err(DockAppointmentRepositoryError::OwnerWarehouseMismatch);
        }

        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        let total: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM dock_appointments a JOIN warehouses w ON w.id=a.warehouse_id WHERE a.owner_id=$1 AND a.warehouse_id=$2 AND w.owner_id=$1 AND w.status='active' AND ($3::UUID IS NULL OR a.dock_id=$3) AND ($4::TIMESTAMPTZ IS NULL OR a.window_end_at > $4) AND ($5::TIMESTAMPTZ IS NULL OR a.window_start_at < $5) AND ($6::TEXT IS NULL OR a.status=$6)",
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .bind(dock_id)
        .bind(from)
        .bind(to)
        .bind(status.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, DockAppointmentRow>(
            "SELECT a.id, a.dock_id, a.owner_id, a.warehouse_id, a.status, a.appointment_no, a.document_type, a.document_no, a.window_start_at, a.window_end_at, a.vehicle_plate_no, a.vehicle_type, a.driver_name, a.driver_phone, a.supersedes_id, a.created_at, a.updated_at, a.version, a.arrived_at, a.arrival_deviation_minutes FROM dock_appointments a JOIN warehouses w ON w.id=a.warehouse_id WHERE a.owner_id=$1 AND a.warehouse_id=$2 AND w.owner_id=$1 AND w.status='active' AND ($3::UUID IS NULL OR a.dock_id=$3) AND ($4::TIMESTAMPTZ IS NULL OR a.window_end_at > $4) AND ($5::TIMESTAMPTZ IS NULL OR a.window_start_at < $5) AND ($6::TEXT IS NULL OR a.status=$6) ORDER BY a.window_start_at ASC, a.dock_id ASC, a.id ASC LIMIT $7 OFFSET $8",
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .bind(dock_id)
        .bind(from)
        .bind(to)
        .bind(status)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        Ok((rows.into_iter().map(DockAppointment::from).collect(), total))
    }

    pub async fn create_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateDockAppointmentRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        mut audit: AuditWriteRequest,
    ) -> Result<DockAppointment, DockAppointmentRepositoryError> {
        validate_request(&req)?;
        validate_window(req.window_start_at, req.window_end_at, now)?;
        let request_hash = request_hash(&serde_json::json!({ "request": &req }))?;
        let path = "/api/v1/dock-appointments";

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<DockAppointment>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }

        ensure_dock_warehouse_owned(&mut tx, ctx.owner_id, req.dock_id, req.warehouse_id).await?;
        ensure_active_document_available(&mut tx, ctx.owner_id, &req).await?;
        ensure_no_time_overlap(
            &mut tx,
            ctx.owner_id,
            req.dock_id,
            req.window_start_at,
            req.window_end_at,
            None,
        )
        .await?;

        let row = sqlx::query_as::<_, DockAppointmentRow>(
            "INSERT INTO dock_appointments (id, dock_id, owner_id, warehouse_id, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_plate_no, vehicle_type, driver_name, driver_phone, status, created_at, updated_at, version, arrival_deviation_minutes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,'pending',$14,$14,1,DEFAULT) RETURNING id, dock_id, owner_id, warehouse_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_plate_no, vehicle_type, driver_name, driver_phone, supersedes_id, created_at, updated_at, version, arrived_at, arrival_deviation_minutes",
        )
        .bind(Uuid::new_v4())
        .bind(req.dock_id)
        .bind(ctx.owner_id)
        .bind(req.warehouse_id)
        .bind(&req.appointment_no)
        .bind(&req.document_type)
        .bind(&req.document_no)
        .bind(req.window_start_at)
        .bind(req.window_end_at)
        .bind(req.vehicle_plate_no)
        .bind(&req.vehicle_type)
        .bind(&req.driver_name)
        .bind(&req.driver_phone)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_create_error)?;
        let appointment: DockAppointment = row.into();

        let after = serde_json::to_value(&appointment)
            .map_err(|error| DockAppointmentRepositoryError::Serialize(error.to_string()))?;
        audit.actor_id = ctx.user_id;
        audit.actor_name = ctx.actor_name.clone();
        audit.owner_id = ctx.owner_id;
        audit.jti = ctx.jti.clone();
        audit.resource_id = appointment.id.to_string();
        audit.diff = Some(AuditDiff::compute(serde_json::json!({}), after));
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DockAppointmentRepositoryError::Audit(format!("{error:?}")))?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "dock_appointment",
            &appointment.id.to_string(),
            &appointment,
            now,
        )
        .await?;

        tx.commit().await.map_err(map_db_error)?;
        Ok(appointment)
    }

    pub async fn change_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateDockAppointmentRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        mut audit: AuditWriteRequest,
    ) -> Result<DockAppointment, DockAppointmentRepositoryError> {
        validate_update_request(&req)?;
        validate_window(req.window_start_at, req.window_end_at, now)?;
        let request_hash = request_hash(&serde_json::json!({ "id": id, "request": &req }))?;
        let path = format!("/api/v1/dock-appointments/{id}");
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<DockAppointment>(
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
            return Ok(replay);
        }

        let old = fetch_appointment_for_update(&mut tx, ctx.owner_id, id).await?;
        if !matches!(old.status.as_str(), "pending" | "confirmed") {
            return Err(DockAppointmentRepositoryError::StatusNotEditable);
        }
        ensure_dock_warehouse_owned(&mut tx, ctx.owner_id, req.dock_id, old.warehouse_id).await?;
        ensure_no_time_overlap(
            &mut tx,
            ctx.owner_id,
            req.dock_id,
            req.window_start_at,
            req.window_end_at,
            Some(id),
        )
        .await?;

        let old_appointment: DockAppointment = old.clone().into();
        let next_version = old
            .version
            .checked_add(1)
            .ok_or_else(|| DockAppointmentRepositoryError::Invalid("预约版本溢出".to_string()))?;
        let appointment_no = format!("{}-V{next_version}", old.appointment_no);
        sqlx::query(
            "UPDATE dock_appointments SET status='cancelled', updated_at=$3 WHERE owner_id=$1 AND id=$2",
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let row = sqlx::query_as::<_, DockAppointmentRow>(
            "INSERT INTO dock_appointments (id, dock_id, owner_id, warehouse_id, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_plate_no, vehicle_type, driver_name, driver_phone, status, supersedes_id, created_at, updated_at, version, arrival_deviation_minutes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$16,$17,DEFAULT) RETURNING id, dock_id, owner_id, warehouse_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_plate_no, vehicle_type, driver_name, driver_phone, supersedes_id, created_at, updated_at, version, arrived_at, arrival_deviation_minutes",
        )
        .bind(Uuid::new_v4())
        .bind(req.dock_id)
        .bind(ctx.owner_id)
        .bind(old.warehouse_id)
        .bind(appointment_no)
        .bind(&old.document_type)
        .bind(&old.document_no)
        .bind(req.window_start_at)
        .bind(req.window_end_at)
        .bind(req.vehicle_plate_no)
        .bind(&req.vehicle_type)
        .bind(&req.driver_name)
        .bind(&req.driver_phone)
        .bind(&old.status)
        .bind(id)
        .bind(now)
        .bind(next_version)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_create_error)?;
        let appointment: DockAppointment = row.into();

        audit.actor_id = ctx.user_id;
        audit.actor_name = ctx.actor_name.clone();
        audit.owner_id = ctx.owner_id;
        audit.jti = ctx.jti.clone();
        audit.resource_id = appointment.id.to_string();
        audit.diff = Some(AuditDiff::compute(
            serde_json::to_value(&old_appointment)
                .map_err(|error| DockAppointmentRepositoryError::Serialize(error.to_string()))?,
            serde_json::json!({
                "appointment": &appointment,
                "reason": req.reason.as_deref().unwrap_or("")
            }),
        ));
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DockAppointmentRepositoryError::Audit(format!("{error:?}")))?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &path,
            "dock_appointment",
            &appointment.id.to_string(),
            &appointment,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(appointment)
    }

    pub async fn cancel_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: CancelDockAppointmentRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        mut audit: AuditWriteRequest,
    ) -> Result<DockAppointment, DockAppointmentRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "id": id, "request": &req }))?;
        let path = format!("/api/v1/dock-appointments/{id}/cancel");
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<DockAppointment>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }

        let old = fetch_appointment_for_update(&mut tx, ctx.owner_id, id).await?;
        if !matches!(old.status.as_str(), "pending" | "confirmed") {
            return Err(DockAppointmentRepositoryError::StatusNotCancellable);
        }
        let old_appointment: DockAppointment = old.clone().into();
        let row = sqlx::query_as::<_, DockAppointmentRow>(
            "UPDATE dock_appointments SET status='cancelled', updated_at=$3 WHERE owner_id=$1 AND id=$2 RETURNING id, dock_id, owner_id, warehouse_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_plate_no, vehicle_type, driver_name, driver_phone, supersedes_id, created_at, updated_at, version, arrived_at, arrival_deviation_minutes",
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let appointment: DockAppointment = row.into();
        audit.actor_id = ctx.user_id;
        audit.actor_name = ctx.actor_name.clone();
        audit.owner_id = ctx.owner_id;
        audit.jti = ctx.jti.clone();
        audit.resource_id = id.to_string();
        audit.diff = Some(AuditDiff::compute(
            serde_json::to_value(&old_appointment)
                .map_err(|error| DockAppointmentRepositoryError::Serialize(error.to_string()))?,
            serde_json::json!({
                "appointment": &appointment,
                "reason": req.reason.as_deref().unwrap_or("")
            }),
        ));
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DockAppointmentRepositoryError::Audit(format!("{error:?}")))?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &path,
            "dock_appointment",
            &appointment.id.to_string(),
            &appointment,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(appointment)
    }

    pub async fn arrive_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ArriveDockAppointmentRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        mut audit: AuditWriteRequest,
    ) -> Result<DockAppointment, DockAppointmentRepositoryError> {
        validate_arrival_request(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "id": id, "request": &req }))?;
        let path = format!("/api/v1/dock-appointments/{id}/arrive");
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<DockAppointment>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }

        let old = fetch_appointment_for_update(&mut tx, ctx.owner_id, id)
            .await
            .map_err(|error| match error {
                DockAppointmentRepositoryError::NotFound => {
                    DockAppointmentRepositoryError::AppointmentNotFound
                }
                other => other,
            })?;
        if old.appointment_no != req.appointment_no
            || old.vehicle_plate_no.as_deref() != Some(req.vehicle_plate_no.trim())
            || old.driver_name != req.driver_name.trim()
            || old.vehicle_type != req.vehicle_type.trim()
        {
            return Err(DockAppointmentRepositoryError::ArrivalCheckMismatch);
        }
        if old.status == "arrived" {
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &request_hash,
                "POST",
                &path,
                "dock_appointment",
                &id.to_string(),
                &DockAppointment::from(old.clone()),
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok(old.into());
        }
        if !matches!(old.status.as_str(), "pending" | "confirmed") {
            return Err(DockAppointmentRepositoryError::StatusNotArrivable);
        }
        let temperature_zone: String =
            sqlx::query_scalar("SELECT temperature_zone FROM warehouse_docks WHERE id=$1")
                .bind(old.dock_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_db_error)?;
        if !vehicle_type_matches_temperature_zone(req.vehicle_type.trim(), &temperature_zone) {
            return Err(DockAppointmentRepositoryError::TemperatureMismatch);
        }
        let arrival_deviation_minutes = (now - old.window_start_at).num_minutes();

        let row = sqlx::query_as::<_, DockAppointmentRow>(
            "UPDATE dock_appointments SET status='arrived', arrived_at=$3, arrival_deviation_minutes=$4, updated_at=$3 WHERE owner_id=$1 AND id=$2 RETURNING id, dock_id, owner_id, warehouse_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_plate_no, vehicle_type, driver_name, driver_phone, supersedes_id, created_at, updated_at, version, arrived_at, arrival_deviation_minutes"
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(now)
        .bind(arrival_deviation_minutes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let appointment: DockAppointment = row.into();
        audit.actor_id = ctx.user_id;
        audit.actor_name = ctx.actor_name.clone();
        audit.owner_id = ctx.owner_id;
        audit.jti = ctx.jti.clone();
        audit.resource_id = id.to_string();
        audit.diff = Some(AuditDiff::compute(
            serde_json::to_value(DockAppointment::from(old))
                .map_err(|error| DockAppointmentRepositoryError::Serialize(error.to_string()))?,
            serde_json::to_value(&appointment)
                .map_err(|error| DockAppointmentRepositoryError::Serialize(error.to_string()))?,
        ));
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DockAppointmentRepositoryError::Audit(format!("{error:?}")))?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &path,
            "dock_appointment",
            &id.to_string(),
            &appointment,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(appointment)
    }
}

async fn fetch_appointment_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<DockAppointmentRow, DockAppointmentRepositoryError> {
    sqlx::query_as::<_, DockAppointmentRow>(
        "SELECT id, dock_id, owner_id, warehouse_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_plate_no, vehicle_type, driver_name, driver_phone, supersedes_id, created_at, updated_at, version, arrived_at, arrival_deviation_minutes FROM dock_appointments WHERE owner_id=$1 AND id=$2 FOR UPDATE",
    )
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(DockAppointmentRepositoryError::NotFound)
}

async fn ensure_dock_warehouse_owned(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    dock_id: Uuid,
    warehouse_id: Uuid,
) -> Result<(), DockAppointmentRepositoryError> {
    let dock_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT d.id FROM warehouse_docks d JOIN warehouses w ON w.id = d.warehouse_id WHERE d.id = $1 AND d.warehouse_id = $2 AND w.owner_id = $3 AND w.status = 'active' FOR UPDATE OF d",
    )
    .bind(dock_id)
    .bind(warehouse_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    dock_id
        .map(|_| ())
        .ok_or(DockAppointmentRepositoryError::OwnerWarehouseMismatch)
}

async fn ensure_active_document_available(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &CreateDockAppointmentRequest,
) -> Result<(), DockAppointmentRepositoryError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM dock_appointments WHERE owner_id = $1 AND document_type = $2 AND document_no = $3 AND status IN ('pending', 'confirmed', 'arrived'))",
    )
    .bind(owner_id)
    .bind(&request.document_type)
    .bind(&request.document_no)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    (!exists)
        .then_some(())
        .ok_or(DockAppointmentRepositoryError::ActiveAppointmentConflict)
}

async fn ensure_no_time_overlap(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    dock_id: Uuid,
    window_start_at: DateTime<Utc>,
    window_end_at: DateTime<Utc>,
    excluded_id: Option<Uuid>,
) -> Result<(), DockAppointmentRepositoryError> {
    let overlaps: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM dock_appointments WHERE owner_id = $1 AND dock_id = $2 AND status IN ('pending', 'confirmed', 'arrived') AND ($5::UUID IS NULL OR id <> $5) AND window_start_at < $4 AND window_end_at > $3)",
    )
    .bind(owner_id)
    .bind(dock_id)
    .bind(window_start_at)
    .bind(window_end_at)
    .bind(excluded_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    (!overlaps)
        .then_some(())
        .ok_or(DockAppointmentRepositoryError::TimeConflict)
}

fn validate_update_request(
    request: &UpdateDockAppointmentRequest,
) -> Result<(), DockAppointmentRepositoryError> {
    [
        ("vehicle_type", request.vehicle_type.trim()),
        ("driver_name", request.driver_name.trim()),
        ("driver_phone", request.driver_phone.trim()),
    ]
    .into_iter()
    .find(|(_, value)| value.is_empty())
    .map(|(field, _)| DockAppointmentRepositoryError::Invalid(format!("{field} 不能为空")))
    .map_or(Ok(()), Err)
}

fn validate_window(
    window_start_at: DateTime<Utc>,
    window_end_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), DockAppointmentRepositoryError> {
    if window_end_at <= window_start_at {
        return Err(DockAppointmentRepositoryError::WindowInvalid);
    }
    if window_end_at <= now {
        return Err(DockAppointmentRepositoryError::WindowEnded);
    }
    Ok(())
}

fn validate_request(
    request: &CreateDockAppointmentRequest,
) -> Result<(), DockAppointmentRepositoryError> {
    [
        ("appointment_no", request.appointment_no.trim()),
        ("document_type", request.document_type.trim()),
        ("document_no", request.document_no.trim()),
        ("vehicle_type", request.vehicle_type.trim()),
        ("driver_name", request.driver_name.trim()),
        ("driver_phone", request.driver_phone.trim()),
    ]
    .into_iter()
    .find(|(_, value)| value.is_empty())
    .map(|(field, _)| DockAppointmentRepositoryError::Invalid(format!("{field} 不能为空")))
    .map_or(Ok(()), Err)
}

fn validate_arrival_request(
    request: &ArriveDockAppointmentRequest,
) -> Result<(), DockAppointmentRepositoryError> {
    [
        ("appointment_no", request.appointment_no.trim()),
        ("vehicle_plate_no", request.vehicle_plate_no.trim()),
        ("driver_name", request.driver_name.trim()),
        ("vehicle_type", request.vehicle_type.trim()),
    ]
    .into_iter()
    .find(|(_, value)| value.is_empty())
    .map(|(field, _)| DockAppointmentRepositoryError::Invalid(format!("{field} 不能为空")))
    .map_or(Ok(()), Err)
}

fn vehicle_type_matches_temperature_zone(vehicle_type: &str, temperature_zone: &str) -> bool {
    let vehicle_type = vehicle_type.to_lowercase();
    let temperature_zone = temperature_zone.to_lowercase();
    if matches!(temperature_zone.as_str(), "both" | "all" | "cold_chain") {
        return true;
    }
    let cold_vehicle = ["cold", "frozen", "refrigerated", "冷链", "冷藏", "冷冻"]
        .iter()
        .any(|marker| vehicle_type.contains(marker));
    let cold_zone = matches!(
        temperature_zone.as_str(),
        "cold" | "frozen" | "冷藏" | "冷冻" | "cold_chain" | "冷链"
    );
    cold_zone == cold_vehicle
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, DockAppointmentRepositoryError> {
    idempotency::replay(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        now,
    )
    .await
    .map_err(Into::into)
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), DockAppointmentRepositoryError> {
    idempotency::lock_key(tx, "dock-appointment", owner_id, idempotency_key)
        .await
        .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: &str,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), DockAppointmentRepositoryError> {
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        resource_id,
        response,
        now,
    )
    .await
    .map_err(Into::into)
}

fn request_hash(value: &serde_json::Value) -> Result<String, DockAppointmentRepositoryError> {
    idempotency::request_hash(value).map_err(Into::into)
}

impl From<DockAppointmentRow> for DockAppointment {
    fn from(row: DockAppointmentRow) -> Self {
        Self {
            id: row.id,
            dock_id: row.dock_id,
            owner_id: row.owner_id,
            warehouse_id: row.warehouse_id,
            status: row.status,
            appointment_no: row.appointment_no,
            document_type: row.document_type,
            document_no: row.document_no,
            window_start_at: row.window_start_at,
            window_end_at: row.window_end_at,
            vehicle_plate_no: row.vehicle_plate_no,
            vehicle_type: row.vehicle_type,
            driver_name: row.driver_name,
            driver_phone: row.driver_phone,
            supersedes_id: row.supersedes_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
            arrived_at: row.arrived_at,
            arrival_deviation_minutes: row.arrival_deviation_minutes,
        }
    }
}

fn map_db_error(error: sqlx::Error) -> DockAppointmentRepositoryError {
    DockAppointmentRepositoryError::Database(error.to_string())
}

fn map_create_error(error: sqlx::Error) -> DockAppointmentRepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.code().as_deref() == Some("23505") {
            return match database_error.constraint() {
                Some("ux_dock_appointments_appointment_no") => {
                    DockAppointmentRepositoryError::AppointmentNoConflict
                }
                Some("ux_dock_appointments_active") => {
                    DockAppointmentRepositoryError::ActiveAppointmentConflict
                }
                _ => DockAppointmentRepositoryError::Database(error.to_string()),
            };
        }
    }
    map_db_error(error)
}
