//! US-H9-011 printers, trays and controlled test prints.

use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::{
    validate_create_printer, validate_create_printer_tray, validate_paper_capability,
    CreatePrinterRequest, CreatePrinterTrayRequest, Printer, PrinterListResponse, PrinterTestPrint,
    PrinterTray, PrinterTrayListResponse, TestPrintRequest, UpdatePrinterRequest,
    UpdatePrinterTrayRequest,
};

use crate::auth::AuthContext;

use super::support::*;
use super::{
    IdempotentMutation, PrintDeviceError, PrintDeviceService, PRINT_DEVICE_READ_PERMISSION,
    PRINT_DEVICE_WRITE_PERMISSION, TEST_PRINT_DISPATCH_NOTE,
};

impl PrintDeviceService {
    /// Lists all printers with site info and the effective lease release mode.
    pub async fn list_printers(
        &self,
        ctx: &AuthContext,
    ) -> Result<PrinterListResponse, PrintDeviceError> {
        let global = global_release_mode(&self.pool).await?;
        let rows = sqlx::query_as::<_, PrinterRow>(&format!(
            "{PRINTER_SELECT} ORDER BY site.site_code, printer.printer_name"
        ))
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
                Ok(_) => data.push(printer_from(row, &global)),
                Err(PrintDeviceError::OwnerPermissionRequired) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(PrinterListResponse { data })
    }

