use crate::{
    audit,
    auth::PortalAuth,
    models::{CreateExportRequest, ExportJob},
    report_download_file_name, resolve_storage_key, PortalError, PortalState,
};
use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use sqlx::Row;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

pub const MAX_EXPORT_FILES: i64 = 200;
pub const MAX_EXPORT_BYTES: i64 = 2 * 1024 * 1024 * 1024;
const EXPORT_RETENTION_DAYS: i64 = 7;

#[derive(Clone, Debug)]
struct ManifestRow {
    order_no: String,
    address_name: String,
    product_code: String,
    product_name: String,
    batch_no: String,
    report_version_id: Option<Uuid>,
    version_number: Option<i32>,
    file_name: Option<String>,
    storage_key: Option<String>,
    status: String,
    copy_available: bool,
}

pub async fn create_export(
    State(state): State<PortalState>,
    auth: PortalAuth,
    Json(request): Json<CreateExportRequest>,
) -> Result<Json<ExportJob>, PortalError> {
    if request.order_ids.is_empty() {
        return Err(PortalError::Validation("至少选择一个订单".to_string()));
    }
    if request.include_history && !auth.can_view_report_history {
        return Err(PortalError::Forbidden);
    }
    let order_ids = request
        .order_ids
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let authorized_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM portal_orders o
         WHERE o.id = ANY($1)
           AND o.customer_id = $2
           AND o.status IN ('shipped', 'signed')
           AND (
               $3 = 'customer_admin'
               OR EXISTS (
                   SELECT 1 FROM portal_user_addresses ua
                   WHERE ua.address_id = o.delivery_address_id AND ua.user_id = $4
               )
           )",
    )
    .bind(order_ids.as_slice())
    .bind(auth.customer_id)
    .bind(&auth.role)
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await?;
    if authorized_count != order_ids.len() as i64 {
        return Err(PortalError::Forbidden);
    }
    let (file_count, total_size) = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(*), COALESCE(SUM(file.customer_copy_size), 0)::BIGINT
         FROM (
             SELECT DISTINCT r.id, r.customer_copy_size
             FROM portal_orders o
             JOIN portal_order_lines l ON l.order_id = o.id
             JOIN portal_report_versions r
               ON r.product_id = l.product_id
              AND r.batch_no = l.batch_no
              AND r.customer_copy_status = 'available'
              AND ($2 OR r.is_current)
             WHERE o.id = ANY($1)
         ) AS file",
    )
    .bind(order_ids.as_slice())
    .bind(request.include_history)
    .fetch_one(&state.pool)
    .await?;
    if file_count > MAX_EXPORT_FILES || total_size > MAX_EXPORT_BYTES {
        return Err(PortalError::Validation(format!(
            "单次导出最多 {MAX_EXPORT_FILES} 份且不超过 2GB，请拆分任务"
        )));
    }
    let job_id = Uuid::new_v4();
    let mut transaction = state.pool.begin().await?;
    let job = sqlx::query_as::<_, ExportJob>(
        "INSERT INTO portal_export_jobs (
            id, customer_id, created_by, include_history, status,
            requested_order_count
         )
         VALUES ($1, $2, $3, $4, 'queued', $5)
         RETURNING id, include_history, status, requested_order_count,
                   report_file_count, missing_count, total_size, result_file_name,
                   last_error, expires_at, created_at, finished_at",
    )
    .bind(job_id)
    .bind(auth.customer_id)
    .bind(auth.user_id)
    .bind(request.include_history)
    .bind(order_ids.len() as i32)
    .fetch_one(&mut *transaction)
    .await?;
    for order_id in order_ids {
        sqlx::query(
            "INSERT INTO portal_export_job_orders (export_job_id, order_id)
             VALUES ($1, $2)",
        )
        .bind(job_id)
        .bind(order_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    audit(
        &state.pool,
        Some(auth.user_id),
        Some(auth.customer_id),
        "create",
        "export",
        &job_id.to_string(),
        serde_json::json!({
            "order_count": job.requested_order_count,
            "include_history": request.include_history
        }),
    )
    .await?;
    Ok(Json(job))
}

pub async fn list_exports(
    State(state): State<PortalState>,
    auth: PortalAuth,
) -> Result<Json<Vec<ExportJob>>, PortalError> {
    let jobs = sqlx::query_as::<_, ExportJob>(
        "SELECT id, include_history, status, requested_order_count,
                report_file_count, missing_count, total_size, result_file_name,
                last_error, expires_at, created_at, finished_at
         FROM portal_export_jobs
         WHERE customer_id = $1 AND created_by = $2
         ORDER BY created_at DESC",
    )
    .bind(auth.customer_id)
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(jobs))
}

