use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    validate_create_maintenance_record_request, CreateMaintenanceRecordRequest, MaintenanceRecord,
    MaintenanceRecordQuery, MaintenanceTask, MaintenanceTaskQuery,
};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
};

use super::{
    map_db_error, replay_idempotency, request_hash, store_idempotency_success, IdempotentMutation,
    PgWave3Repository, Wave3RepositoryError,
};

#[derive(Clone, FromRow)]
struct MaintenanceTaskRow {
    id: Uuid,
    owner_id: Uuid,
    inventory_batch_id: Uuid,
    product_code: String,
    batch_no: String,
    expiry_date: NaiveDate,
    quality_status: String,
    location_id: Uuid,
    location_code: String,
    planned_at: DateTime<Utc>,
    status: String,
    assigned_user_id: Option<Uuid>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Clone, FromRow)]
struct MaintenanceRecordRow {
    id: Uuid,
    task_id: Uuid,
    owner_id: Uuid,
    inventory_batch_id: Uuid,
    product_code: String,
    batch_no: String,
    expiry_date: NaiveDate,
    inventory_status: String,
    temperature_celsius: f64,
    humidity_percent: f64,
    appearance: String,
    packaging: String,
    pest: String,
    rodent: String,
    mildew: String,
    conclusion: String,
    exception_type: Option<String>,
    notes: Option<String>,
    performed_by: Uuid,
    performed_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub async fn list_maintenance_tasks(
        &self,
        ctx: &AuthContext,
        query: MaintenanceTaskQuery,
    ) -> Result<Vec<MaintenanceTask>, Wave3RepositoryError> {
        let status = query.status.filter(|value| !value.trim().is_empty());
        let rows = sqlx::query_as::<_, MaintenanceTaskRow>(
            r#"
            SELECT task.id,
                   task.owner_id,
                   task.inventory_batch_id,
                   batch.product_code,
                   batch.batch_no,
                   batch.expiry_date,
                   batch.quality_status,
                   batch.location_id,
                   batch.location_code,
                   task.planned_at,
                   task.status,
                   task.assigned_user_id,
                   task.completed_at,
                   task.created_at
              FROM inventory_maintenance_tasks AS task
              JOIN inventory_batches AS batch
                ON batch.id = task.inventory_batch_id
               AND batch.owner_id = task.owner_id
             WHERE task.owner_id = $1
               AND ($2::UUID IS NULL OR task.id = $2)
               AND ($3::UUID IS NULL OR task.inventory_batch_id = $3)
               AND ($4::TEXT IS NULL OR task.status = $4)
             ORDER BY task.planned_at ASC, task.id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(query.task_id)
        .bind(query.batch_id)
        .bind(status.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(map_maintenance_task).collect())
    }

    pub async fn list_maintenance_records(
        &self,
        ctx: &AuthContext,
        query: MaintenanceRecordQuery,
    ) -> Result<Vec<MaintenanceRecord>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, MaintenanceRecordRow>(
            r#"
            SELECT id, task_id, owner_id, inventory_batch_id, product_code, batch_no,
                   expiry_date, inventory_status, temperature_celsius, humidity_percent,
                   appearance, packaging, pest, rodent, mildew, conclusion,
                   exception_type, notes, performed_by, performed_at
              FROM inventory_maintenance_records
             WHERE owner_id = $1
               AND ($2::UUID IS NULL OR task_id = $2)
               AND ($3::UUID IS NULL OR inventory_batch_id = $3)
             ORDER BY performed_at DESC, id DESC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(query.task_id)
        .bind(query.batch_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(map_maintenance_record).collect())
    }

    pub async fn create_maintenance_record_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateMaintenanceRecordRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<MaintenanceRecord>, Wave3RepositoryError> {
        validate_create_maintenance_record_request(&req)
            .map_err(|_| Wave3RepositoryError::InvalidMaintenanceResult)?;
        let request_hash = request_hash(&serde_json::json!({ "request": &req }))?;
        let mut tx = self.begin().await?;
        super::lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<MaintenanceRecord>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let task = sqlx::query_as::<_, MaintenanceTaskRow>(
            r#"
            SELECT task.id,
                   task.owner_id,
                   task.inventory_batch_id,
                   batch.product_code,
                   batch.batch_no,
                   batch.expiry_date,
                   batch.quality_status,
                   batch.location_id,
                   batch.location_code,
                   task.planned_at,
                   task.status,
                   task.assigned_user_id,
                   task.completed_at,
                   task.created_at
              FROM inventory_maintenance_tasks AS task
              JOIN inventory_batches AS batch
                ON batch.id = task.inventory_batch_id
               AND batch.owner_id = task.owner_id
             WHERE task.id = $1 AND task.owner_id = $2
             FOR UPDATE OF task, batch
            "#,
        )
        .bind(req.task_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;

        let performed_date = now.date_naive();
        if let Some(existing) =
            load_maintenance_record(&mut tx, ctx.owner_id, task.id, performed_date).await?
        {
            let record = map_maintenance_record(existing);
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &request_hash,
                "POST",
                "/api/v1/inventory/maintenance/records",
                "inventory_maintenance_record",
                record.id.to_string(),
                &record,
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value: record,
                replayed: true,
            });
        }

        if task.status != "pending" {
            return Err(Wave3RepositoryError::InvalidMaintenanceTaskState);
        }
        if task.expiry_date <= performed_date {
            return Err(Wave3RepositoryError::BatchExpired);
        }
        if !matches!(
            task.quality_status.as_str(),
            STATUS_QUALIFIED | STATUS_QUARANTINED
        ) {
            return Err(Wave3RepositoryError::InvalidInventoryState);
        }

        let record = MaintenanceRecord {
            id: Uuid::new_v4(),
            task_id: task.id,
            owner_id: ctx.owner_id,
            batch_id: task.inventory_batch_id,
            product_code: task.product_code.clone(),
            batch_no: task.batch_no.clone(),
            expiry_date: task.expiry_date,
            inventory_status: task.quality_status.clone(),
            temperature_celsius: req.temperature_celsius,
            humidity_percent: req.humidity_percent,
            appearance: req.appearance.clone(),
            packaging: req.packaging.clone(),
            pest: req.pest.clone(),
            rodent: req.rodent.clone(),
            mildew: req.mildew.clone(),
            conclusion: req.conclusion.clone(),
            exception_type: req.exception_type.clone(),
            notes: req.notes.clone(),
            performed_by: ctx.user_id,
            performed_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO inventory_maintenance_records (
                id, task_id, owner_id, inventory_batch_id, product_code, batch_no,
                expiry_date, inventory_status, temperature_celsius, humidity_percent,
                appearance, packaging, pest, rodent, mildew, conclusion,
                exception_type, notes, performed_by, performed_at, performed_date
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                    $14, $15, $16, $17, $18, $19, $20, $21)
            "#,
        )
        .bind(record.id)
        .bind(record.task_id)
        .bind(record.owner_id)
        .bind(record.batch_id)
        .bind(&record.product_code)
        .bind(&record.batch_no)
        .bind(record.expiry_date)
        .bind(&record.inventory_status)
        .bind(record.temperature_celsius)
        .bind(record.humidity_percent)
        .bind(&record.appearance)
        .bind(&record.packaging)
        .bind(&record.pest)
        .bind(&record.rodent)
        .bind(&record.mildew)
        .bind(&record.conclusion)
        .bind(&record.exception_type)
        .bind(&record.notes)
        .bind(record.performed_by)
        .bind(record.performed_at)
        .bind(performed_date)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        sqlx::query(
            r#"
            UPDATE inventory_maintenance_tasks
               SET status = 'completed', completed_at = $3
             WHERE id = $1 AND owner_id = $2 AND status = 'pending'
            "#,
        )
        .bind(task.id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut audit_event = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "create_maintenance_record",
                "M3",
                "inventory_maintenance_record",
                record.id.to_string(),
                None,
            )
        });
        audit_event.occurred_at = now;
        audit_event.module = "M3".to_string();
        audit_event.resource_type = "inventory_maintenance_record".to_string();
        audit_event.resource_id = record.id.to_string();
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/maintenance/records",
            "inventory_maintenance_record",
            record.id.to_string(),
            &record,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: record,
            replayed: false,
        })
    }
}

