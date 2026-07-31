use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{CreateDockImportRequest, CreateDockRequest, Dock, UpdateDockRequest};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    idempotency,
    operation_context::OperationContext as AuthContext,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DockRepositoryError {
    NotFound,
    DuplicateCode,
    InUse(i64),
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<crate::idempotency::IdempotencyError> for DockRepositoryError {
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
pub struct PgDockRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, FromRow)]
struct DockRow {
    id: Uuid,
    warehouse_id: Uuid,
    dock_code: String,
    dock_type: String,
    temperature_zone: String,
    status: String,
    maintenance_recovery_at: Option<DateTime<Utc>>,
    location_description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgDockRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_dock(
        &self,
        ctx: &AuthContext,
        req: CreateDockRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Dock, DockRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "request": &req }))?;
        let path = "/api/v1/docks";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<Dock>(
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
        ensure_warehouse(&mut tx, ctx.owner_id, req.warehouse_id).await?;
        let row = sqlx::query_as::<_, DockRow>(
            "INSERT INTO warehouse_docks (id, warehouse_id, dock_code, dock_type, temperature_zone, location_description, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$7) RETURNING id, warehouse_id, dock_code, dock_type, temperature_zone, status, maintenance_recovery_at, location_description, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(req.warehouse_id)
        .bind(&req.dock_code)
        .bind(&req.dock_type)
        .bind(&req.temperature_zone)
        .bind(&req.location_description)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;
        let dock: Dock = row.into();
        append_dock_audit(&mut tx, ctx, "create_dock", &dock, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "warehouse_dock",
            &dock.id.to_string(),
            &dock,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(dock)
    }

    pub async fn list_docks(
        &self,
        ctx: &AuthContext,
        warehouse_id: Uuid,
    ) -> Result<Vec<Dock>, DockRepositoryError> {
        let rows = sqlx::query_as::<_, DockRow>(
            "SELECT d.id, d.warehouse_id, d.dock_code, d.dock_type, d.temperature_zone, d.status, d.maintenance_recovery_at, d.location_description, d.created_at, d.updated_at FROM warehouse_docks d JOIN warehouses w ON w.id=d.warehouse_id WHERE d.warehouse_id=$1 AND w.owner_id=$2 ORDER BY d.dock_code",
        )
        .bind(warehouse_id)
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update_dock(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateDockRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Dock, DockRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "dock_id": id,
            "request": &req,
        }))?;
        let path = "/api/v1/docks/{id}";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<Dock>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }
        let before = sqlx::query_as::<_, DockRow>(
            "SELECT d.id, d.warehouse_id, d.dock_code, d.dock_type, d.temperature_zone, d.status, d.maintenance_recovery_at, d.location_description, d.created_at, d.updated_at FROM warehouse_docks d JOIN warehouses w ON w.id=d.warehouse_id WHERE d.id=$1 AND w.owner_id=$2 FOR UPDATE",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(DockRepositoryError::NotFound)?;
        let row = sqlx::query_as::<_, DockRow>(
            "UPDATE warehouse_docks SET status=$3, maintenance_recovery_at=$4, updated_at=$5 WHERE id=$1 AND warehouse_id=$2 RETURNING id, warehouse_id, dock_code, dock_type, temperature_zone, status, maintenance_recovery_at, location_description, created_at, updated_at",
        )
        .bind(id)
        .bind(before.warehouse_id)
        .bind(&req.status)
        .bind(req.maintenance_recovery_at)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;
        let dock: Dock = row.into();
        append_dock_audit(&mut tx, ctx, "update_dock", &dock, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            path,
            "warehouse_dock",
            &dock.id.to_string(),
            &dock,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(dock)
    }

    pub async fn import_docks(
        &self,
        ctx: &AuthContext,
        req: CreateDockImportRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Vec<Dock>, DockRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "request": &req }))?;
        let path = "/api/v1/docks/import";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<Vec<Dock>>(
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
        let warehouse_id = req.warehouse_id;
        let items = req.docks;
        ensure_warehouse(&mut tx, ctx.owner_id, warehouse_id).await?;
        let mut docks = Vec::with_capacity(items.len());
        for item in items {
            let row = sqlx::query_as::<_, DockRow>(
                "INSERT INTO warehouse_docks (id, warehouse_id, dock_code, dock_type, temperature_zone, location_description, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$7) RETURNING id, warehouse_id, dock_code, dock_type, temperature_zone, status, maintenance_recovery_at, location_description, created_at, updated_at",
            )
            .bind(Uuid::new_v4())
            .bind(warehouse_id)
            .bind(&item.dock_code)
            .bind(&item.dock_type)
            .bind(&item.temperature_zone)
            .bind(&item.location_description)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_write_error)?;
            let dock: Dock = row.into();
            append_dock_audit(&mut tx, ctx, "import_dock", &dock, now).await?;
            docks.push(dock);
        }
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "warehouse_dock",
            &warehouse_id.to_string(),
            &docks,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(docks)
    }

    pub async fn delete_dock(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<(), DockRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "dock_id": id }))?;
        let path = "/api/v1/docks/{id}";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if replay_idempotency::<()>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "DELETE",
            path,
            now,
        )
        .await?
        .is_some()
        {
            return Ok(());
        }
        let row = sqlx::query_as::<_, DockRow>(
            "SELECT d.id, d.warehouse_id, d.dock_code, d.dock_type, d.temperature_zone, d.status, d.maintenance_recovery_at, d.location_description, d.created_at, d.updated_at FROM warehouse_docks d JOIN warehouses w ON w.id=d.warehouse_id WHERE d.id=$1 AND w.owner_id=$2 FOR UPDATE",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(DockRepositoryError::NotFound)?;
        let active_appointments: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::BIGINT FROM dock_appointments WHERE dock_id=$1 AND owner_id=$2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if active_appointments > 0 {
            return Err(DockRepositoryError::InUse(active_appointments));
        }
        let dock: Dock = row.into();
        sqlx::query("DELETE FROM warehouse_docks WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        append_dock_audit(&mut tx, ctx, "delete_dock", &dock, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "DELETE",
            path,
            "warehouse_dock",
            &id.to_string(),
            &(),
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(())
    }
}