pub fn spawn_export_worker(state: PortalState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match process_next_export(&state).await {
                Ok(true) => {}
                Ok(false) => tokio::time::sleep(std::time::Duration::from_secs(2)).await,
                Err(error) => {
                    tracing::error!(error = %error, "customer portal export worker failed");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    })
}

pub async fn process_next_export(state: &PortalState) -> Result<bool, PortalError> {
    let mut transaction = state.pool.begin().await?;
    let job = sqlx::query_as::<_, (Uuid, Uuid, Uuid, bool)>(
        "SELECT id, customer_id, created_by, include_history
         FROM portal_export_jobs
         WHERE status = 'queued'
         ORDER BY created_at
         FOR UPDATE SKIP LOCKED
         LIMIT 1",
    )
    .fetch_optional(&mut *transaction)
    .await?;
    let Some((job_id, customer_id, created_by, include_history)) = job else {
        transaction.rollback().await?;
        return Ok(false);
    };
    sqlx::query(
        "UPDATE portal_export_jobs
         SET status = 'processing', started_at = now(), updated_at = now()
         WHERE id = $1",
    )
    .bind(job_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let result = build_export(state, job_id, customer_id, created_by, include_history).await;
    match result {
        Ok((storage_key, file_name, file_count, missing_count, total_size)) => {
            let expires_at = Utc::now() + Duration::days(EXPORT_RETENTION_DAYS);
            sqlx::query(
                "UPDATE portal_export_jobs
                 SET status = 'completed',
                     report_file_count = $2,
                     missing_count = $3,
                     total_size = $4,
                     result_storage_key = $5,
                     result_file_name = $6,
                     expires_at = $7,
                     finished_at = now(),
                     updated_at = now()
                 WHERE id = $1",
            )
            .bind(job_id)
            .bind(file_count)
            .bind(missing_count)
            .bind(total_size)
            .bind(storage_key)
            .bind(file_name)
            .bind(expires_at)
            .execute(&state.pool)
            .await?;
            audit(
                &state.pool,
                Some(created_by),
                Some(customer_id),
                "completed",
                "export",
                &job_id.to_string(),
                serde_json::json!({
                    "file_count": file_count,
                    "missing_count": missing_count,
                    "total_size": total_size
                }),
            )
            .await?;
        }
        Err(error) => {
            sqlx::query(
                "UPDATE portal_export_jobs
                 SET status = 'failed', last_error = $2, finished_at = now(), updated_at = now()
                 WHERE id = $1",
            )
            .bind(job_id)
            .bind(error.to_string())
            .execute(&state.pool)
            .await?;
            return Err(error);
        }
    }
    Ok(true)
}

async fn build_export(
    state: &PortalState,
    job_id: Uuid,
    customer_id: Uuid,
    created_by: Uuid,
    include_history: bool,
) -> Result<(String, String, i32, i32, i64), PortalError> {
    let history_allowed = sqlx::query_scalar::<_, bool>(
        "SELECT status = 'active' AND ($2 = FALSE OR can_view_report_history)
         FROM portal_users
         WHERE id = $1 AND customer_id = $3",
    )
    .bind(created_by)
    .bind(include_history)
    .bind(customer_id)
    .fetch_optional(&state.pool)
    .await?
    .unwrap_or(false);
    if !history_allowed {
        return Err(PortalError::Forbidden);
    }
    let rows = fetch_manifest_rows(state, job_id, include_history).await?;
    let mut unique_files = HashMap::<Uuid, (PathBuf, String, i64)>::new();
    let mut missing_count = 0_i32;
    for row in &rows {
        match (
            row.report_version_id,
            row.storage_key.as_deref(),
            row.file_name.as_deref(),
        ) {
            // 与单份下载同口径：只有 customer_copy_status = 'available' 的副本才允许进包，
            // 防止"生成失败/处理中"版本残留的旧附件被批量导出带出。
            (Some(report_id), Some(storage_key), Some(file_name)) if row.copy_available => {
                let path = resolve_storage_key(&state.storage_root, storage_key)?;
                if path.is_file() {
                    let size = std::fs::metadata(&path)
                        .map_err(|error| PortalError::Internal(error.to_string()))?
                        .len() as i64;
                    unique_files
                        .entry(report_id)
                        .or_insert((path, safe_zip_name(file_name), size));
                } else {
                    missing_count += 1;
                }
            }
            _ => missing_count += 1,
        }
    }
    if unique_files.len() as i64 > MAX_EXPORT_FILES {
        return Err(PortalError::Validation(format!(
            "单次导出最多 {MAX_EXPORT_FILES} 份，请拆分任务"
        )));
    }
    let total_size = unique_files.values().map(|(_, _, size)| *size).sum::<i64>();
    if total_size > MAX_EXPORT_BYTES {
        return Err(PortalError::Validation(
            "单次导出不得超过 2GB，请拆分任务".to_string(),
        ));
    }
    let storage_key = format!("wms-exports/{customer_id}/{job_id}.zip");
    let target = resolve_storage_key(&state.storage_root, &storage_key)?;
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| PortalError::Internal(error.to_string()))?;
    }
    let file_name = format!("药检单导出-{job_id}.zip");
    // 先写 .part 再原子改名，任务中途失败不会留下半截 ZIP。
    let part_path = target.with_extension("zip.part");
    let part_for_task = part_path.clone();
    let write_result =
        tokio::task::spawn_blocking(move || write_zip(&part_for_task, &rows, unique_files))
            .await
            .map_err(|error| PortalError::Internal(error.to_string()))
            .and_then(|inner| inner);
    if let Err(error) = write_result {
        remove_partial_export(&part_path).await;
        return Err(error);
    }
    if let Err(error) = tokio::fs::rename(&part_path, &target).await {
        remove_partial_export(&part_path).await;
        return Err(PortalError::Internal(error.to_string()));
    }
    Ok((
        storage_key,
        file_name,
        unique_files_len(&target)?,
        missing_count,
        total_size,
    ))
}

