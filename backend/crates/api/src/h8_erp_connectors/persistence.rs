//! H8 ERP 连接 PostgreSQL 原子持久化边界。

use chrono::Utc;
use serde::de::DeserializeOwned;
use sqlx::{PgPool, Postgres, Transaction};
use wms_domain::{H8ErpConnector, H8ErpConnectorError, H8ErpConnectorTestResult};

use super::{
    error::H8ErpConnectorRepoError, idempotency::H8IdempotencyWrite,
    repository::H8ConnectorStatusTransition,
};
use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    idempotency,
};

pub(super) async fn insert(
    pool: &PgPool,
    connector: &H8ErpConnector,
) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    insert_in_tx(&mut tx, connector).await?;
    commit(tx).await?;
    Ok(connector.clone())
}

pub(super) async fn save(
    pool: &PgPool,
    connector: &H8ErpConnector,
    observed_version: i64,
    observed_probe_version: i64,
) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    save_in_tx(&mut tx, connector, observed_version, observed_probe_version).await?;
    commit(tx).await?;
    Ok(connector.clone())
}

pub(super) async fn delete(
    pool: &PgPool,
    owner_id: uuid::Uuid,
    connector_id: uuid::Uuid,
) -> Result<(), H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    delete_in_tx(&mut tx, owner_id, connector_id).await?;
    commit(tx).await
}

pub(super) async fn commit_create(
    pool: &PgPool,
    connector: &H8ErpConnector,
    audit_request: &AuditWriteRequest,
    idempotency: &H8IdempotencyWrite,
) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    if let Some(replayed) = replay_idempotency(&mut tx, idempotency).await? {
        commit(tx).await?;
        return Ok(replayed);
    }
    insert_in_tx(&mut tx, connector).await?;
    append_audit(&mut tx, audit_request).await?;
    store_idempotency(&mut tx, idempotency).await?;
    commit(tx).await?;
    Ok(connector.clone())
}

pub(super) async fn commit_update(
    pool: &PgPool,
    connector: &H8ErpConnector,
    observed_version: i64,
    observed_probe_version: i64,
    audit_request: &AuditWriteRequest,
    idempotency: &H8IdempotencyWrite,
) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    if let Some(replayed) = replay_idempotency(&mut tx, idempotency).await? {
        commit(tx).await?;
        return Ok(replayed);
    }
    save_in_tx(&mut tx, connector, observed_version, observed_probe_version).await?;
    append_audit(&mut tx, audit_request).await?;
    store_idempotency(&mut tx, idempotency).await?;
    commit(tx).await?;
    Ok(connector.clone())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn commit_test(
    pool: &PgPool,
    connector: &H8ErpConnector,
    observed_version: i64,
    observed_probe_version: i64,
    result: &H8ErpConnectorTestResult,
    audit_request: &AuditWriteRequest,
    idempotency: &H8IdempotencyWrite,
) -> Result<H8ErpConnectorTestResult, H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    if let Some(replayed) = replay_idempotency(&mut tx, idempotency).await? {
        commit(tx).await?;
        return Ok(replayed);
    }
    save_in_tx(&mut tx, connector, observed_version, observed_probe_version).await?;
    append_audit(&mut tx, audit_request).await?;
    store_idempotency(&mut tx, idempotency).await?;
    commit(tx).await?;
    Ok(result.clone())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn commit_status_transition(
    pool: &PgPool,
    connector: &H8ErpConnector,
    observed_version: i64,
    observed_probe_version: i64,
    transition: H8ConnectorStatusTransition,
    audit_request: &AuditWriteRequest,
    inflight_audit_request: Option<&AuditWriteRequest>,
    idempotency: &H8IdempotencyWrite,
) -> Result<(H8ErpConnector, u64), H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    if let Some(replayed) = replay_idempotency(&mut tx, idempotency).await? {
        commit(tx).await?;
        return Ok((replayed, 0));
    }
    save_in_tx(&mut tx, connector, observed_version, observed_probe_version).await?;
    let (from_status, to_status) = transition.inflight_statuses();
    let affected = sqlx::query(
        r#"UPDATE h8_erp_in_flight_messages
              SET status=$3, updated_at=now()
            WHERE owner_id=$1 AND connector_id=$2 AND status=$4"#,
    )
    .bind(connector.owner_id)
    .bind(connector.id)
    .bind(to_status)
    .bind(from_status)
    .execute(&mut *tx)
    .await
    .map_err(|error| H8ErpConnectorRepoError::Db(error.to_string()))?
    .rows_affected();
    append_audit(&mut tx, audit_request).await?;
    if affected > 0 {
        if let Some(request) = inflight_audit_request {
            append_audit(&mut tx, request).await?;
        }
    }
    store_idempotency(&mut tx, idempotency).await?;
    commit(tx).await?;
    Ok((connector.clone(), affected))
}

