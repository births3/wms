//! US-H9-011 physical print sites and explicit owner + warehouse mappings.

use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::{
    validate_create_print_site, CreatePrintSiteRequest, CreateSiteOwnerMappingRequest, PrintSite,
    PrintSiteListResponse, PrintSiteOwnerMapping, PrintSiteOwnerMappingListResponse,
};

use crate::operation_context::OperationContext as AuthContext;

use super::support::*;
use super::{
    IdempotentMutation, PrintDeviceError, PrintDeviceService, PRINT_DEVICE_READ_PERMISSION,
    PRINT_DEVICE_WRITE_PERMISSION,
};

impl PrintDeviceService {
    /// Lists all physical print sites.
    pub async fn list_sites(
        &self,
        ctx: &AuthContext,
    ) -> Result<PrintSiteListResponse, PrintDeviceError> {
        let rows = sqlx::query_as::<_, SiteRow>(
            r#"
            SELECT id, site_code, site_name, status, created_at
              FROM h9_print_sites
             ORDER BY site_code
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let mut data = Vec::with_capacity(rows.len());
        for row in rows {
            match require_site_permission(&self.pool, ctx, row.id, PRINT_DEVICE_READ_PERMISSION)
                .await
            {
                Ok(_) => data.push(PrintSite::from(row)),
                Err(PrintDeviceError::OwnerPermissionRequired) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(PrintSiteListResponse { data })
    }

    /// Creates one physical print site.
    pub async fn create_site(
        &self,
        ctx: &AuthContext,
        request: CreatePrintSiteRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSite>, PrintDeviceError> {
        validate_create_print_site(&request)?;
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&request)?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(site) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: site,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, SiteRow>(
            r#"
            INSERT INTO h9_print_sites (id, site_code, site_name, status, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, 'active', $4, $5, $5)
            RETURNING id, site_code, site_name, status, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(request.site_code.trim())
        .bind(request.site_name.trim())
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| map_unique_violation(error, PrintDeviceError::SiteCodeConflict))?;
        let site = PrintSite::from(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "/api/v1/print-devices/sites",
            "h9_print_site",
            &site,
            now,
        )
        .await?;
        append_device_audit(
            &mut tx,
            ctx,
            "create_print_site",
            "print_site",
            site.id,
            &site,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: site,
            replayed: false,
        })
    }

    /// Lists one site's owner + warehouse mappings, including soft-disabled rows.
    pub async fn list_site_owner_mappings(
        &self,
        ctx: &AuthContext,
        site_id: Uuid,
    ) -> Result<PrintSiteOwnerMappingListResponse, PrintDeviceError> {
        require_site_permission(&self.pool, ctx, site_id, PRINT_DEVICE_READ_PERMISSION).await?;
        let rows = sqlx::query_as::<_, MappingRow>(
            r#"
            SELECT id, site_id, owner_id, warehouse_id, status, created_at, disabled_at
              FROM h9_print_site_owner_mappings
             WHERE site_id = $1
             ORDER BY status, created_at DESC
            "#,
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(PrintSiteOwnerMappingListResponse {
            data: rows.into_iter().map(PrintSiteOwnerMapping::from).collect(),
        })
    }

    /// Maps one owner + warehouse pair onto a physical print site.
    pub async fn create_site_owner_mapping(
        &self,
        ctx: &AuthContext,
        site_id: Uuid,
        request: CreateSiteOwnerMappingRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSiteOwnerMapping>, PrintDeviceError> {
        if site_id.is_nil() || request.owner_id.is_nil() || request.warehouse_id.is_nil() {
            return Err(PrintDeviceError::InvalidRequest);
        }
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&json!({
            "site_id": site_id,
            "owner_id": request.owner_id,
            "warehouse_id": request.warehouse_id,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let audit_owner_ids = require_site_permission_with_target_in_tx(
            &mut tx,
            ctx,
            site_id,
            request.owner_id,
            PRINT_DEVICE_WRITE_PERMISSION,
        )
        .await?;
        if let Some(mapping) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: mapping,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, MappingRow>(
            r#"
            INSERT INTO h9_print_site_owner_mappings (
                id, site_id, owner_id, warehouse_id, status, created_by, created_at
            )
            VALUES ($1, $2, $3, $4, 'active', $5, $6)
            RETURNING id, site_id, owner_id, warehouse_id, status, created_at, disabled_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(site_id)
        .bind(request.owner_id)
        .bind(request.warehouse_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| map_unique_violation(error, PrintDeviceError::MappingConflict))?;
        let mapping = PrintSiteOwnerMapping::from(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-devices/sites/{site_id}/owner-mappings"),
            "h9_print_site_owner_mapping",
            &mapping,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "map_print_site_owner",
            "print_site_owner_mapping",
            mapping.id,
            &mapping,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: mapping,
            replayed: false,
        })
    }

    /// Soft-disables one site owner mapping.
    pub async fn disable_site_owner_mapping(
        &self,
        ctx: &AuthContext,
        site_id: Uuid,
        mapping_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSiteOwnerMapping>, PrintDeviceError> {
        if site_id.is_nil() || mapping_id.is_nil() {
            return Err(PrintDeviceError::InvalidRequest);
        }
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&json!({
            "site_id": site_id,
            "mapping_id": mapping_id,
            "action": "disable",
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let audit_owner_ids =
            require_site_permission_in_tx(&mut tx, ctx, site_id, PRINT_DEVICE_WRITE_PERMISSION)
                .await?;
        if let Some(mapping) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: mapping,
                replayed: true,
            });
        }
        let current = sqlx::query_as::<_, MappingRow>(
            r#"
            SELECT id, site_id, owner_id, warehouse_id, status, created_at, disabled_at
              FROM h9_print_site_owner_mappings
             WHERE site_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(site_id)
        .bind(mapping_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintDeviceError::MappingNotFound)?;
        if current.status != "active" {
            return Err(PrintDeviceError::MappingAlreadyDisabled);
        }
        let row = sqlx::query_as::<_, MappingRow>(
            r#"
            UPDATE h9_print_site_owner_mappings
               SET status = 'disabled', disabled_by = $3, disabled_at = $4
             WHERE site_id = $1 AND id = $2
            RETURNING id, site_id, owner_id, warehouse_id, status, created_at, disabled_at
            "#,
        )
        .bind(site_id)
        .bind(mapping_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let mapping = PrintSiteOwnerMapping::from(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-devices/sites/{site_id}/owner-mappings/{mapping_id}/disable"),
            "h9_print_site_owner_mapping",
            &mapping,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "disable_print_site_owner_mapping",
            "print_site_owner_mapping",
            mapping.id,
            &mapping,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: mapping,
            replayed: false,
        })
    }
}
