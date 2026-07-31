//! US-H9-011 print device rows, mapping helpers and idempotency support.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{DeviceLease, PrintSite, PrintSiteOwnerMapping, Printer, PrinterTray};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
};

use super::{PrintDeviceError, DEFAULT_RELEASE_MODE};

const USER_OWNER_PERMISSION_SQL: &str = r#"
    WITH RECURSIVE hierarchy AS (
        SELECT role.id, role.parent_role_id, 0 AS depth
          FROM auth_user_roles user_role
          JOIN auth_roles role
            ON role.id = user_role.role_id
           AND role.owner_id = user_role.owner_id
         WHERE user_role.user_id = $1
           AND user_role.owner_id = $2
        UNION ALL
        SELECT parent.id, parent.parent_role_id, hierarchy.depth + 1
          FROM auth_roles parent
          JOIN hierarchy ON hierarchy.parent_role_id = parent.id
         WHERE parent.owner_id = $2
    ), decisions AS (
        SELECT grant_row.permission_id, hierarchy.depth, TRUE AS allowed
          FROM hierarchy
          JOIN auth_role_permissions grant_row ON grant_row.role_id = hierarchy.id
        UNION ALL
        SELECT exclusion.permission_id, hierarchy.depth, FALSE AS allowed
          FROM hierarchy
          JOIN auth_role_permission_exclusions exclusion ON exclusion.role_id = hierarchy.id
    ), nearest AS (
        SELECT DISTINCT ON (permission_id) permission_id, allowed
          FROM decisions
         ORDER BY permission_id, depth, allowed
    )
    SELECT EXISTS (
        SELECT 1
          FROM auth_users user_row
          JOIN auth_user_owner_bindings binding
            ON binding.user_id = user_row.id
           AND binding.owner_id = $2
           AND binding.is_active
         WHERE user_row.id = $1
           AND user_row.status = 'active'
           AND EXISTS (
                SELECT 1
                  FROM nearest
                  JOIN auth_permissions permission ON permission.id = nearest.permission_id
                 WHERE nearest.allowed
                   AND permission.permission_code = $3
           )
    )
"#;