async fn remove_partial_export(path: &std::path::Path) {
    if let Err(error) = tokio::fs::remove_file(path).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(path = %path.display(), %error, "failed to remove partial export");
        }
    }
}

fn unique_files_len(path: &std::path::Path) -> Result<i32, PortalError> {
    let file = File::open(path).map_err(|error| PortalError::Internal(error.to_string()))?;
    let archive =
        zip::ZipArchive::new(file).map_err(|error| PortalError::Internal(error.to_string()))?;
    Ok(archive
        .file_names()
        .filter(|name| name.starts_with("reports/"))
        .count() as i32)
}

async fn fetch_manifest_rows(
    state: &PortalState,
    job_id: Uuid,
    include_history: bool,
) -> Result<Vec<ManifestRow>, PortalError> {
    let rows = sqlx::query(
        "SELECT o.order_no, a.address_name, l.product_code, l.product_name, l.batch_no,
                r.id AS report_version_id, r.version_number,
                r.customer_copy_file_name AS file_name,
                r.customer_copy_storage_key AS storage_key,
                COALESCE(r.customer_copy_status = 'available', FALSE) AS copy_available,
                CASE
                    WHEN r.id IS NULL THEN '资料暂缺'
                    WHEN r.customer_copy_status = 'available' THEN '可下载'
                    WHEN r.customer_copy_status = 'processing' THEN '处理中'
                    WHEN r.customer_copy_status = 'failed' THEN '生成失败'
                    ELSE '资料暂缺'
                END AS status
         FROM portal_export_job_orders jo
         JOIN portal_orders o ON o.id = jo.order_id
         JOIN portal_customer_addresses a ON a.id = o.delivery_address_id
         JOIN portal_order_lines l ON l.order_id = o.id
         LEFT JOIN portal_report_versions r
           ON r.product_id = l.product_id
          AND r.batch_no = l.batch_no
          AND ($2 OR r.is_current)
         WHERE jo.export_job_id = $1
         ORDER BY o.order_no, l.product_code, l.batch_no, r.version_number DESC",
    )
    .bind(job_id)
    .bind(include_history)
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let product_code: String = row.try_get("product_code")?;
            let product_name: String = row.try_get("product_name")?;
            let batch_no: String = row.try_get("batch_no")?;
            let version_number: Option<i32> = row.try_get("version_number")?;
            let file_name = match (
                row.try_get::<Option<String>, _>("file_name")?,
                version_number,
            ) {
                (Some(_), Some(version_number)) => Some(report_download_file_name(
                    &product_name,
                    &product_code,
                    &batch_no,
                    version_number,
                )),
                _ => None,
            };
            Ok(ManifestRow {
                order_no: row.try_get("order_no")?,
                address_name: row.try_get("address_name")?,
                product_code,
                product_name,
                batch_no,
                report_version_id: row.try_get("report_version_id")?,
                version_number,
                file_name,
                storage_key: row.try_get("storage_key")?,
                status: row.try_get("status")?,
                copy_available: row.try_get("copy_available")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(PortalError::from)
}

fn write_zip(
    target: &std::path::Path,
    rows: &[ManifestRow],
    unique_files: HashMap<Uuid, (PathBuf, String, i64)>,
) -> Result<(), PortalError> {
    let file = File::create(target).map_err(|error| PortalError::Internal(error.to_string()))?;
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut files = unique_files.into_iter().collect::<Vec<_>>();
    files.sort_by(|left, right| {
        left.1
             .1
            .cmp(&right.1 .1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let mut used_file_names = HashSet::new();
    for (_, (path, file_name, _)) in files {
        let file_name = unique_zip_file_name(&file_name, &mut used_file_names);
        zip.start_file(format!("reports/{file_name}"), options)
            .map_err(|error| PortalError::Internal(error.to_string()))?;
        let mut source =
            File::open(path).map_err(|error| PortalError::Internal(error.to_string()))?;
        std::io::copy(&mut source, &mut zip)
            .map_err(|error| PortalError::Internal(error.to_string()))?;
    }
    zip.start_file("药检单清单.csv", options)
        .map_err(|error| PortalError::Internal(error.to_string()))?;
    zip.write_all(b"\xEF\xBB\xBF")
        .map_err(|error| PortalError::Internal(error.to_string()))?;
    zip.write_all("订单号,地址,商品编码,商品名称,批号,报告版本,文件名,状态\n".as_bytes())
        .map_err(|error| PortalError::Internal(error.to_string()))?;
    for row in rows {
        let values = [
            row.order_no.clone(),
            row.address_name.clone(),
            row.product_code.clone(),
            row.product_name.clone(),
            row.batch_no.clone(),
            row.version_number
                .map(|version| version.to_string())
                .unwrap_or_default(),
            row.file_name.clone().unwrap_or_default(),
            row.status.clone(),
        ];
        let line = values
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(",");
        zip.write_all(format!("{line}\n").as_bytes())
            .map_err(|error| PortalError::Internal(error.to_string()))?;
    }
    zip.finish()
        .map_err(|error| PortalError::Internal(error.to_string()))?;
    Ok(())
}

fn safe_zip_name(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '/' | '\\' | '\0' | '\r' | '\n' => '_',
            other => other,
        })
        .collect()
}

fn unique_zip_file_name(file_name: &str, used: &mut HashSet<String>) -> String {
    if used.insert(file_name.to_string()) {
        return file_name.to_string();
    }
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{extension}")),
        None => (file_name, String::new()),
    };
    let mut sequence = 2_u64;
    loop {
        let candidate = format!("{stem}_{sequence}{extension}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        sequence += 1;
    }
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::unique_zip_file_name;
    use std::collections::HashSet;

    #[test]
    fn duplicate_zip_file_names_receive_a_stable_suffix() {
        let mut used = HashSet::new();
        assert_eq!(
            unique_zip_file_name("阿莫西林_P-001_B-01_药检单_V1.pdf", &mut used),
            "阿莫西林_P-001_B-01_药检单_V1.pdf"
        );
        assert_eq!(
            unique_zip_file_name("阿莫西林_P-001_B-01_药检单_V1.pdf", &mut used),
            "阿莫西林_P-001_B-01_药检单_V1_2.pdf"
        );
    }
}