async fn load_maintenance_record(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    task_id: Uuid,
    performed_date: NaiveDate,
) -> Result<Option<MaintenanceRecordRow>, Wave3RepositoryError> {
    sqlx::query_as::<_, MaintenanceRecordRow>(
        r#"
        SELECT id, task_id, owner_id, inventory_batch_id, product_code, batch_no,
               expiry_date, inventory_status, temperature_celsius, humidity_percent,
               appearance, packaging, pest, rodent, mildew, conclusion,
               exception_type, notes, performed_by, performed_at
          FROM inventory_maintenance_records
         WHERE owner_id = $1 AND task_id = $2 AND performed_date = $3
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(task_id)
    .bind(performed_date)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

fn map_maintenance_task(row: MaintenanceTaskRow) -> MaintenanceTask {
    MaintenanceTask {
        id: row.id,
        owner_id: row.owner_id,
        batch_id: row.inventory_batch_id,
        product_code: row.product_code,
        batch_no: row.batch_no,
        expiry_date: row.expiry_date,
        quality_status: row.quality_status,
        location_id: row.location_id,
        location_code: row.location_code,
        planned_at: row.planned_at,
        status: row.status,
        assigned_user_id: row.assigned_user_id,
        completed_at: row.completed_at,
        created_at: row.created_at,
    }
}

fn map_maintenance_record(row: MaintenanceRecordRow) -> MaintenanceRecord {
    MaintenanceRecord {
        id: row.id,
        task_id: row.task_id,
        owner_id: row.owner_id,
        batch_id: row.inventory_batch_id,
        product_code: row.product_code,
        batch_no: row.batch_no,
        expiry_date: row.expiry_date,
        inventory_status: row.inventory_status,
        temperature_celsius: row.temperature_celsius,
        humidity_percent: row.humidity_percent,
        appearance: row.appearance,
        packaging: row.packaging,
        pest: row.pest,
        rodent: row.rodent,
        mildew: row.mildew,
        conclusion: row.conclusion,
        exception_type: row.exception_type,
        notes: row.notes,
        performed_by: row.performed_by,
        performed_at: row.performed_at,
    }
}