async fn ensure_warehouse(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    warehouse_id: Uuid,
) -> Result<(), DockRepositoryError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM warehouses WHERE id=$1 AND owner_id=$2 AND status='active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    exists.then_some(()).ok_or(DockRepositoryError::NotFound)
}

async fn append_dock_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    after: &Dock,
    now: DateTime<Utc>,
) -> Result<(), DockRepositoryError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H-DOCK",
        "warehouse_dock",
        after.id.to_string(),
        None,
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| DockRepositoryError::Audit(format!("{error:?}")))
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, DockRepositoryError> {
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
) -> Result<(), DockRepositoryError> {
    idempotency::lock_key(tx, "dock", owner_id, idempotency_key)
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
) -> Result<(), DockRepositoryError> {
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

fn request_hash(value: &serde_json::Value) -> Result<String, DockRepositoryError> {
    idempotency::request_hash(value).map_err(Into::into)
}

impl From<DockRow> for Dock {
    fn from(row: DockRow) -> Self {
        Self {
            id: row.id,
            warehouse_id: row.warehouse_id,
            dock_code: row.dock_code,
            dock_type: row.dock_type,
            temperature_zone: row.temperature_zone,
            status: row.status,
            maintenance_recovery_at: row.maintenance_recovery_at,
            location_description: row.location_description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn map_db_error(error: sqlx::Error) -> DockRepositoryError {
    DockRepositoryError::Database(error.to_string())
}

fn map_write_error(error: sqlx::Error) -> DockRepositoryError {
    if let sqlx::Error::Database(db_error) = &error {
        if db_error.code().as_deref() == Some("23505") {
            return DockRepositoryError::DuplicateCode;
        }
    }
    map_db_error(error)
}