    /// Creates one printer that belongs to exactly one physical print site.
    pub async fn create_printer(
        &self,
        ctx: &AuthContext,
        request: CreatePrinterRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<Printer>, PrintDeviceError> {
        validate_create_printer(&request)?;
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&request)?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let audit_owner_ids = require_site_permission_in_tx(
            &mut tx,
            ctx,
            request.site_id,
            PRINT_DEVICE_WRITE_PERMISSION,
        )
        .await?;
        if let Some(printer) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: printer,
                replayed: true,
            });
        }
        let printer_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO h9_printers (
                id, site_id, printer_name, printer_model, connection_type,
                status, release_mode_override, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'active', $6, $7, $8, $8)
            "#,
        )
        .bind(printer_id)
        .bind(request.site_id)
        .bind(request.printer_name.trim())
        .bind(request.printer_model.as_deref().map(str::trim))
        .bind(&request.connection_type)
        .bind(&request.release_mode_override)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|error| map_unique_violation(error, PrintDeviceError::PrinterNameConflict))?;
        let printer = load_printer_in_tx(&mut tx, printer_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "/api/v1/print-devices/printers",
            "h9_printer",
            &printer,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "create_printer",
            "printer",
            printer.id,
            &printer,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: printer,
            replayed: false,
        })
    }

    /// Updates one printer's status or per-printer release-mode override.
    pub async fn update_printer(
        &self,
        ctx: &AuthContext,
        printer_id: Uuid,
        request: UpdatePrinterRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<Printer>, PrintDeviceError> {
        if printer_id.is_nil()
            || (request.status.is_none() && request.release_mode_override.is_none())
        {
            return Err(PrintDeviceError::InvalidRequest);
        }
        if let Some(status) = &request.status {
            if !matches!(status.as_str(), "active" | "disabled") {
                return Err(PrintDeviceError::InvalidRequest);
            }
        }
        if let Some(mode) = &request.release_mode_override {
            if !matches!(mode.as_str(), "manual_only" | "safe_auto" | "inherit") {
                return Err(PrintDeviceError::InvalidRequest);
            }
        }
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&json!({
            "printer_id": printer_id,
            "status": request.status,
            "release_mode_override": request.release_mode_override,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let (_, audit_owner_ids) = require_printer_permission_in_tx(
            &mut tx,
            ctx,
            printer_id,
            PRINT_DEVICE_WRITE_PERMISSION,
        )
        .await?;
        if let Some(printer) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: printer,
                replayed: true,
            });
        }
        if let Some(status) = &request.status {
            sqlx::query("UPDATE h9_printers SET status = $2, updated_at = $3 WHERE id = $1")
                .bind(printer_id)
                .bind(status)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
        }
        if let Some(mode) = &request.release_mode_override {
            let stored = if mode == "inherit" {
                None
            } else {
                Some(mode.clone())
            };
            sqlx::query(
                "UPDATE h9_printers SET release_mode_override = $2, updated_at = $3 WHERE id = $1",
            )
            .bind(printer_id)
            .bind(stored)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let printer = load_printer_in_tx(&mut tx, printer_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-devices/printers/{printer_id}"),
            "h9_printer",
            &printer,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "update_printer",
            "printer",
            printer.id,
            &printer,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: printer,
            replayed: false,
        })
    }

    /// Lists one printer's trays.
    pub async fn list_printer_trays(
        &self,
        ctx: &AuthContext,
        printer_id: Uuid,
    ) -> Result<PrinterTrayListResponse, PrintDeviceError> {
        require_printer_permission(&self.pool, ctx, printer_id, PRINT_DEVICE_READ_PERMISSION)
            .await?;
        let rows = sqlx::query_as::<_, TrayRow>(
            r#"
            SELECT id, printer_id, tray_code, paper_size, paper_type, enabled, created_at
              FROM h9_printer_trays
             WHERE printer_id = $1
             ORDER BY tray_code
            "#,
        )
        .bind(printer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(PrinterTrayListResponse {
            data: rows.into_iter().map(PrinterTray::from).collect(),
        })
    }

    /// Creates one tray under a printer; the composite FK keeps it inside the site.
    pub async fn create_printer_tray(
        &self,
        ctx: &AuthContext,
        printer_id: Uuid,
        request: CreatePrinterTrayRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrinterTray>, PrintDeviceError> {
        if printer_id.is_nil() {
            return Err(PrintDeviceError::InvalidRequest);
        }
        validate_create_printer_tray(&request)?;
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&json!({
            "printer_id": printer_id,
            "request": request,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let (site_id, audit_owner_ids) = require_printer_permission_in_tx(
            &mut tx,
            ctx,
            printer_id,
            PRINT_DEVICE_WRITE_PERMISSION,
        )
        .await?;
        if let Some(tray) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: tray,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, TrayRow>(
            r#"
            INSERT INTO h9_printer_trays (
                id, site_id, printer_id, tray_code, paper_size, paper_type,
                enabled, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, TRUE, $7, $8, $8)
            RETURNING id, printer_id, tray_code, paper_size, paper_type, enabled, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(site_id)
        .bind(printer_id)
        .bind(request.tray_code.trim())
        .bind(request.paper_size.trim())
        .bind(request.paper_type.trim())
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| map_unique_violation(error, PrintDeviceError::TrayConflict))?;
        let tray = PrinterTray::from(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-devices/printers/{printer_id}/trays"),
            "h9_printer_tray",
            &tray,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "create_printer_tray",
            "printer_tray",
            tray.id,
            &tray,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: tray,
            replayed: false,
        })
    }

    /// Updates one tray's paper capability or enabled flag.
    pub async fn update_printer_tray(
        &self,
        ctx: &AuthContext,
        printer_id: Uuid,
        tray_id: Uuid,
        request: UpdatePrinterTrayRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrinterTray>, PrintDeviceError> {
        if printer_id.is_nil()
            || tray_id.is_nil()
            || (request.paper_size.is_none()
                && request.paper_type.is_none()
                && request.enabled.is_none())
        {
            return Err(PrintDeviceError::InvalidRequest);
        }
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&json!({
            "printer_id": printer_id,
            "tray_id": tray_id,
            "request": request,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let (_, audit_owner_ids) = require_printer_permission_in_tx(
            &mut tx,
            ctx,
            printer_id,
            PRINT_DEVICE_WRITE_PERMISSION,
        )
        .await?;
        if let Some(tray) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: tray,
                replayed: true,
            });
        }
        let current = sqlx::query_as::<_, TrayRow>(
            r#"
            SELECT id, printer_id, tray_code, paper_size, paper_type, enabled, created_at
              FROM h9_printer_trays
             WHERE printer_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(printer_id)
        .bind(tray_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintDeviceError::TrayNotFound)?;
        let paper_size = request
            .paper_size
            .as_deref()
            .map(str::trim)
            .unwrap_or(&current.paper_size)
            .to_string();
        let paper_type = request
            .paper_type
            .as_deref()
            .map(str::trim)
            .unwrap_or(&current.paper_type)
            .to_string();
        validate_paper_capability(&paper_size, &paper_type)?;
        let enabled = request.enabled.unwrap_or(current.enabled);
        let row = sqlx::query_as::<_, TrayRow>(
            r#"
            UPDATE h9_printer_trays
               SET paper_size = $3, paper_type = $4, enabled = $5, updated_at = $6
             WHERE printer_id = $1 AND id = $2
            RETURNING id, printer_id, tray_code, paper_size, paper_type, enabled, created_at
            "#,
        )
        .bind(printer_id)
        .bind(tray_id)
        .bind(&paper_size)
        .bind(&paper_type)
        .bind(enabled)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let tray = PrinterTray::from(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-devices/printers/{printer_id}/trays/{tray_id}"),
            "h9_printer_tray",
            &tray,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "update_printer_tray",
            "printer_tray",
            tray.id,
            &tray,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: tray,
            replayed: false,
        })
    }

    /// Dispatches one controlled test print for a printer + tray and records it.
    ///
    /// 真实物理打印机在本机不可达：记录"已下发测试指令"，回执字段等待
    /// Print Agent（US-H9-012）或 S4 硬件验收登记，不伪造成功结果。
    pub async fn test_print(
        &self,
        ctx: &AuthContext,
        printer_id: Uuid,
        request: TestPrintRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrinterTestPrint>, PrintDeviceError> {
        if printer_id.is_nil() || request.tray_id.is_nil() {
            return Err(PrintDeviceError::InvalidRequest);
        }
        ensure_idempotency_key(idempotency_key)?;
        let request_hash = json_request_hash(&json!({
            "printer_id": printer_id,
            "tray_id": request.tray_id,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        let (_, audit_owner_ids) = require_printer_permission_in_tx(
            &mut tx,
            ctx,
            printer_id,
            PRINT_DEVICE_WRITE_PERMISSION,
        )
        .await?;
        if let Some(record) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: record,
                replayed: true,
            });
        }
        let printer: Option<(Uuid, String)> =
            sqlx::query_as("SELECT site_id, status FROM h9_printers WHERE id = $1")
                .bind(printer_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        let Some((site_id, printer_status)) = printer else {
            return Err(PrintDeviceError::PrinterNotFound);
        };
        if printer_status != "active" {
            return Err(PrintDeviceError::PrinterDisabled);
        }
        let tray_enabled: Option<bool> = sqlx::query_scalar(
            "SELECT enabled FROM h9_printer_trays WHERE printer_id = $1 AND id = $2",
        )
        .bind(printer_id)
        .bind(request.tray_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        match tray_enabled {
            None => return Err(PrintDeviceError::TrayNotFound),
            Some(false) => return Err(PrintDeviceError::TrayDisabled),
            Some(true) => {}
        }
        let record_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO h9_printer_test_prints (
                id, site_id, printer_id, tray_id, result, result_note,
                requested_by, requested_at
            )
            VALUES ($1, $2, $3, $4, 'dispatched', $5, $6, $7)
            "#,
        )
        .bind(record_id)
        .bind(site_id)
        .bind(printer_id)
        .bind(request.tray_id)
        .bind(TEST_PRINT_DISPATCH_NOTE)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let record = PrinterTestPrint {
            id: record_id,
            printer_id,
            tray_id: request.tray_id,
            result: "dispatched".to_string(),
            result_note: Some(TEST_PRINT_DISPATCH_NOTE.to_string()),
            requested_at: now,
            result_at: None,
        };
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-devices/printers/{printer_id}/test-print"),
            "h9_printer_test_print",
            &record,
            now,
        )
        .await?;
        append_device_audits(
            &mut tx,
            ctx,
            &audit_owner_ids,
            "test_print_printer",
            "printer_test_print",
            record.id,
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