#[derive(Debug, FromRow)]
pub(super) struct SiteRow {
    pub(super) id: Uuid,
    pub(super) site_code: String,
    pub(super) site_name: String,
    pub(super) status: String,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(super) struct MappingRow {
    pub(super) id: Uuid,
    pub(super) site_id: Uuid,
    pub(super) owner_id: Uuid,
    pub(super) warehouse_id: Uuid,
    pub(super) status: String,
    pub(super) created_at: DateTime<Utc>,
    pub(super) disabled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
pub(super) struct PrinterRow {
    pub(super) id: Uuid,
    pub(super) site_id: Uuid,
    pub(super) site_code: String,
    pub(super) site_name: String,
    pub(super) printer_name: String,
    pub(super) printer_model: Option<String>,
    pub(super) connection_type: String,
    pub(super) status: String,
    pub(super) release_mode_override: Option<String>,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(super) struct TrayRow {
    pub(super) id: Uuid,
    pub(super) printer_id: Uuid,
    pub(super) tray_code: String,
    pub(super) paper_size: String,
    pub(super) paper_type: String,
    pub(super) enabled: bool,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(super) struct LeaseRow {
    pub(super) id: Uuid,
    pub(super) site_id: Uuid,
    pub(super) printer_id: Uuid,
    pub(super) printer_name: String,
    pub(super) connection_type: String,
    pub(super) holder_agent_id: Option<Uuid>,
    pub(super) lease_token: String,
    pub(super) release_mode: String,
    pub(super) busy_state: String,
    pub(super) status: String,
    pub(super) assigned_at: DateTime<Utc>,
    pub(super) acquired_at: Option<DateTime<Utc>>,
    pub(super) released_at: Option<DateTime<Utc>>,
    pub(super) release_reason: Option<String>,
}

impl From<SiteRow> for PrintSite {
    fn from(row: SiteRow) -> Self {
        Self {
            id: row.id,
            site_code: row.site_code,
            site_name: row.site_name,
            status: row.status,
            created_at: row.created_at,
        }
    }
}

impl From<MappingRow> for PrintSiteOwnerMapping {
    fn from(row: MappingRow) -> Self {
        Self {
            id: row.id,
            site_id: row.site_id,
            owner_id: row.owner_id,
            warehouse_id: row.warehouse_id,
            status: row.status,
            created_at: row.created_at,
            disabled_at: row.disabled_at,
        }
    }
}

impl From<TrayRow> for PrinterTray {
    fn from(row: TrayRow) -> Self {
        Self {
            id: row.id,
            printer_id: row.printer_id,
            tray_code: row.tray_code,
            paper_size: row.paper_size,
            paper_type: row.paper_type,
            enabled: row.enabled,
            created_at: row.created_at,
        }
    }
}

impl From<LeaseRow> for DeviceLease {
    fn from(row: LeaseRow) -> Self {
        Self {
            id: row.id,
            site_id: row.site_id,
            printer_id: row.printer_id,
            printer_name: row.printer_name,
            connection_type: row.connection_type,
            holder_agent_id: row.holder_agent_id,
            lease_token: row.lease_token,
            release_mode: row.release_mode,
            busy_state: row.busy_state,
            status: row.status,
            assigned_at: row.assigned_at,
            acquired_at: row.acquired_at,
            released_at: row.released_at,
            release_reason: row.release_reason,
        }
    }
}

pub(super) fn printer_from(row: PrinterRow, global_release_mode: &str) -> Printer {
    let effective_release_mode = row
        .release_mode_override
        .clone()
        .unwrap_or_else(|| global_release_mode.to_string());
    Printer {
        id: row.id,
        site_id: row.site_id,
        site_code: row.site_code,
        site_name: row.site_name,
        printer_name: row.printer_name,
        printer_model: row.printer_model,
        connection_type: row.connection_type,
        status: row.status,
        release_mode_override: row.release_mode_override,
        effective_release_mode,
        created_at: row.created_at,
    }
}

pub(super) const PRINTER_SELECT: &str = r#"
    SELECT printer.id, printer.site_id, site.site_code, site.site_name,
           printer.printer_name, printer.printer_model, printer.connection_type,
           printer.status, printer.release_mode_override, printer.created_at
      FROM h9_printers printer
      JOIN h9_print_sites site ON site.id = printer.site_id
"#;

fn context_has_permission(ctx: &AuthContext, permission: &str) -> bool {
    ctx.permissions.iter().any(|code| code == permission)
}

async fn user_has_owner_permission(
    pool: &PgPool,
    ctx: &AuthContext,
    owner_id: Uuid,
    permission: &str,
) -> Result<bool, PrintDeviceError> {
    if owner_id == ctx.owner_id {
        return Ok(context_has_permission(ctx, permission));
    }
    sqlx::query_scalar(USER_OWNER_PERMISSION_SQL)
        .bind(ctx.user_id)
        .bind(owner_id)
        .bind(permission)
        .fetch_one(pool)
        .await
        .map_err(map_db_error)
}

async fn user_has_owner_permission_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    owner_id: Uuid,
    permission: &str,
) -> Result<bool, PrintDeviceError> {
    if owner_id == ctx.owner_id {
        return Ok(context_has_permission(ctx, permission));
    }
    sqlx::query_scalar(USER_OWNER_PERMISSION_SQL)
        .bind(ctx.user_id)
        .bind(owner_id)
        .bind(permission)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

async fn active_site_owner_ids(
    pool: &PgPool,
    site_id: Uuid,
) -> Result<Vec<Uuid>, PrintDeviceError> {
    sqlx::query_scalar(
        "SELECT DISTINCT owner_id FROM h9_print_site_owner_mappings WHERE site_id = $1 AND status = 'active' ORDER BY owner_id",
    )
    .bind(site_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)
}

async fn active_site_owner_ids_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    site_id: Uuid,
) -> Result<Vec<Uuid>, PrintDeviceError> {
    sqlx::query_scalar(
        "SELECT DISTINCT owner_id FROM h9_print_site_owner_mappings WHERE site_id = $1 AND status = 'active' ORDER BY owner_id",
    )
    .bind(site_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub(super) async fn require_site_permission(
    pool: &PgPool,
    ctx: &AuthContext,
    site_id: Uuid,
    permission: &str,
) -> Result<Vec<Uuid>, PrintDeviceError> {
    let created_by: Uuid =
        sqlx::query_scalar("SELECT created_by FROM h9_print_sites WHERE id = $1")
            .bind(site_id)
            .fetch_optional(pool)
            .await
            .map_err(map_db_error)?
            .ok_or(PrintDeviceError::SiteNotFound)?;
    let owner_ids = active_site_owner_ids(pool, site_id).await?;
    if owner_ids.is_empty() {
        if created_by == ctx.user_id && context_has_permission(ctx, permission) {
            return Ok(owner_ids);
        }
        return Err(PrintDeviceError::OwnerPermissionRequired);
    }
    for owner_id in &owner_ids {
        if !user_has_owner_permission(pool, ctx, *owner_id, permission).await? {
            return Err(PrintDeviceError::OwnerPermissionRequired);
        }
    }
    Ok(owner_ids)
}

pub(super) async fn require_site_permission_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    site_id: Uuid,
    permission: &str,
) -> Result<Vec<Uuid>, PrintDeviceError> {
    let created_by: Uuid =
        sqlx::query_scalar("SELECT created_by FROM h9_print_sites WHERE id = $1 FOR UPDATE")
            .bind(site_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?
            .ok_or(PrintDeviceError::SiteNotFound)?;
    let owner_ids = active_site_owner_ids_in_tx(tx, site_id).await?;
    if owner_ids.is_empty() {
        if created_by == ctx.user_id && context_has_permission(ctx, permission) {
            return Ok(owner_ids);
        }
        return Err(PrintDeviceError::OwnerPermissionRequired);
    }
    for owner_id in &owner_ids {
        if !user_has_owner_permission_in_tx(tx, ctx, *owner_id, permission).await? {
            return Err(PrintDeviceError::OwnerPermissionRequired);
        }
    }
    Ok(owner_ids)
}

pub(super) async fn require_site_permission_with_target_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    site_id: Uuid,
    target_owner_id: Uuid,
    permission: &str,
) -> Result<Vec<Uuid>, PrintDeviceError> {
    let mut owner_ids = require_site_permission_in_tx(tx, ctx, site_id, permission).await?;
    if !owner_ids.contains(&target_owner_id) {
        if !user_has_owner_permission_in_tx(tx, ctx, target_owner_id, permission).await? {
            return Err(PrintDeviceError::OwnerPermissionRequired);
        }
        owner_ids.push(target_owner_id);
        owner_ids.sort_unstable();
    }
    Ok(owner_ids)
}

pub(super) async fn require_printer_permission(
    pool: &PgPool,
    ctx: &AuthContext,
    printer_id: Uuid,
    permission: &str,
) -> Result<(Uuid, Vec<Uuid>), PrintDeviceError> {
    let site_id: Uuid = sqlx::query_scalar("SELECT site_id FROM h9_printers WHERE id = $1")
        .bind(printer_id)
        .fetch_optional(pool)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintDeviceError::PrinterNotFound)?;
    let owner_ids = require_site_permission(pool, ctx, site_id, permission).await?;
    Ok((site_id, owner_ids))
}

pub(super) async fn require_printer_permission_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    printer_id: Uuid,
    permission: &str,
) -> Result<(Uuid, Vec<Uuid>), PrintDeviceError> {
    let site_id: Uuid =
        sqlx::query_scalar("SELECT site_id FROM h9_printers WHERE id = $1 FOR UPDATE")
            .bind(printer_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?
            .ok_or(PrintDeviceError::PrinterNotFound)?;
    let owner_ids = require_site_permission_in_tx(tx, ctx, site_id, permission).await?;
    Ok((site_id, owner_ids))
}

pub(super) async fn require_lease_permission_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    lease_id: Uuid,
    permission: &str,
) -> Result<Vec<Uuid>, PrintDeviceError> {
    let site_id: Uuid = sqlx::query_scalar("SELECT site_id FROM h9_device_leases WHERE id = $1")
        .bind(lease_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintDeviceError::LeaseNotFound)?;
    require_site_permission_in_tx(tx, ctx, site_id, permission).await
}

pub(super) async fn load_printer_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    printer_id: Uuid,
) -> Result<Printer, PrintDeviceError> {
    let global = global_release_mode_in_tx(tx).await?;
    let row = sqlx::query_as::<_, PrinterRow>(&format!("{PRINTER_SELECT} WHERE printer.id = $1"))
        .bind(printer_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintDeviceError::PrinterNotFound)?;
    Ok(printer_from(row, &global))
}

pub(super) const GLOBAL_RELEASE_MODE_SQL: &str = r#"
    SELECT params->>'release_mode'
      FROM system_dictionary_items
     WHERE dict_code = 'h9_device_lease_release'
       AND item_code = 'default'
       AND owner_id IS NULL
       AND enabled
"#;

pub(super) async fn global_release_mode(pool: &PgPool) -> Result<String, PrintDeviceError> {
    let mode: Option<Option<String>> = sqlx::query_scalar(GLOBAL_RELEASE_MODE_SQL)
        .fetch_optional(pool)
        .await
        .map_err(map_db_error)?;
    Ok(mode
        .flatten()
        .unwrap_or_else(|| DEFAULT_RELEASE_MODE.to_string()))
}

pub(super) async fn global_release_mode_in_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<String, PrintDeviceError> {
    let mode: Option<Option<String>> = sqlx::query_scalar(GLOBAL_RELEASE_MODE_SQL)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(mode
        .flatten()
        .unwrap_or_else(|| DEFAULT_RELEASE_MODE.to_string()))
}

pub(super) fn ensure_idempotency_key(idempotency_key: &str) -> Result<(), PrintDeviceError> {
    if idempotency_key.trim().is_empty() {
        Err(PrintDeviceError::InvalidRequest)
    } else {
        Ok(())
    }
}

pub(super) fn map_unique_violation(
    error: sqlx::Error,
    conflict: PrintDeviceError,
) -> PrintDeviceError {
    if error
        .as_database_error()
        .is_some_and(|db| db.is_unique_violation())
    {
        conflict
    } else {
        map_db_error(error)
    }
}

pub(super) async fn append_device_audit<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    resource: &T,
    now: DateTime<Utc>,
) -> Result<(), PrintDeviceError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H9",
        resource_type,
        resource_id.to_string(),
        Some(AuditDiff::compute(
            Value::Null,
            serde_json::to_value(resource).map_err(serialize_error)?,
        )),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintDeviceError::Audit(format!("{error:?}")))?;
    Ok(())
}

