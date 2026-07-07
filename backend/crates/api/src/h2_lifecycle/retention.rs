use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::support::{append_system_audit_in_tx, db_error, subtract_months};
use super::types::{
    BusinessArchiveJob, BusinessArchiveJobRow, BusinessRetentionPolicy, BusinessRetentionPolicyRow,
    H2LifecycleError,
};

pub async fn seed_default_business_retention_policies(
    pool: &PgPool,
    owner_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Vec<BusinessRetentionPolicy>, H2LifecycleError> {
    let defaults = [
        ("normal_business", "普通业务数据", Some(5), 12, false, false),
        (
            "special_drug_ledger",
            "特殊药品流通台账",
            Some(30),
            12,
            false,
            true,
        ),
        (
            "traceability_report",
            "追溯码上报记录",
            Some(5),
            12,
            false,
            false,
        ),
        ("cold_chain", "冷链温湿度", Some(5), 12, false, false),
        ("master_data", "主数据", None, 12, true, false),
        ("system_config", "系统配置变更", Some(3), 12, false, false),
    ];

    for (code, name, years, online_months, permanent, special_drug) in defaults {
        sqlx::query(
            r#"
            INSERT INTO business_retention_policy (
                id, owner_id, policy_code, policy_name, retention_years,
                online_retention_months, permanent, special_drug, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            ON CONFLICT (owner_id, policy_code)
            DO UPDATE SET
                policy_name = EXCLUDED.policy_name,
                retention_years = EXCLUDED.retention_years,
                online_retention_months = EXCLUDED.online_retention_months,
                permanent = EXCLUDED.permanent,
                special_drug = EXCLUDED.special_drug,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(code)
        .bind(name)
        .bind(years)
        .bind(online_months)
        .bind(permanent)
        .bind(special_drug)
        .bind(now)
        .execute(pool)
        .await
        .map_err(db_error)?;
    }

    list_business_retention_policies(pool, owner_id).await
}

pub async fn list_business_retention_policies(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<Vec<BusinessRetentionPolicy>, H2LifecycleError> {
    let rows: Vec<BusinessRetentionPolicyRow> = sqlx::query_as(
        r#"
        SELECT id, owner_id, policy_code, retention_years,
               online_retention_months, permanent, special_drug
          FROM business_retention_policy
         WHERE owner_id = $1
         ORDER BY policy_code
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    Ok(rows
        .into_iter()
        .map(BusinessRetentionPolicy::from)
        .collect())
}

pub async fn plan_business_archive_job(
    pool: &PgPool,
    owner_id: Uuid,
    policy_code: &str,
    table_name: &str,
    reference_date: NaiveDate,
    now: DateTime<Utc>,
    idempotency_key: &str,
) -> Result<BusinessArchiveJob, H2LifecycleError> {
    if let Some(existing) = load_business_archive_job(pool, owner_id, idempotency_key).await? {
        return Ok(existing);
    }
    let policy = load_business_retention_policy(pool, owner_id, policy_code).await?;
    let cutoff_date = if policy.permanent {
        None
    } else {
        Some(subtract_months(
            reference_date,
            policy.online_retention_months,
        )?)
    };
    let (target_layer, status, skip_reason) = if policy.permanent {
        ("skip", "skipped", Some("主数据永久保留，不归档"))
    } else {
        ("archive", "planned", None)
    };
    let mut tx = pool.begin().await.map_err(db_error)?;
    let row = sqlx::query_as::<_, BusinessArchiveJobRow>(
        r#"
        INSERT INTO business_archive_job (
            id, owner_id, idempotency_key, policy_id, table_name, target_layer,
            cutoff_date, status, delete_allowed, skip_reason, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE, $9, $10)
        RETURNING id, owner_id, $11::text AS policy_code, table_name,
                  target_layer, status, cutoff_date, delete_allowed
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(policy.id)
    .bind(table_name)
    .bind(target_layer)
    .bind(cutoff_date)
    .bind(status)
    .bind(skip_reason)
    .bind(now)
    .bind(&policy.policy_code)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;

    let job = BusinessArchiveJob::from(row);
    append_system_audit_in_tx(
        &mut tx,
        owner_id,
        "business_archive.plan",
        "business_archive_job",
        &job.id.to_string(),
        serde_json::json!({
            "policy_code": policy.policy_code,
            "table_name": table_name,
            "target_layer": target_layer,
            "cutoff_date": cutoff_date,
            "delete_allowed": false,
        }),
        now,
        "system-archive-business",
    )
    .await?;
    tx.commit().await.map_err(db_error)?;

    Ok(job)
}

async fn load_business_retention_policy(
    pool: &PgPool,
    owner_id: Uuid,
    policy_code: &str,
) -> Result<BusinessRetentionPolicy, H2LifecycleError> {
    sqlx::query_as::<_, BusinessRetentionPolicyRow>(
        r#"
        SELECT id, owner_id, policy_code, retention_years,
               online_retention_months, permanent, special_drug
          FROM business_retention_policy
         WHERE owner_id = $1 AND policy_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(policy_code)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    .map(BusinessRetentionPolicy::from)
    .ok_or(H2LifecycleError::NotFound)
}

async fn load_business_archive_job(
    pool: &PgPool,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<BusinessArchiveJob>, H2LifecycleError> {
    let row = sqlx::query_as::<_, BusinessArchiveJobRow>(
        r#"
        SELECT j.id, j.owner_id, p.policy_code, j.table_name,
               j.target_layer, j.status, j.cutoff_date, j.delete_allowed
          FROM business_archive_job j
          JOIN business_retention_policy p ON p.id = j.policy_id
         WHERE j.owner_id = $1 AND j.idempotency_key = $2
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?;
    Ok(row.map(BusinessArchiveJob::from))
}
