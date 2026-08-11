//! US-H9-011 device leases: single active lease per printer and manual release.

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{validate_release_device_lease, DeviceLease, ReleaseDeviceLeaseRequest};

use crate::operation_context::OperationContext as AuthContext;

use super::support::*;
use super::{
    IdempotentMutation, PrintDeviceError, PrintDeviceService, DEVICE_LEASE_RELEASE_PERMISSION,
    PRINT_DEVICE_READ_PERMISSION,
};

impl PrintDeviceService {
    /// Lists device leases with printer context, newest assignment first.
    pub async fn list_leases(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<DeviceLease>, i64), PrintDeviceError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        // 站点可见性必须先于分页落地：先算出调用方可见的站点集合，count(*) 与主查询
        // 共用同一 `site_id = ANY($1)` 过滤，保证 total 与可见行一一对应、分页切片连续。
        // （可见性判定只依赖站点本身，无需逐租约行判断。）
        let visible_sites: Vec<Uuid> = {
            let site_ids: Vec<Uuid> =
                sqlx::query_scalar("SELECT DISTINCT site_id FROM h9_device_leases")
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_db_error)?;
            let mut visible = Vec::with_capacity(site_ids.len());
            for site_id in site_ids {
                match require_site_permission(
                    &self.pool,
                    ctx,
                    site_id,
                    PRINT_DEVICE_READ_PERMISSION,
                )
                .await
                {
                    Ok(_) => visible.push(site_id),
                    Err(PrintDeviceError::OwnerPermissionRequired) => {}
                    Err(error) => return Err(error),
                }
            }
            visible
        };
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)
              FROM h9_device_leases lease
              JOIN h9_printers printer ON printer.id = lease.printer_id
             WHERE lease.site_id = ANY($1)
            "#,
        )
        .bind(&visible_sites)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, LeaseRow>(
            r#"
            SELECT lease.id, lease.site_id, lease.printer_id, printer.printer_name,
                   printer.connection_type, lease.holder_agent_id, lease.lease_token,
                   lease.release_mode, lease.busy_state, lease.status, lease.assigned_at,
                   lease.acquired_at, lease.released_at, lease.release_reason
              FROM h9_device_leases lease
              JOIN h9_printers printer ON printer.id = lease.printer_id
             WHERE lease.site_id = ANY($1)
             ORDER BY lease.assigned_at DESC, lease.id
             LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&visible_sites)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let mut data = Vec::with_capacity(rows.len());
        for row in rows {
            match require_site_permission(
                &self.pool,
                ctx,
                row.site_id,
                PRINT_DEVICE_READ_PERMISSION,
            )
            .await
            {
                Ok(_) => data.push(DeviceLease::from(row)),
                Err(PrintDeviceError::OwnerPermissionRequired) => {}
                Err(error) => return Err(error),
            }
        }
        Ok((data, total))
    }

    /// Manually releases one active lease.
    ///
    /// AC7：专用权限 + 原因必填 + 二次确认；printing/result_unknown/reconciling 是
    /// 任何人都不可覆盖的硬安全条件，人工权限只能覆盖 manual_only 模式本身。
    pub async fn release_lease(
        &self,
        ctx: &AuthContext,
        lease_id: Uuid,
        request: ReleaseDeviceLeaseRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<DeviceLease>, PrintDeviceError> {
        if lease_id.is_nil() {
            return Err(PrintDeviceError::InvalidRequest);
        }
        validate_release_device_lease(&request)?;
        if !ctx
            .permissions
            .iter()
            .any(|permission| permission == DEVICE_LEASE_RELEASE_PERMISSION)
        {
            return Err(PrintDeviceError::ReleasePermissionRequired);
        }
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&json!({
            "lease_id": lease_id,
            "reason": request.reason.trim(),
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let audit_owner_ids =
            require_lease_permission_in_tx(&mut tx, ctx, lease_id, DEVICE_LEASE_RELEASE_PERMISSION)
                .await?;
        if let Some(lease) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: lease,
                replayed: true,
            });
        }
        let current = sqlx::query_as::<_, LeaseRow>(
            r#"
            SELECT lease.id, lease.site_id, lease.printer_id, printer.printer_name,
                   printer.connection_type, lease.holder_agent_id, lease.lease_token,
                   lease.release_mode, lease.busy_state, lease.status, lease.assigned_at,
                   lease.acquired_at, lease.released_at, lease.release_reason
              FROM h9_device_leases lease
              JOIN h9_printers printer ON printer.id = lease.printer_id
             WHERE lease.id = $1
             FOR UPDATE OF lease
            "#,
        )
        .bind(lease_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintDeviceError::LeaseNotFound)?;
        if current.status != "active" {
            return Err(PrintDeviceError::LeaseAlreadyReleased);
        }
        if current.busy_state != "idle" {
            return Err(PrintDeviceError::LeaseBusy(current.busy_state));
        }
        sqlx::query(
            r#"
            UPDATE h9_device_leases
               SET status = 'released', released_at = $2, released_by = $3, release_reason = $4
             WHERE id = $1
            "#,
        )
        .bind(lease_id)
        .bind(now)
        .bind(ctx.user_id)
        .bind(request.reason.trim())
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let mut lease = DeviceLease::from(current);
        lease.status = "released".to_string();
        lease.released_at = Some(now);
        lease.release_reason = Some(request.reason.trim().to_string());
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-devices/leases/{lease_id}/release"),
            "h9_device_lease",
            &lease,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "release_device_lease",
            "device_lease",
            lease.id,
            &lease,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: lease,
            replayed: false,
        })
    }
}

/// Resolves the release-mode snapshot for a new lease of one printer.
///
/// US-H9-012 的租约签发使用同一函数：覆盖优先，其次全局默认。
pub async fn resolve_lease_release_mode(
    pool: &PgPool,
    printer_id: Uuid,
) -> Result<String, PrintDeviceError> {
    let override_mode: Option<Option<String>> =
        sqlx::query_scalar("SELECT release_mode_override FROM h9_printers WHERE id = $1")
            .bind(printer_id)
            .fetch_optional(pool)
            .await
            .map_err(map_db_error)?;
    match override_mode {
        None => Err(PrintDeviceError::PrinterNotFound),
        Some(Some(mode)) => Ok(mode),
        Some(None) => global_release_mode(pool).await,
    }
}