pub(super) async fn commit_delete(
    pool: &PgPool,
    owner_id: uuid::Uuid,
    connector_id: uuid::Uuid,
    audit_request: &AuditWriteRequest,
    idempotency: &H8IdempotencyWrite,
) -> Result<(), H8ErpConnectorRepoError> {
    let mut tx = begin(pool).await?;
    if replay_idempotency::<serde_json::Value>(&mut tx, idempotency)
        .await?
        .is_some()
    {
        commit(tx).await?;
        return Ok(());
    }
    delete_in_tx(&mut tx, owner_id, connector_id).await?;
    append_audit(&mut tx, audit_request).await?;
    store_idempotency(&mut tx, idempotency).await?;
    commit(tx).await
}

async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, H8ErpConnectorRepoError> {
    pool.begin()
        .await
        .map_err(|error| H8ErpConnectorRepoError::Db(error.to_string()))
}

async fn commit(tx: Transaction<'_, Postgres>) -> Result<(), H8ErpConnectorRepoError> {
    tx.commit()
        .await
        .map_err(|error| H8ErpConnectorRepoError::Db(error.to_string()))
}

async fn append_audit(
    tx: &mut Transaction<'_, Postgres>,
    request: &AuditWriteRequest,
) -> Result<(), H8ErpConnectorRepoError> {
    append_event_in_tx(tx, request)
        .await
        .map(|_| ())
        .map_err(|error| H8ErpConnectorRepoError::Db(format!("{error:?}")))
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    request: &H8IdempotencyWrite,
) -> Result<Option<T>, H8ErpConnectorRepoError> {
    idempotency::lock_key(tx, "h8-erp-connector", request.owner_id, &request.key)
        .await
        .map_err(map_idempotency_error)?;
    idempotency::replay(
        tx,
        request.owner_id,
        &request.key,
        &request.request_hash,
        &request.method,
        &request.path,
        Utc::now(),
    )
    .await
    .map_err(map_idempotency_error)
}

async fn store_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    request: &H8IdempotencyWrite,
) -> Result<(), H8ErpConnectorRepoError> {
    let now = Utc::now();
    idempotency::store_success_with_status(
        tx,
        request.owner_id,
        &request.key,
        &request.request_hash,
        &request.method,
        &request.path,
        request.status_code,
        "h8_erp_connector",
        &request.resource_id,
        &request.response_body,
        now,
    )
    .await
    .map_err(map_idempotency_error)
}

fn map_idempotency_error(error: idempotency::IdempotencyError) -> H8ErpConnectorRepoError {
    match error {
        idempotency::IdempotencyError::Conflict => {
            H8ErpConnectorRepoError::Domain(H8ErpConnectorError::IdempotencyConflict)
        }
        idempotency::IdempotencyError::Database(error) => {
            H8ErpConnectorRepoError::Db(error.to_string())
        }
        idempotency::IdempotencyError::Serialize(error) => H8ErpConnectorRepoError::Db(error),
    }
}

async fn insert_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    connector: &H8ErpConnector,
) -> Result<(), H8ErpConnectorRepoError> {
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors (
               id, owner_id, connector_code, connector_name, warehouse_ids, directions,
               message_types, channel_mode, api_base_url, interface_db_host, interface_db_port,
               interface_db_name, interface_db_username, api_key_id, bearer_secret_alias,
               interface_db_password_alias, interface_probe_db_username,
               interface_probe_db_password_alias, interface_probe_config_version,
               status, config_version, first_activated_at,
               last_tested_version, last_tested_at, last_tested_succeeded,
               last_tested_error_summary, created_at, updated_at
           ) VALUES (
               $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
               $21,$22,$23,$24,$25,$26,$27,$28
           )"#,
    )
    .bind(connector.id)
    .bind(connector.owner_id)
    .bind(&connector.connector_code)
    .bind(&connector.connector_name)
    .bind(&connector.warehouse_ids)
    .bind(&connector.directions)
    .bind(&connector.message_types)
    .bind(&connector.channel_mode)
    .bind(&connector.api_base_url)
    .bind(&connector.interface_db_host)
    .bind(connector.interface_db_port)
    .bind(&connector.interface_db_name)
    .bind(&connector.interface_db_username)
    .bind(connector.api_key_id)
    .bind(&connector.bearer_secret_alias)
    .bind(&connector.interface_db_password_alias)
    .bind(&connector.interface_probe_db_username)
    .bind(&connector.interface_probe_db_password_alias)
    .bind(connector.interface_probe_config_version)
    .bind(&connector.status)
    .bind(connector.config_version)
    .bind(connector.first_activated_at)
    .bind(connector.last_tested_version)
    .bind(connector.last_tested_at)
    .bind(connector.last_tested_succeeded)
    .bind(&connector.last_tested_error_summary)
    .bind(connector.created_at)
    .bind(connector.updated_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| {
        let message = error.to_string();
        if message.contains("uq_h8_erp_connectors_owner_code") {
            H8ErpConnectorRepoError::DuplicateCode
        } else {
            H8ErpConnectorRepoError::Db(message)
        }
    })?;
    Ok(())
}