pub(super) async fn append_device_audits<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    owner_ids: &[Uuid],
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    resource: &T,
    now: DateTime<Utc>,
) -> Result<(), PrintDeviceError> {
    let owners = if owner_ids.is_empty() {
        BTreeSet::from([ctx.owner_id])
    } else {
        owner_ids.iter().copied().collect()
    };
    for owner_id in owners {
        let mut owner_ctx = ctx.clone();
        owner_ctx.owner_id = owner_id;
        append_device_audit(
            tx,
            &owner_ctx,
            action,
            resource_type,
            resource_id,
            resource,
            now,
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), PrintDeviceError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(owner_id.to_string())
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

pub(super) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, PrintDeviceError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(PrintDeviceError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(serialize_error)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    path: &str,
    resource_type: &str,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), PrintDeviceError> {
    let response_body = serde_json::to_value(response).map_err(serialize_error)?;
    let resource_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(resource_type)
        .to_string();
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id,
            expires_at, created_at
        )
        VALUES (
            $1, $2, $3, $4, 'POST', $5,
            200, $6, $7, $8, $9, $10
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
    .map_err(map_db_error)
}

pub(super) fn json_request_hash<T: Serialize>(value: &T) -> Result<String, PrintDeviceError> {
    let bytes = serde_json::to_vec(value).map_err(serialize_error)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub(super) fn serialize_error(error: serde_json::Error) -> PrintDeviceError {
    PrintDeviceError::Serialize(error.to_string())
}

pub(super) fn map_db_error(error: sqlx::Error) -> PrintDeviceError {
    PrintDeviceError::Database(error.to_string())
}
