use chrono::{DateTime, Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::support::{add_months, append_system_audit_in_tx, db_error};
use super::types::{
    AuditArchiveRun, AuditArchiveRunRow, AuditPartitionState, AuditPartitionStateRow,
    H2LifecycleError, StorageTier,
};
use super::{DEFAULT_AUDIT_RETENTION_YEARS, DEFAULT_ONLINE_QUARTERS};

pub fn audit_target_tier(
    partition_start: NaiveDate,
    reference_date: NaiveDate,
    online_quarters: i32,
    retention_years: i32,
) -> StorageTier {
    let age_months = months_between(partition_start, reference_date).max(0);
    if age_months >= retention_years * 12 {
        StorageTier::DeepArchive
    } else if age_months >= online_quarters * 3 {
        StorageTier::Archive
    } else {
        StorageTier::Online
    }
}

pub async fn sync_audit_partition_states(
    pool: &PgPool,
    reference_date: NaiveDate,
) -> Result<Vec<AuditPartitionState>, H2LifecycleError> {
    let names: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT c.relname
          FROM pg_inherits i
          JOIN pg_class c ON c.oid = i.inhrelid
         WHERE i.inhparent = 'public.audit_event'::regclass
           AND c.relname ~ '^audit_event_[0-9]{4}_[0-9]{2}$'
         ORDER BY c.relname
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_error)?;

    for name in names {
        let partition_start = parse_audit_partition_start(&name)?;
        let partition_end = add_months(partition_start, 1)?;
        let target = audit_target_tier(
            partition_start,
            reference_date,
            DEFAULT_ONLINE_QUARTERS,
            DEFAULT_AUDIT_RETENTION_YEARS,
        );
        sqlx::query(
            r#"
            INSERT INTO audit_archive_partition_state (
                partition_name, partition_start, partition_end, storage_tier, target_tier, updated_at
            )
            VALUES ($1, $2, $3, 'online', $4, now())
            ON CONFLICT (partition_name)
            DO UPDATE SET
                partition_start = EXCLUDED.partition_start,
                partition_end = EXCLUDED.partition_end,
                target_tier = EXCLUDED.target_tier,
                updated_at = now()
            "#,
        )
        .bind(&name)
        .bind(partition_start)
        .bind(partition_end)
        .bind(target.as_str())
        .execute(pool)
        .await
        .map_err(db_error)?;
    }

    load_audit_partition_states(pool).await
}

pub async fn list_audit_partition_states(
    pool: &PgPool,
) -> Result<Vec<AuditPartitionState>, H2LifecycleError> {
    load_audit_partition_states(pool).await
}

pub async fn run_audit_archive_cycle(
    pool: &PgPool,
    owner_id: Uuid,
    reference_date: NaiveDate,
    now: DateTime<Utc>,
    idempotency_key: &str,
) -> Result<AuditArchiveRun, H2LifecycleError> {
    if let Some(existing) = load_audit_archive_run(pool, owner_id, idempotency_key).await? {
        return Ok(existing);
    }

    let states = sync_audit_partition_states(pool, reference_date).await?;
    let partitions_seen = states.len() as i32;
    let partitions_archived = states
        .iter()
        .filter(|state| {
            state.target_tier != StorageTier::Online && state.storage_tier != state.target_tier
        })
        .count() as i32;
    let run_id = Uuid::new_v4();

    let mut tx = pool.begin().await.map_err(db_error)?;
    let row = sqlx::query_as::<_, AuditArchiveRunRow>(
        r#"
        INSERT INTO audit_archive_run (
            id, owner_id, idempotency_key, reference_date, online_quarters, retention_years,
            partitions_seen, partitions_archived, status, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'completed', $9)
        RETURNING id, owner_id, reference_date, partitions_seen, partitions_archived, created_at
        "#,
    )
    .bind(run_id)
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(reference_date)
    .bind(DEFAULT_ONLINE_QUARTERS)
    .bind(DEFAULT_AUDIT_RETENTION_YEARS)
    .bind(partitions_seen)
    .bind(partitions_archived)
    .bind(now)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;

    sqlx::query(
        r#"
        UPDATE audit_archive_partition_state
           SET storage_tier = target_tier,
               archived_at = $2,
               last_run_id = $1,
               updated_at = $2
         WHERE target_tier <> 'online'
           AND storage_tier <> target_tier
        "#,
    )
    .bind(run_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;

    append_system_audit_in_tx(
        &mut tx,
        owner_id,
        "audit.archive.run",
        "audit_archive_run",
        &run_id.to_string(),
        serde_json::json!({
            "reference_date": reference_date,
            "partitions_seen": partitions_seen,
            "partitions_archived": partitions_archived,
        }),
        now,
        "system-archive",
    )
    .await?;
    tx.commit().await.map_err(db_error)?;

    Ok(row.into())
}

async fn load_audit_partition_states(
    pool: &PgPool,
) -> Result<Vec<AuditPartitionState>, H2LifecycleError> {
    let rows: Vec<AuditPartitionStateRow> = sqlx::query_as(
        r#"
        SELECT partition_name, partition_start, partition_end, storage_tier, target_tier
          FROM audit_archive_partition_state
         ORDER BY partition_start
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_error)?;

    rows.into_iter()
        .map(AuditPartitionState::try_from)
        .collect()
}

async fn load_audit_archive_run(
    pool: &PgPool,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<AuditArchiveRun>, H2LifecycleError> {
    let row = sqlx::query_as::<_, AuditArchiveRunRow>(
        r#"
        SELECT id, owner_id, reference_date, partitions_seen, partitions_archived, created_at
          FROM audit_archive_run
         WHERE owner_id = $1 AND idempotency_key = $2
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?;
    Ok(row.map(AuditArchiveRun::from))
}

fn parse_audit_partition_start(name: &str) -> Result<NaiveDate, H2LifecycleError> {
    let suffix = name
        .strip_prefix("audit_event_")
        .ok_or_else(|| H2LifecycleError::InvalidInput(format!("invalid partition: {name}")))?;
    NaiveDate::parse_from_str(&format!("{suffix}_01"), "%Y_%m_%d")
        .map_err(|error| H2LifecycleError::InvalidInput(error.to_string()))
}

fn months_between(start: NaiveDate, reference: NaiveDate) -> i32 {
    (reference.year() - start.year()) * 12 + reference.month() as i32 - start.month() as i32
}