async fn delete_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: uuid::Uuid,
    connector_id: uuid::Uuid,
) -> Result<(), H8ErpConnectorRepoError> {
    let result = sqlx::query("DELETE FROM h8_erp_connectors WHERE owner_id=$1 AND id=$2")
        .bind(owner_id)
        .bind(connector_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| H8ErpConnectorRepoError::Db(error.to_string()))?;
    if result.rows_affected() == 0 {
        return Err(H8ErpConnectorRepoError::Domain(
            H8ErpConnectorError::NotFound,
        ));
    }
    Ok(())
}

async fn save_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    connector: &H8ErpConnector,
    observed_version: i64,
    observed_probe_version: i64,
) -> Result<(), H8ErpConnectorRepoError> {
    let result = sqlx::query(
        r#"UPDATE h8_erp_connectors SET
               connector_name=$3, warehouse_ids=$4, directions=$5, message_types=$6,
               channel_mode=$7, api_base_url=$8, interface_db_host=$9,
               interface_db_port=$10, interface_db_name=$11, interface_db_username=$12,
               api_key_id=$13, bearer_secret_alias=$14, interface_db_password_alias=$15,
               interface_probe_db_username=$16, interface_probe_db_password_alias=$17,
               interface_probe_config_version=$18, status=$19, config_version=$20,
               first_activated_at=$21, last_tested_version=$22, last_tested_at=$23,
               last_tested_succeeded=$24, last_tested_error_summary=$25, updated_at=$26
           WHERE owner_id=$1 AND id=$2 AND config_version=$27
             AND interface_probe_config_version=$28"#,
    )
    .bind(connector.owner_id)
    .bind(connector.id)
    .bind(&connector.connector_name)
    .bind(&connector.warehouse_ids)
    .bind(&connector.directions)
    .bind(&connector.message_types)
    .bind(&connector.channel_mode)
    .bind(&connector.api_base_url)
    .bind(&connector.interface_db_host)
    .bind(connector.interface_db_port)
    .bind(&connector.interface_db_name)
    .bind(&connector.interface_db_username)
    .bind(connector.api_key_id)
    .bind(&connector.bearer_secret_alias)
    .bind(&connector.interface_db_password_alias)
    .bind(&connector.interface_probe_db_username)
    .bind(&connector.interface_probe_db_password_alias)
    .bind(connector.interface_probe_config_version)
    .bind(&connector.status)
    .bind(connector.config_version)
    .bind(connector.first_activated_at)
    .bind(connector.last_tested_version)
    .bind(connector.last_tested_at)
    .bind(connector.last_tested_succeeded)
    .bind(&connector.last_tested_error_summary)
    .bind(connector.updated_at)
    .bind(observed_version)
    .bind(observed_probe_version)
    .execute(&mut **tx)
    .await
    .map_err(|error| H8ErpConnectorRepoError::Db(error.to_string()))?;
    if result.rows_affected() > 0 {
        return Ok(());
    }
    let versions: Option<(i64, i64)> = sqlx::query_as(
        r#"SELECT config_version, interface_probe_config_version
             FROM h8_erp_connectors
            WHERE owner_id=$1 AND id=$2"#,
    )
    .bind(connector.owner_id)
    .bind(connector.id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| H8ErpConnectorRepoError::Db(error.to_string()))?;
    Err(H8ErpConnectorRepoError::Domain(match versions {
        Some((config_version, _)) if config_version != observed_version => {
            H8ErpConnectorError::VersionConflict
        }
        Some((_, probe_version)) if probe_version != observed_probe_version => {
            H8ErpConnectorError::ProbeVersionConflict
        }
        Some(_) => H8ErpConnectorError::VersionConflict,
        None => H8ErpConnectorError::NotFound,
    }))
}
